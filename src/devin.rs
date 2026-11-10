use crate::db::{ContextStats, CostStats, TokenStats, UsageEntry};
use crate::timeline::TimelineItem;
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub(crate) const SOURCE_KIND: &str = "devin-cli";

pub(crate) struct DevinSession {
    pub session_id: String,
    pub entries: Vec<UsageEntry>,
    pub timeline: Vec<TimelineItem>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug)]
struct SessionRecord {
    session_id: String,
    cwd: Option<String>,
    model: Option<String>,
    title: Option<String>,
}

struct MessageNode {
    node_id: i64,
    created_at: Option<i64>,
    chat_message: String,
}

struct ParsedUsage {
    tokens: TokenStats,
    context: Option<ContextStats>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    duration_ms: Option<u64>,
}

fn database_error(error: rusqlite::Error) -> String {
    format!("Devin database read failed: {error}")
}

fn open_read_only(database: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("Devin database open failed: {error}"))
}

fn read_session_records(
    transaction: &Transaction<'_>,
    session_id: Option<&str>,
    should_cancel: Option<&(dyn Fn() -> bool + Sync)>,
) -> Result<Vec<SessionRecord>, String> {
    let sql = if session_id.is_some() {
        "SELECT id, working_directory, model, title FROM sessions WHERE id = ?1"
    } else {
        "SELECT id, working_directory, model, title FROM sessions ORDER BY created_at, id"
    };
    let mut statement = transaction.prepare(sql).map_err(database_error)?;
    let mut records = Vec::new();
    if let Some(session_id) = session_id {
        if let Some(record) = statement
            .query_row(params![session_id], |row| {
                Ok(SessionRecord {
                    session_id: row.get(0)?,
                    cwd: row.get(1)?,
                    model: row.get(2)?,
                    title: row.get(3)?,
                })
            })
            .optional()
            .map_err(database_error)?
        {
            records.push(record);
        }
    } else {
        let rows = statement
            .query_map([], |row| {
                Ok(SessionRecord {
                    session_id: row.get(0)?,
                    cwd: row.get(1)?,
                    model: row.get(2)?,
                    title: row.get(3)?,
                })
            })
            .map_err(database_error)?;
        for row in rows {
            if should_cancel.is_some_and(|should_cancel| should_cancel()) {
                return Err("Devin sync cancelled".to_string());
            }
            records.push(row.map_err(database_error)?);
        }
    }
    Ok(records)
}

fn read_message_nodes(
    transaction: &Transaction<'_>,
    session_id: &str,
    should_cancel: Option<&(dyn Fn() -> bool + Sync)>,
) -> Result<Vec<MessageNode>, String> {
    let mut statement = transaction
        .prepare(
            "SELECT node_id, created_at, chat_message \
             FROM message_nodes WHERE session_id = ?1 ORDER BY created_at, row_id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![session_id], |row| {
            Ok(MessageNode {
                node_id: row.get(0)?,
                created_at: row.get(1)?,
                chat_message: row.get(2)?,
            })
        })
        .map_err(database_error)?;
    let mut nodes = Vec::new();
    for row in rows {
        if should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return Err("Devin sync cancelled".to_string());
        }
        nodes.push(row.map_err(database_error)?);
    }
    Ok(nodes)
}

fn nonempty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn split_model_effort(model: &str) -> (String, Option<String>) {
    let model = model.trim();
    let lowercase = model.to_ascii_lowercase();
    let speed = ["-lightning", "-priority", "-fast"]
        .into_iter()
        .find(|suffix| lowercase.ends_with(suffix));
    let without_speed = speed.map_or(model, |suffix| &model[..model.len() - suffix.len()]);
    let lowercase_without_speed = without_speed.to_ascii_lowercase();
    let effort = [
        "-extra-high",
        "-extra_high",
        "-minimal",
        "-medium",
        "-xhigh",
        "-none",
        "-high",
        "-low",
        "-max",
    ]
    .into_iter()
    .find(|suffix| lowercase_without_speed.ends_with(suffix));
    let base = effort.map_or(without_speed, |suffix| {
        &without_speed[..without_speed.len() - suffix.len()]
    });
    let mut normalized_model = base.to_string();
    if let Some(speed) = speed {
        normalized_model.push_str(speed);
    }
    (
        normalized_model,
        effort.map(|suffix| suffix[1..].replace('_', "-")),
    )
}

