//! Shared parser for the Pi Coding Agent (<https://pi.dev/>) local session
//! files, and for its open-source fork OMP (<https://omp.sh/>), which persists
//! sessions using the exact same tree-structured JSONL format under
//! `<dir>/agent/sessions/`.
//!
//! File layout: `<dir>/agent/sessions/--<cwd>--/<timestamp>_<uuid>.jsonl`.
//! Format reference:
//! <https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/session-format.md>
use crate::db::{CostStats, TokenStats, UsageEntry};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub(crate) const SOURCE_KIND: &str = "pi-session";

/// Recursively collects every `*.jsonl` session file under
/// `<dir>/agent/sessions/`.
pub(crate) fn find_session_files(dir: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                visit(&path, files);
            } else if file_type.is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("jsonl")
            {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    let sessions_dir = dir.join("agent").join("sessions");
    if sessions_dir.is_dir() {
        visit(&sessions_dir, &mut files);
    }
    files.sort();
    files
}

fn value_as_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn parse_token_stats(usage: &Value) -> Option<TokenStats> {
    let input = value_as_u64(usage.get("input")).unwrap_or(0);
    let output = value_as_u64(usage.get("output")).unwrap_or(0);
    let cache_read = value_as_u64(usage.get("cacheRead"));
    let cache_write = value_as_u64(usage.get("cacheWrite"));
    let reasoning =
        value_as_u64(usage.get("reasoningTokens")).or_else(|| value_as_u64(usage.get("reasoning")));
    let total = value_as_u64(usage.get("totalTokens")).unwrap_or_else(|| {
        input
            .saturating_add(output)
            .saturating_add(cache_read.unwrap_or(0))
            .saturating_add(cache_write.unwrap_or(0))
    });

    if input == 0 && output == 0 && cache_read.unwrap_or(0) == 0 && total == 0 {
        return None;
    }

    Some(TokenStats {
        input,
        output,
        cache_read: cache_read.filter(|value| *value > 0),
        cache_write: cache_write.filter(|value| *value > 0),
        cache_write_5m: None,
        cache_write_1h: None,
        reasoning: reasoning.filter(|value| *value > 0),
        total,
    })
}

pub(crate) fn parse_reported_cost(usage: &Value) -> Option<f64> {
    usage
        .get("cost")
        .and_then(|cost| cost.get("total"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
}

struct SessionHeaderInfo {
    session_id: String,
    session_name: Option<String>,
    cwd: Option<String>,
    parent_session_id: Option<String>,
}

fn session_id_from_file(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(10).flatten() {
        let entry = serde_json::from_str::<Value>(&line).ok()?;
        if entry.get("type").and_then(Value::as_str) == Some("session") {
            return entry
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string);
        }
    }
    None
}

fn session_id_from_path(value: &str, source_kind: &str) -> Option<String> {
    let path = Path::new(value);
    if is_omp_session(source_kind) {
        return session_id_from_file(path);
    }
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn is_omp_session(source_kind: &str) -> bool {
    source_kind == crate::omp::SOURCE_KIND
}

fn is_advisor_transcript(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name == "__advisor.jsonl"
                || (name.starts_with("__advisor.") && name.ends_with(".jsonl"))
        })
}

fn advisor_nickname(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if name == "__advisor.jsonl" {
        return Some("advisor".to_string());
    }
    name.strip_prefix("__advisor.")
        .and_then(|name| name.strip_suffix(".jsonl"))
        .filter(|name| !name.is_empty())
        .map(|name| format!("advisor:{name}"))
}

fn artifact_parent_session_id(path: &Path, source_kind: &str) -> Option<String> {
    let artifact_dir = path.parent()?;
    if is_omp_session(source_kind) {
        // OMP stores agent transcripts in `<parent-stem>/<agent>.jsonl` and
        // advisor transcripts in `<parent-stem>/<agent>/__advisor*.jsonl`.
        // In both shapes, `<artifact-dir>.jsonl` is the owning session file.
        return session_id_from_file(&artifact_dir.with_extension("jsonl"));
    }
    is_advisor_transcript(path).then(|| {
        artifact_dir
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    })?
}

fn canonical_omp_provider(provider: &str) -> &str {
    match provider {
        // OMP's ChatGPT OAuth transport bills at OpenAI's public API rates.
        "openai-codex" => "openai",
        "google-gemini-cli" | "google-antigravity" => "google",
        other => other,
    }
}

fn omp_model_id(provider: Option<&str>, model: Option<&str>) -> Option<String> {
    let model = model?.trim();
    if model.is_empty() {
        return None;
    }
    if let Some((model_provider, model_id)) = model.split_once('/') {
        return Some(format!(
            "{}/{}",
            canonical_omp_provider(model_provider),
            model_id
        ));
    }
    let provider = provider?.trim();
    if provider.is_empty() {
        return Some(model.to_string());
    }
    Some(format!("{}/{}", canonical_omp_provider(provider), model))
}

fn model_id(source_kind: &str, provider: Option<&str>, model: Option<&str>) -> Option<String> {
    if is_omp_session(source_kind) {
        omp_model_id(provider, model)
    } else {
        model.map(str::to_string)
    }
}

fn trimmed_session_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(100).collect())
}

fn execution_role(base_role: Option<&str>, purpose: Option<&str>) -> Option<String> {
    let purpose = purpose.map(str::trim).filter(|value| !value.is_empty());
    match (base_role, purpose) {
        (Some(base), Some(purpose)) => Some(format!("{base}:{purpose}")),
        (Some(base), None) => Some(base.to_string()),
        (None, Some(purpose)) => Some(purpose.to_string()),
        (None, None) => None,
    }
}

struct UsageEntryContext<'a> {
    header: &'a SessionHeaderInfo,
    transcript_path: &'a str,
    session_name: &'a Option<String>,
    source_kind: &'a str,
    agent_nickname: &'a Option<String>,
    agent_role: &'a Option<String>,
}

fn append_usage_entry(
    entries: &mut Vec<UsageEntry>,
    turn_no: &mut u32,
    timestamp: String,
    model: Option<String>,
    usage: &Value,
    context: UsageEntryContext<'_>,
) {
    let Some(tokens) = parse_token_stats(usage) else {
        return;
    };
    let reported_cost_usd = parse_reported_cost(usage);
    entries.push(UsageEntry {
        timestamp,
        session_id: context.header.session_id.clone(),
        session_name: context.session_name.clone(),
        transcript_path: Some(context.transcript_path.to_string()),
        cwd: context.header.cwd.clone(),
        version: None,
        turn_no: *turn_no,
        model: model.clone(),
        model_id: model,
        tokens: Some(tokens.clone()),
        delta_tokens: Some(tokens),
        context: None,
        cost: reported_cost_usd.map(|reported_cost_usd| CostStats {
            total_api_duration_ms: None,
            total_duration_ms: None,
            total_premium_requests: None,
            reported_cost_usd: Some(reported_cost_usd),
        }),
        source_kind: Some(context.source_kind.to_string()),
        source_dir_key: None,
        parent_session_id: context.header.parent_session_id.clone(),
        agent_nickname: context.agent_nickname.clone(),
        agent_role: context.agent_role.clone(),
        reasoning_effort: None,
    });
    *turn_no += 1;
}

fn read_session_header(path: &Path, source_kind: &str) -> SessionHeaderInfo {
    let fallback_id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let Ok(file) = File::open(path) else {
        return SessionHeaderInfo {
            session_id: fallback_id,
            session_name: None,
            cwd: None,
            parent_session_id: artifact_parent_session_id(path, source_kind),
        };
    };
    let reader = BufReader::new(file);
    let mut title = None;
    // The `session` header entry is usually the first line, but OMP (and
    // potentially future Pi versions) may prepend a `title` entry before it.
    for line in reader.lines().take(10) {
        let Ok(line) = line else {
            continue;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(header) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if header.get("type").and_then(Value::as_str) == Some("title") {
            title = trimmed_session_name(header.get("title").and_then(Value::as_str));
            continue;
        }
        if header.get("type").and_then(Value::as_str) == Some("session") {
            let session_id = header
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or(fallback_id);
            let cwd = header
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_string);
            let session_name =
                title.or_else(|| trimmed_session_name(header.get("title").and_then(Value::as_str)));
            let parent_session_id = header
                .get("parentSession")
                .and_then(Value::as_str)
                .and_then(|value| session_id_from_path(value, source_kind))
                .or_else(|| artifact_parent_session_id(path, source_kind));
            return SessionHeaderInfo {
                session_id,
                session_name,
                cwd,
                parent_session_id,
            };
        }
    }

    SessionHeaderInfo {
        session_id: fallback_id,
        session_name: title,
        cwd: None,
        parent_session_id: artifact_parent_session_id(path, source_kind),
    }
}