fn normalize_model(model: &str) -> String {
    match model.to_ascii_lowercase().as_str() {
        "swe-1-7" => "swe-1.7".to_string(),
        "swe-1-6" => "swe-1.6".to_string(),
        "swe-2" => "swe-2".to_string(),
        "swe-2-lightning" => "swe-2-lightning".to_string(),
        "glm-5-3" => "glm-5.3".to_string(),
        "gemini-3-8-flash" => "gemini-3.8-flash".to_string(),
        "muse-wonder-1-0" => "muse-wonder-1.0".to_string(),
        "claude-opus-5-5-fast" => "claude-opus-5.5-fast".to_string(),
        _ => model.to_string(),
    }
}

fn model_and_effort(
    model: Option<String>,
    explicit_effort: Option<String>,
) -> (Option<String>, Option<String>) {
    let Some(model) = model else {
        return (None, explicit_effort);
    };
    let (model, suffix_effort) = split_model_effort(&model);
    (
        Some(normalize_model(&model)),
        explicit_effort.or(suffix_effort),
    )
}

fn token_value(value: Option<&Value>) -> Option<u64> {
    match value {
        None | Some(Value::Null) => Some(0),
        Some(value) => value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok())),
    }
}

fn parse_usage(message: &Value) -> Result<Option<ParsedUsage>, String> {
    let Some(metadata_value) = message.get("metadata").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(metadata) = metadata_value.as_object() else {
        return Err("Devin message metadata is invalid".to_string());
    };
    let Some(metrics_value) = metadata.get("metrics").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(metrics) = metrics_value.as_object() else {
        return Err("Devin message metrics are invalid".to_string());
    };
    if [
        "input_tokens",
        "output_tokens",
        "cache_read_tokens",
        "cache_creation_tokens",
        "cache_write_tokens",
    ]
    .iter()
    .all(|key| token_value(metrics.get(*key)) == Some(0))
    {
        return Ok(None);
    }
    let input = metrics
        .get("input_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Devin message input token count is invalid".to_string())?;
    let output = metrics
        .get("output_tokens")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Devin message output token count is invalid".to_string())?;
    let cache_read = token_value(metrics.get("cache_read_tokens"))
        .ok_or_else(|| "Devin message cache read count is invalid".to_string())?;
    let cache_write = token_value(
        metrics
            .get("cache_creation_tokens")
            .or_else(|| metrics.get("cache_write_tokens")),
    )
    .ok_or_else(|| "Devin message cache creation count is invalid".to_string())?;
    let total = input
        .saturating_add(output)
        .saturating_add(cache_read)
        .saturating_add(cache_write);
    if total == 0 {
        return Ok(None);
    }
    let tokens = TokenStats {
        input,
        output,
        cache_read: Some(cache_read),
        cache_write: Some(cache_write),
        cache_write_5m: None,
        cache_write_1h: None,
        reasoning: None,
        total,
    };
    let context = metrics
        .get("context_input_tokens")
        .and_then(Value::as_u64)
        .map(|current_context_tokens| ContextStats {
            current_context_tokens: Some(current_context_tokens),
            displayed_context_limit: None,
            current_context_used_percentage: None,
        });
    let model = nonempty_string(metadata.get("generation_model"));
    let explicit_effort = nonempty_string(metadata.get("reasoning_effort"))
        .or_else(|| nonempty_string(message.get("reasoning_effort")));
    let (model, reasoning_effort) = model_and_effort(model, explicit_effort);
    Ok(Some(ParsedUsage {
        tokens,
        context,
        model,
        reasoning_effort,
        duration_ms: metrics.get("total_time_ms").and_then(Value::as_u64),
    }))
}

fn node_turn_no(node_id: i64) -> Option<u32> {
    u32::try_from(node_id).ok()
}