/// Parses one Pi/OMP session JSONL file into per-turn [`UsageEntry`] rows.
/// OMP v18 additionally persists `model_usage` calls (auto-thinking,
/// preflight, and other non-conversation work), which are collected with their
/// purpose preserved in `agent_role`.
pub(crate) fn parse_session_usage_file(
    path: &Path,
    source_kind: &str,
) -> Result<Vec<UsageEntry>, String> {
    let file =
        File::open(path).map_err(|error| format!("無法開啟 session 檔案 {:?}: {error}", path))?;
    let reader = BufReader::new(file);
    let header = read_session_header(path, source_kind);
    let transcript_path = path.to_string_lossy().into_owned();
    let mut entries = Vec::new();
    let mut turn_no = 1u32;
    let mut session_name = header.session_name.clone();
    let is_advisor = is_advisor_transcript(path);
    let mut agent_nickname = if is_advisor {
        advisor_nickname(path)
    } else {
        (is_omp_session(source_kind) && header.parent_session_id.is_some())
            .then(|| trimmed_session_name(path.file_stem().and_then(|name| name.to_str())))
            .flatten()
    };
    let mut agent_role = if is_advisor {
        Some("advisor".to_string())
    } else {
        agent_nickname.as_ref().map(|_| "subagent".to_string())
    };

    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(entry_value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let entry_type = entry_value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("");

        if entry_type == "session_info" {
            if let Some(name) = entry_value.get("name").and_then(Value::as_str) {
                session_name = trimmed_session_name(Some(name));
            }
            continue;
        }

        if is_omp_session(source_kind) && entry_type == "session_init" {
            session_name = trimmed_session_name(entry_value.get("task").and_then(Value::as_str));
            agent_nickname = entry_value
                .get("agent")
                .and_then(Value::as_str)
                .and_then(|value| trimmed_session_name(Some(value)));
            if agent_nickname.is_some() {
                agent_role = Some("subagent".to_string());
            }
            continue;
        }

        let timestamp = entry_value
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if is_omp_session(source_kind) && entry_type == "model_usage" {
            let model = model_id(
                source_kind,
                entry_value.get("provider").and_then(Value::as_str),
                entry_value.get("model").and_then(Value::as_str),
            );
            let usage_role = execution_role(
                agent_role.as_deref(),
                entry_value.get("purpose").and_then(Value::as_str),
            );
            append_usage_entry(
                &mut entries,
                &mut turn_no,
                timestamp,
                model,
                entry_value.get("usage").unwrap_or(&Value::Null),
                UsageEntryContext {
                    header: &header,
                    transcript_path: &transcript_path,
                    session_name: &session_name,
                    source_kind,
                    agent_nickname: &agent_nickname,
                    agent_role: &usage_role,
                },
            );
            continue;
        }

        if entry_type != "message" {
            continue;
        }
        let Some(message) = entry_value.get("message") else {
            continue;
        };
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let model = model_id(
            source_kind,
            message.get("provider").and_then(Value::as_str),
            message.get("model").and_then(Value::as_str),
        );
        append_usage_entry(
            &mut entries,
            &mut turn_no,
            timestamp,
            model,
            message.get("usage").unwrap_or(&Value::Null),
            UsageEntryContext {
                header: &header,
                transcript_path: &transcript_path,
                session_name: &session_name,
                source_kind,
                agent_nickname: &agent_nickname,
                agent_role: &agent_role,
            },
        );
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_jsonl_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "token-usage-insights-pi-test-{}-{}",
            label,
            std::process::id()
        ))
    }

    #[test]
    fn parse_session_usage_file_reads_cwd_when_title_precedes_session_header() {
        // OMP prepends a `{"type":"title",...}` line before the session
        // header, unlike Pi which starts directly with `{"type":"session"}`.
        let root = temp_jsonl_path("omp-title-prefix");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"title","v":1,"title":"","updatedAt":"2024-12-03T14:00:00.000Z"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"omp-sess-1","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/omp-project"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"model_change","id":"m0","parentId":null,"timestamp":"2024-12-03T14:00:00.500Z","model":"ollama/glm-5.3-flash:cloud"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m1","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Hi!"}}],"model":"ollama/glm-5.3-flash:cloud","usage":{{"input":10,"output":5,"totalTokens":15,"cost":{{"total":0}}}}}}}}"#
        )
        .unwrap();

        let entries = parse_session_usage_file(&path, SOURCE_KIND).unwrap();

        fs::remove_dir_all(&root).ok();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "omp-sess-1");
        assert_eq!(entries[0].cwd.as_deref(), Some("/tmp/omp-project"));
    }

    #[test]
    fn find_session_files_recurses_under_agent_sessions() {
        let root = temp_jsonl_path("find-session-files");
        let session_dir = root.join("agent").join("sessions").join("--tmp--project");
        fs::create_dir_all(&session_dir).unwrap();
        let session_file = session_dir.join("2024-12-03T14-00-00_abc.jsonl");
        fs::write(&session_file, "").unwrap();
        // Non-jsonl files must be ignored.
        fs::write(session_dir.join("notes.txt"), "ignore me").unwrap();

        let files = find_session_files(&root);

        fs::remove_dir_all(&root).ok();
        assert_eq!(files, vec![session_file]);
    }

    #[test]
    fn parse_session_usage_file_extracts_turns_with_cost() {
        let root = temp_jsonl_path("parse-usage");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"sess-1","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m1","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","message":{{"role":"user","content":"Hello"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m2","parentId":"m1","timestamp":"2024-12-03T14:00:02.000Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Hi!"}}],"provider":"anthropic","model":"claude-sonnet-4-5","usage":{{"input":100,"output":50,"cacheRead":10,"cacheWrite":0,"reasoning":15,"totalTokens":150,"cost":{{"input":0.001,"output":0.002,"cacheRead":0.0001,"cacheWrite":0,"total":0.0031}}}},"stopReason":"stop"}}}}"#
        )
        .unwrap();

        let entries = parse_session_usage_file(&path, SOURCE_KIND).unwrap();

        fs::remove_dir_all(&root).ok();

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.session_id, "sess-1");
        assert_eq!(entry.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(entry.turn_no, 1);
        assert_eq!(entry.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(entry.source_kind.as_deref(), Some(SOURCE_KIND));
        let tokens = entry.tokens.as_ref().unwrap();
        assert_eq!(tokens.input, 100);
        assert_eq!(tokens.output, 50);
        assert_eq!(tokens.cache_read, Some(10));
        assert_eq!(tokens.reasoning, Some(15));
        assert_eq!(tokens.total, 150);
        assert_eq!(
            entry.cost.as_ref().and_then(|cost| cost.reported_cost_usd),
            Some(0.0031)
        );
    }

    #[test]
    fn parse_session_usage_file_skips_non_usage_messages() {
        let root = temp_jsonl_path("parse-skip");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"sess-2","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m1","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","message":{{"role":"user","content":"Hello"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m2","parentId":"m1","timestamp":"2024-12-03T14:00:02.000Z","message":{{"role":"toolResult","toolCallId":"call_1","toolName":"bash","content":[{{"type":"text","text":"ok"}}],"isError":false}}}}"#
        )
        .unwrap();

        let entries = parse_session_usage_file(&path, SOURCE_KIND).unwrap();

        fs::remove_dir_all(&root).ok();
        assert!(entries.is_empty());
    }
}