fn timestamp(node_created_at: Option<i64>, message: &Value) -> String {
    if let Some(raw) = message
        .get("metadata")
        .and_then(|metadata| metadata.get("created_at"))
        .and_then(Value::as_str)
    {
        if let Ok(timestamp) = DateTime::parse_from_rfc3339(raw) {
            return timestamp
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true);
        }
    }
    node_created_at
        .and_then(|created_at| DateTime::<Utc>::from_timestamp(created_at, 0))
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Millis, true))
        .unwrap_or_default()
}

fn content_text(value: Option<&Value>) -> String {
    crate::grok::value_to_text(value)
}

fn tool_call_parts(call: &Value) -> Option<(String, Value, Option<String>)> {
    let function = call.get("function");
    let name = nonempty_string(function.and_then(|function| function.get("name")))
        .or_else(|| nonempty_string(call.get("name")))?;
    let arguments = function
        .and_then(|function| function.get("arguments"))
        .or_else(|| call.get("arguments"))
        .cloned()
        .map(|arguments| match arguments {
            Value::String(text) => serde_json::from_str(&text).unwrap_or(Value::String(text)),
            arguments => arguments,
        })
        .unwrap_or(Value::Null);
    let call_id = nonempty_string(call.get("id"));
    Some((name, arguments, call_id))
}

fn update_tool_result(
    timeline: &mut [TimelineItem],
    call_id: &str,
    timestamp: &str,
    output: &str,
) -> bool {
    for item in timeline.iter_mut().rev() {
        if let TimelineItem::ToolStep {
            timestamp: step_timestamp,
            stdout,
            status,
            tool_call_id: Some(existing_id),
            ..
        } = item
        {
            if existing_id == call_id {
                *step_timestamp = timestamp.to_string();
                *stdout = output.to_string();
                *status = "success".to_string();
                return true;
            }
        }
    }
    false
}

fn read_devin_session(
    transaction: &Transaction<'_>,
    database: &Path,
    record: SessionRecord,
    should_cancel: Option<&(dyn Fn() -> bool + Sync)>,
) -> Result<DevinSession, String> {
    let session_model = record
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string);
    let (selected_model, _) = session_model
        .as_deref()
        .map(|model| model_and_effort(Some(model.to_string()), None))
        .unwrap_or((None, None));
    let nodes = read_message_nodes(transaction, &record.session_id, should_cancel)?;
    let mut entries: Vec<UsageEntry> = Vec::new();
    let mut timeline = Vec::new();
    let mut seen_message_ids = HashSet::new();
    let mut seen_tool_ids = HashSet::new();
    let mut usage_by_call_key: HashMap<String, usize> = HashMap::new();
    let mut reply_by_message_id = HashMap::new();
    let mut entry_reply_indices = Vec::new();
    let session_name = record
        .title
        .filter(|title| !title.trim().is_empty())
        .or_else(|| Some(record.session_id.clone()));
    let transcript_path = database.to_string_lossy().into_owned();

    for node in nodes {
        if should_cancel.is_some_and(|should_cancel| should_cancel()) {
            return Err("Devin sync cancelled".to_string());
        }
        let Some(turn_no) = node_turn_no(node.node_id) else {
            continue;
        };
        let message: Value = serde_json::from_str(&node.chat_message)
            .map_err(|_| "Devin session contains invalid message JSON".to_string())?;
        if !message.is_object() {
            return Err("Devin session contains an invalid message".to_string());
        }
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| "Devin session message has no role".to_string())?;
        let message_id = nonempty_string(message.get("message_id"));
        let request_id = nonempty_string(
            message
                .get("metadata")
                .and_then(|metadata| metadata.get("request_id")),
        );
        let timestamp = timestamp(node.created_at, &message);

        if role == "assistant" {
            if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    let Some((tool_name, arguments, tool_call_id)) = tool_call_parts(call) else {
                        continue;
                    };
                    if tool_call_id
                        .as_ref()
                        .is_some_and(|id| !seen_tool_ids.insert(id.clone()))
                    {
                        continue;
                    }
                    timeline.push(TimelineItem::ToolStep {
                        timestamp: timestamp.clone(),
                        tool_name,
                        arguments,
                        env: None,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: String::new(),
                        tool_call_id,
                        status: "running".to_string(),
                    });
                }
            }
            if let Some(usage) = parse_usage(&message)? {
                let content = content_text(message.get("content"));
                let reasoning = message
                    .get("reasoning")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let mut call_keys = Vec::new();
                if let Some(message_id) = message_id.as_deref() {
                    call_keys.push(format!("message:{message_id}"));
                }
                if let Some(request_id) = request_id.as_deref() {
                    call_keys.push(format!("request:{request_id}"));
                }
                let existing_entry = call_keys
                    .iter()
                    .find_map(|key| usage_by_call_key.get(key).copied());
                let turn_no = existing_entry.map_or(turn_no, |index| entries[index].turn_no);
                let model = usage
                    .model
                    .clone()
                    .unwrap_or_else(|| "Unknown Model".to_string());
                let entry = UsageEntry {
                    timestamp: timestamp.clone(),
                    session_id: record.session_id.clone(),
                    session_name: session_name.clone(),
                    transcript_path: Some(transcript_path.clone()),
                    cwd: record.cwd.clone(),
                    version: None,
                    turn_no,
                    model: usage.model.clone(),
                    model_id: usage.model.clone(),
                    tokens: Some(usage.tokens.clone()),
                    delta_tokens: Some(usage.tokens.clone()),
                    context: usage.context.clone(),
                    cost: usage.duration_ms.map(|duration_ms| CostStats {
                        total_api_duration_ms: Some(duration_ms as f64),
                        total_duration_ms: Some(duration_ms as f64),
                        total_premium_requests: None,
                        reported_cost_usd: None,
                    }),
                    source_kind: Some(SOURCE_KIND.to_string()),
                    source_dir_key: None,
                    parent_session_id: None,
                    agent_nickname: None,
                    agent_role: None,
                    reasoning_effort: usage.reasoning_effort.clone(),
                    session_pricing: None,
                };
                let reply_index = if let Some(entry_index) = existing_entry {
                    entries[entry_index] = entry;
                    let reply_index = entry_reply_indices[entry_index];
                    if let Some(TimelineItem::AgentReply {
                        timestamp: reply_timestamp,
                        reply,
                        reasoning: reply_reasoning,
                        turn_no: reply_turn_no,
                        model: reply_model,
                        tokens,
                        duration_ms,
                        reasoning_effort,
                    }) = timeline.get_mut(reply_index)
                    {
                        *reply_timestamp = timestamp.clone();
                        *reply = content;
                        *reply_reasoning = reasoning;
                        *reply_turn_no = turn_no;
                        *reply_model = model;
                        *tokens = Some(usage.tokens.clone());
                        *duration_ms = usage.duration_ms;
                        *reasoning_effort = usage.reasoning_effort.clone();
                    }
                    reply_index
                } else if let Some(reply_index) = message_id
                    .as_ref()
                    .and_then(|message_id| reply_by_message_id.get(message_id).copied())
                {
                    if let Some(TimelineItem::AgentReply {
                        timestamp: reply_timestamp,
                        reply,
                        reasoning: reply_reasoning,
                        turn_no: reply_turn_no,
                        model: reply_model,
                        tokens,
                        duration_ms,
                        reasoning_effort,
                    }) = timeline.get_mut(reply_index)
                    {
                        *reply_timestamp = timestamp.clone();
                        *reply = content;
                        *reply_reasoning = reasoning;
                        *reply_turn_no = turn_no;
                        *reply_model = model;
                        *tokens = Some(usage.tokens.clone());
                        *duration_ms = usage.duration_ms;
                        *reasoning_effort = usage.reasoning_effort.clone();
                    }
                    entries.push(entry);
                    entry_reply_indices.push(reply_index);
                    reply_index
                } else {
                    timeline.push(TimelineItem::AgentReply {
                        timestamp: timestamp.clone(),
                        reply: content,
                        reasoning,
                        turn_no,
                        model,
                        tokens: Some(usage.tokens.clone()),
                        duration_ms: usage.duration_ms,
                        reasoning_effort: usage.reasoning_effort.clone(),
                    });
                    let reply_index = timeline.len() - 1;
                    entries.push(entry);
                    entry_reply_indices.push(reply_index);
                    reply_index
                };
                let entry_index = existing_entry.unwrap_or(entries.len() - 1);
                entry_reply_indices[entry_index] = reply_index;
                for key in call_keys {
                    usage_by_call_key.insert(key, entry_index);
                }
                if let Some(message_id) = message_id {
                    seen_message_ids.insert(message_id.clone());
                    reply_by_message_id.insert(message_id, reply_index);
                }
                continue;
            }
        }

        if message_id
            .as_ref()
            .is_some_and(|message_id| seen_message_ids.contains(message_id))
        {
            continue;
        }
        if let Some(message_id) = message_id.clone() {
            seen_message_ids.insert(message_id);
        }
        match role {
            "user" => {
                let prompt = content_text(message.get("content"));
                if !prompt.trim().is_empty() {
                    timeline.push(TimelineItem::UserPrompt {
                        timestamp,
                        prompt,
                        context: None,
                        turn_no,
                    });
                }
            }
            "assistant" => {
                let content = content_text(message.get("content"));
                let reasoning = message
                    .get("reasoning")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if !content.is_empty() {
                    timeline.push(TimelineItem::AgentReply {
                        timestamp,
                        reply: content,
                        reasoning,
                        turn_no,
                        model: "Unknown Model".to_string(),
                        tokens: None,
                        duration_ms: None,
                        reasoning_effort: None,
                    });
                    if let Some(message_id) = message_id {
                        reply_by_message_id.insert(message_id, timeline.len() - 1);
                    }
                }
            }
            "tool" => {
                let tool_call_id = nonempty_string(message.get("tool_call_id"));
                let output = content_text(message.get("content"));
                let updated = tool_call_id.as_deref().is_some_and(|call_id| {
                    update_tool_result(&mut timeline, call_id, &timestamp, &output)
                });
                if !updated {
                    timeline.push(TimelineItem::ToolStep {
                        timestamp,
                        tool_name: nonempty_string(message.get("name"))
                            .unwrap_or_else(|| "tool".to_string()),
                        arguments: Value::Null,
                        env: None,
                        exit_code: None,
                        stdout: output,
                        stderr: String::new(),
                        tool_call_id,
                        status: "success".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    entries.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.turn_no.cmp(&right.turn_no))
    });
    let mut metadata = HashMap::new();
    if let Some(cwd) = record.cwd {
        metadata.insert("cwd".to_string(), Value::String(cwd));
    }
    if let Some(selected_model) = selected_model {
        metadata.insert("selected_model".to_string(), Value::String(selected_model));
    }
    Ok(DevinSession {
        session_id: record.session_id,
        entries,
        timeline,
        metadata,
    })
}

pub(crate) fn read_sessions(
    database: &Path,
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Vec<DevinSession>, String> {
    if should_cancel() {
        return Err("Devin sync cancelled".to_string());
    }
    if !database
        .try_exists()
        .map_err(|error| format!("Devin database access check failed: {error}"))?
    {
        return Ok(Vec::new());
    }
    let mut connection = open_read_only(database)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(database_error)?;
    let records = read_session_records(&transaction, None, Some(should_cancel))?;
    let mut sessions = Vec::with_capacity(records.len());
    for record in records {
        if should_cancel() {
            return Err("Devin sync cancelled".to_string());
        }
        sessions.push(read_devin_session(
            &transaction,
            database,
            record,
            Some(should_cancel),
        )?);
    }
    transaction.commit().map_err(database_error)?;
    Ok(sessions)
}

pub(crate) fn read_session(database: &Path, session_id: &str) -> Result<DevinSession, String> {
    let mut connection = open_read_only(database)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(database_error)?;
    let record = read_session_records(&transaction, Some(session_id), None)?
        .into_iter()
        .next()
        .ok_or_else(|| "Devin session not found".to_string())?;
    let session = read_devin_session(&transaction, database, record, None)?;
    transaction.commit().map_err(database_error)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDatabase(PathBuf);

    impl TestDatabase {
        fn new() -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "token-usage-insights-devin-{}-{id}.sqlite",
                std::process::id()
            ));
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE sessions (
                        id TEXT PRIMARY KEY,
                        working_directory TEXT,
                        backend_type TEXT,
                        model TEXT,
                        agent_mode TEXT,
                        created_at INTEGER,
                        last_activity_at INTEGER,
                        title TEXT,
                        main_chain_id TEXT
                    );
                    CREATE TABLE message_nodes (
                        row_id INTEGER PRIMARY KEY,
                        session_id TEXT,
                        node_id INTEGER NOT NULL,
                        parent_node_id,
                        chat_message TEXT,
                        created_at INTEGER,
                        UNIQUE(session_id, node_id)
                    );",
                )
                .unwrap();
            drop(connection);
            Self(path)
        }

        fn insert_session(&self, id: &str, cwd: &str, model: Option<&str>, title: &str) {
            let connection = Connection::open(&self.0).unwrap();
            connection
                .execute(
                    "INSERT INTO sessions (id, working_directory, model, title, created_at) \
                     VALUES (?1, ?2, ?3, ?4, 1)",
                    params![id, cwd, model, title],
                )
                .unwrap();
        }

        fn insert_message(&self, session_id: &str, node_id: &str, created_at: i64, message: &str) {
            let connection = Connection::open(&self.0).unwrap();
            connection
                .execute(
                    "INSERT INTO message_nodes (session_id, node_id, chat_message, created_at) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![session_id, node_id, message, created_at],
                )
                .unwrap();
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn assistant_message(
        message_id: &str,
        request_id: &str,
        model: Option<&str>,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_write: Option<u64>,
    ) -> String {
        let mut metadata = serde_json::json!({
            "request_id": request_id,
            "metrics": {
                "input_tokens": input,
                "output_tokens": output,
                "cache_read_tokens": cache_read,
                "cache_creation_tokens": cache_write,
                "context_input_tokens": 78029,
                "total_time_ms": 29669
            }
        });
        if let Some(model) = model {
            metadata["generation_model"] = serde_json::json!(model);
        }
        serde_json::json!({
            "message_id": message_id,
            "role": "assistant",
            "content": "Synthetic reply",
            "tool_calls": [],
            "metadata": metadata
        })
        .to_string()
    }

    fn no_cancel() -> bool {
        false
    }

    #[test]
    fn parses_metrics_and_deduplicates_saved_calls() {
        let database = TestDatabase::new();
        database.insert_session("session-a", "/synthetic/project", None, "Synthetic session");
        database.insert_message(
            "session-a",
            "1",
            1_735_689_600,
            r#"{"message_id":"user-1","role":"user","content":"Synthetic prompt","metadata":{"created_at":"2025-01-01T00:00:00Z"}}"#,
        );
        let earlier = assistant_message(
            "message-1",
            "request-1",
            Some("swe-2-high"),
            10,
            2,
            0,
            Some(0),
        );
        let latest = assistant_message(
            "message-1",
            "request-1",
            Some("swe-2-high"),
            5721,
            2771,
            72308,
            None,
        );
        database.insert_message("session-a", "2", 1_735_689_601, &earlier);
        database.insert_message("session-a", "3", 1_735_689_602, &latest);

        let sessions = read_sessions(&database.0, &no_cancel).unwrap();
        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(session.entries.len(), 1);
        let entry = &session.entries[0];
        assert_eq!(entry.model.as_deref(), Some("swe-2"));
        assert_eq!(entry.reasoning_effort.as_deref(), Some("high"));
        assert_eq!(entry.turn_no, 2);
        let tokens = entry.tokens.as_ref().unwrap();
        assert_eq!(tokens.input, 5721);
        assert_eq!(tokens.output, 2771);
        assert_eq!(tokens.cache_read, Some(72308));
        assert_eq!(tokens.cache_write, Some(0));
        assert_eq!(tokens.total, 80800);
        assert_eq!(entry.delta_tokens.as_ref().unwrap().total, 80800);
        assert_eq!(
            entry.context.as_ref().unwrap().current_context_tokens,
            Some(78029)
        );
        assert_eq!(entry.source_kind.as_deref(), Some(SOURCE_KIND));
        assert_eq!(
            entry.transcript_path.as_deref(),
            Some(database.0.to_str().unwrap())
        );
        assert_eq!(
            session.metadata.get("cwd").and_then(Value::as_str),
            Some("/synthetic/project")
        );
        assert!(session.timeline.iter().any(|item| matches!(
            item,
            TimelineItem::UserPrompt { turn_no: 1, prompt, .. } if prompt == "Synthetic prompt"
        )));
        let replies: Vec<_> = session
            .timeline
            .iter()
            .filter_map(|item| match item {
                TimelineItem::AgentReply {
                    turn_no,
                    tokens: Some(tokens),
                    ..
                } => Some((*turn_no, tokens.total)),
                _ => None,
            })
            .collect();
        assert_eq!(replies, vec![(2, 80800)]);
    }

    #[test]
    fn normalizes_mixed_generation_models_and_effort_suffixes() {
        let database = TestDatabase::new();
        database.insert_session(
            "session-b",
            "/synthetic",
            Some("muse-wonder-1-0-medium"),
            "Mixed models",
        );
        database.insert_message(
            "session-b",
            "5",
            1,
            &assistant_message("a", "ra", Some("glm-5-3-max"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "6",
            2,
            &assistant_message("b", "rb", Some("gemini-3-8-flash-medium"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "7",
            3,
            &assistant_message("c", "rc", None, 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "8",
            4,
            &assistant_message("d", "rd", Some("muse-wonder-1-0-medium"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "9",
            5,
            &assistant_message("e", "re", Some("SWE-1-7-high"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "10",
            6,
            &assistant_message("f", "rf", Some("SWE-1-6-minimal"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "11",
            7,
            &assistant_message("g", "rg", Some("SWE-2-high-lightning"), 1, 2, 3, Some(4)),
        );
        database.insert_message(
            "session-b",
            "12",
            8,
            &assistant_message(
                "h",
                "rh",
                Some("claude-opus-5-5-medium-fast"),
                1,
                2,
                3,
                Some(4),
            ),
        );
        database.insert_message(
            "session-b",
            "13",
            9,
            &assistant_message("i", "ri", Some("gpt-6-sol-high-priority"), 1, 2, 3, Some(4)),
        );

        let session = read_session(&database.0, "session-b").unwrap();
        let models: Vec<_> = session
            .entries
            .iter()
            .map(|entry| (entry.model.as_deref(), entry.reasoning_effort.as_deref()))
            .collect();
        assert_eq!(
            models,
            vec![
                (Some("glm-5.3"), Some("max")),
                (Some("gemini-3.8-flash"), Some("medium")),
                (None, None),
                (Some("muse-wonder-1.0"), Some("medium")),
                (Some("swe-1.7"), Some("high")),
                (Some("swe-1.6"), Some("minimal")),
                (Some("swe-2-lightning"), Some("high")),
                (Some("claude-opus-5.5-fast"), Some("medium")),
                (Some("gpt-6-sol-priority"), Some("high")),
            ]
        );
        assert_eq!(
            session
                .metadata
                .get("selected_model")
                .and_then(Value::as_str),
            Some("muse-wonder-1.0")
        );
    }

    #[test]
    fn rejects_malformed_messages_and_metrics_before_session_replacement() {
        let database = TestDatabase::new();
        database.insert_session("session-c", "/synthetic", None, "Corrupt JSON");
        database.insert_message("session-c", "10", 1, "not json");
        assert!(read_session(&database.0, "session-c")
            .err()
            .unwrap()
            .contains("invalid message JSON"));

        let metrics_database = TestDatabase::new();
        metrics_database.insert_session("session-d", "/synthetic", None, "Bad metrics");
        metrics_database.insert_message("session-d", "11", 2, r#"{"message_id":"bad-metrics","role":"assistant","metadata":{"metrics":{"input_tokens":"bad","output_tokens":2}}}"#);
        assert!(read_session(&metrics_database.0, "session-d")
            .err()
            .unwrap()
            .contains("input token count is invalid"));
    }

    #[test]
    fn failed_generation_with_null_input_does_not_block_other_usage() {
        let database = TestDatabase::new();
        database.insert_session("failed-call", "/synthetic", None, "Failed generation");
        database.insert_message("failed-call", "1", 1_735_689_600,
            r#"{"message_id":"failed-message","role":"assistant","content":"","metadata":{"generation_model":"glm-5-2","metrics":{"ttft_ms":null,"total_time_ms":26521,"input_tokens":null,"output_tokens":0,"cache_read_tokens":null,"cache_creation_tokens":null}}}"#);
        database.insert_message(
            "failed-call",
            "2",
            1_735_689_601,
            &assistant_message(
                "valid-message",
                "valid-call",
                Some("swe-2-high"),
                10,
                2,
                0,
                None,
            ),
        );
        let sessions = read_sessions(&database.0, &no_cancel).unwrap();
        assert_eq!(sessions[0].entries.len(), 1);
        assert_eq!(sessions[0].entries[0].tokens.as_ref().unwrap().total, 12);
    }

    #[test]
    fn skips_empty_metrics_and_keeps_timeline_calls_aligned() {
        let database = TestDatabase::new();
        database.insert_session("session-e", "/synthetic", None, "Timeline");
        database.insert_message(
            "session-e",
            "10",
            1_735_689_600,
            r#"{"message_id":"prompt-1","role":"user","content":"Synthetic prompt"}"#,
        );
        database.insert_message(
            "session-e",
            "11",
            1_735_689_601,
            &assistant_message("empty", "empty-request", None, 0, 0, 0, Some(0)),
        );
        database.insert_message(
            "session-e",
            "9",
            1_735_689_601,
            &assistant_message(
                "tool-call-message",
                "tool-request",
                Some("gemini-3-8-flash-medium"),
                1,
                1,
                0,
                None,
            ),
        );
        database.insert_message("session-e", "14", 1_735_689_604, r#"{"message_id":"unfinished","role":"assistant","content":"Unfinished","metadata":{"metrics":null}}"#);
        database.insert_message(
            "session-e",
            "12",
            1_735_689_602,
            r#"{"message_id":"tool-call-message","role":"assistant","content":"","tool_calls":[{"id":"tool-1","type":"function","function":{"name":"lookup","arguments":"{\"q\":\"synthetic\"}"}}],"metadata":{"request_id":"tool-request","generation_model":"gemini-3-8-flash-medium","metrics":{"input_tokens":1,"output_tokens":2,"cache_read_tokens":0,"cache_creation_tokens":null,"total_time_ms":9}}}"#,
        );
        database.insert_message(
            "session-e",
            "13",
            1_735_689_603,
            r#"{"message_id":"tool-result","role":"tool","tool_call_id":"tool-1","name":"lookup","content":"Synthetic result"}"#,
        );

        let session = read_session(&database.0, "session-e").unwrap();
        assert_eq!(session.entries.len(), 1);
        assert_eq!(session.entries[0].turn_no, 9);
        assert!(session.timeline.iter().any(|item| matches!(
            item,
            TimelineItem::UserPrompt { turn_no: 10, prompt, .. } if prompt == "Synthetic prompt"
        )));
        assert!(session.timeline.iter().any(|item| matches!(
            item,
            TimelineItem::ToolStep { tool_name, arguments, stdout, status, .. }
                if tool_name == "lookup"
                    && arguments.get("q").and_then(Value::as_str) == Some("synthetic")
                    && stdout == "Synthetic result"
                    && status == "success"
        )));
        assert!(session.timeline.iter().any(|item| matches!(
            item,
            TimelineItem::AgentReply { turn_no: 9, tokens: Some(_), reasoning_effort: Some(effort), .. }
                if effort == "medium"
        )));
    }

    #[test]
    fn missing_database_is_not_created_and_single_session_read_errors() {
        let path = std::env::temp_dir().join(format!(
            "token-usage-insights-devin-missing-{}.sqlite",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        assert!(read_sessions(&path, &no_cancel).unwrap().is_empty());
        assert!(!path.exists());
        assert!(read_session(&path, "missing").is_err());
        assert!(!path.exists());
    }
}
