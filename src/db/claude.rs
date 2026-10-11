use super::*;

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation: ClaudeCacheCreation,
    #[serde(default)]
    speed: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ClaudeCacheCreation {
    #[serde(default)]
    ephemeral_5m_input_tokens: u64,
    #[serde(default)]
    ephemeral_1h_input_tokens: u64,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeSubagentMeta {
    #[serde(default)]
    agent_type: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    parent_agent_id: Option<String>,
    #[serde(default)]
    workflow_phase: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ClaudeSubagentPathInfo {
    is_subagent_path: bool,
    root_session_id: Option<String>,
    workflow_id: Option<String>,
}

fn non_empty_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn format_claude_subagent_session_id(agent_id: &str) -> String {
    let trimmed = agent_id.trim();
    if trimmed.starts_with("agent-") {
        trimmed.to_string()
    } else {
        format!("agent-{trimmed}")
    }
}

fn inspect_claude_subagent_path(filepath: &Path) -> ClaudeSubagentPathInfo {
    let components: Vec<_> = filepath
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(os_str) => os_str.to_str(),
            _ => None,
        })
        .collect();
    let Some(subagents_idx) = components.iter().rposition(|part| *part == "subagents") else {
        return ClaudeSubagentPathInfo::default();
    };
    let root_session_id = subagents_idx
        .checked_sub(1)
        .and_then(|idx| components.get(idx))
        .and_then(|part| non_empty_trimmed(Some(part)));
    let workflow_id = if components.get(subagents_idx + 1) == Some(&"workflows")
        && subagents_idx + 3 < components.len()
    {
        non_empty_trimmed(components.get(subagents_idx + 2).copied())
    } else {
        None
    };
    ClaudeSubagentPathInfo {
        is_subagent_path: true,
        root_session_id,
        workflow_id,
    }
}

fn load_claude_subagent_meta(filepath: &Path) -> Option<ClaudeSubagentMeta> {
    let raw = fs::read_to_string(filepath.with_extension("meta.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

pub(super) fn find_claude_session_files(dir: &Path) -> Vec<PathBuf> {
    find_jsonl_files(dir)
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("journal.jsonl"))
        .collect()
}

fn claude_content_to_text(content: &serde_json::Value) -> String {
    if let Some(text) = content.as_str() {
        return text.replace('\r', "").replace('\n', " ");
    }

    let mut parts = Vec::new();
    if let Some(items) = content.as_array() {
        for item in items {
            match item.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                "text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        parts.push(text.replace('\r', "").replace('\n', " "));
                    }
                }
                "tool_result" => {
                    if let Some(text) = item.get("content").and_then(|c| c.as_str()) {
                        parts.push(text.replace('\r', "").replace('\n', " "));
                    }
                }
                _ => {}
            }
        }
    }
    parts.join(" ")
}

fn extract_claude_user_prompt_for_session_name(raw_text: &str) -> Option<String> {
    let trimmed = raw_text.trim();
    if trimmed.is_empty()
        || trimmed.starts_with("<local-command-stdout>")
        || trimmed.starts_with("<local-command-stderr>")
        || trimmed.starts_with("<local-command-caveat>")
    {
        return None;
    }
    if let (Some(start), Some(end)) = (
        trimmed.find("<command-args>"),
        trimmed.find("</command-args>"),
    ) {
        let args_start = start + "<command-args>".len();
        if args_start <= end {
            return non_empty_trimmed(Some(&trimmed[args_start..end]));
        }
    }
    Some(trimmed.to_string())
}

/// Fast Mode is reported in usage rather than the API model name.
fn apply_claude_speed_suffix(model: Option<String>, speed: Option<&str>) -> Option<String> {
    let is_fast = speed
        .map(str::trim)
        .is_some_and(|speed| speed.eq_ignore_ascii_case("fast"));
    if !is_fast {
        return model;
    }
    model.map(|model| {
        if model.to_ascii_lowercase().ends_with("-fast") {
            model
        } else {
            format!("{model}-fast")
        }
    })
}

pub(super) fn parse_claude_session_file(filepath: &Path) -> Result<Vec<UsageEntry>, String> {
    let file = File::open(filepath).map_err(|e| format!("無法開啟檔案: {}", e))?;
    let reader = BufReader::new(file);
    let fallback_session_id = filepath
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown-session")
        .to_string();

    let path_info = inspect_claude_subagent_path(filepath);
    let subagent_meta = load_claude_subagent_meta(filepath);
    let mut session_name_selector = InitialUserPromptSelector::default();
    let mut custom_title: Option<String> = None;
    let mut session_cwd: Option<String> = None;
    let mut session_version: Option<String> = None;
    let mut seen_response_indices: HashMap<String, usize> = HashMap::new();
    let mut results: Vec<UsageEntry> = Vec::new();

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(line) => line,
            Err(_) => continue,
        };
        let event: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if session_cwd.is_none() {
            session_cwd = event
                .get("cwd")
                .and_then(|cwd| cwd.as_str())
                .map(|cwd| cwd.to_string());
        }
        if session_version.is_none() {
            session_version = event
                .get("version")
                .and_then(|version| version.as_str())
                .map(|version| version.to_string());
        }

        if event.get("type").and_then(|kind| kind.as_str()) == Some("custom-title") {
            if let Some(title) = event
                .get("customTitle")
                .and_then(|title| title.as_str())
                .map(str::trim)
                .filter(|title| !title.is_empty())
            {
                custom_title = Some(title.to_string());
            }
        }

        let message = match event.get("message") {
            Some(message) => message,
            None => continue,
        };
        let role = message
            .get("role")
            .and_then(|role| role.as_str())
            .unwrap_or("");

        if role == "user" {
            if !event
                .get("isMeta")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                if let Some(content) = message.get("content") {
                    let has_tool_result = content.as_array().is_some_and(|items| {
                        items.iter().any(|item| {
                            item.get("type").and_then(|item_type| item_type.as_str())
                                == Some("tool_result")
                        })
                    });
                    if has_tool_result {
                        session_name_selector.observe_non_user_message();
                    } else if let Some(prompt) = extract_claude_user_prompt_for_session_name(
                        &claude_content_to_text(content),
                    ) {
                        session_name_selector.observe_user_prompt(&prompt);
                    }
                }
            }
            continue;
        }

        if role != "assistant" {
            continue;
        }
        session_name_selector.observe_non_user_message();

        let usage_value = match message.get("usage") {
            Some(usage) => usage.clone(),
            None => continue,
        };
        let usage = match serde_json::from_value::<ClaudeUsage>(usage_value) {
            Ok(usage) => usage,
            Err(_) => continue,
        };

        let response_key = event
            .get("requestId")
            .and_then(|id| id.as_str())
            .or_else(|| message.get("id").and_then(|id| id.as_str()))
            .or_else(|| event.get("uuid").and_then(|id| id.as_str()))
            .unwrap_or("");
        if response_key.is_empty() {
            continue;
        }

        let input = usage.input_tokens;
        let cache_read = usage.cache_read_input_tokens;
        let reported_cache_write = usage.cache_creation_input_tokens;
        let explicit_cache_write_5m = usage.cache_creation.ephemeral_5m_input_tokens;
        let cache_write_1h = usage.cache_creation.ephemeral_1h_input_tokens;
        let explicit_cache_write = explicit_cache_write_5m.saturating_add(cache_write_1h);
        let cache_write = reported_cache_write.max(explicit_cache_write);
        let cache_write_5m = explicit_cache_write_5m
            .saturating_add(reported_cache_write.saturating_sub(explicit_cache_write));
        let output = usage.output_tokens;
        let total = input
            .saturating_add(cache_read)
            .saturating_add(cache_write)
            .saturating_add(output);
        let tokens = TokenStats {
            input,
            output,
            cache_read: Some(cache_read),
            cache_write: Some(cache_write),
            cache_write_5m: Some(cache_write_5m),
            cache_write_1h: Some(cache_write_1h),
            reasoning: None,
            total,
        };

        let reasoning_effort = super::claude_effort_for_assistant_event(&event);
        let model = apply_claude_speed_suffix(
            message
                .get("model")
                .and_then(|model| model.as_str())
                .map(str::to_string),
            usage.speed.as_deref(),
        );
        if let Some(&existing_idx) = seen_response_indices.get(response_key) {
            let existing = &mut results[existing_idx];
            let existing_total = existing.tokens.as_ref().map_or(0, |stats| stats.total);
            if tokens.total >= existing_total {
                existing.tokens = Some(tokens.clone());
                existing.delta_tokens = Some(tokens);
                if model.is_some() {
                    if existing.model != model {
                        existing.reasoning_effort = reasoning_effort.clone();
                    }
                    existing.model = model.clone();
                    existing.model_id = model;
                }
                if reasoning_effort.is_some() {
                    existing.reasoning_effort = reasoning_effort;
                }
            }
            continue;
        }

        let timestamp = event
            .get("timestamp")
            .and_then(|timestamp| timestamp.as_str())
            .unwrap_or("")
            .to_string();
        let event_session_id = non_empty_trimmed(event.get("sessionId").and_then(|id| id.as_str()));
        let event_agent_id = non_empty_trimmed(event.get("agentId").and_then(|id| id.as_str()));
        let is_subagent = fallback_session_id.starts_with("agent-")
            || path_info.is_subagent_path
            || subagent_meta.is_some()
            || event_agent_id.is_some()
            || event
                .get("isSidechain")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
        let (session_id, parent_session_id, session_name, agent_nickname, agent_role) =
            if is_subagent {
                let subagent_session_id = if fallback_session_id.starts_with("agent-") {
                    fallback_session_id.clone()
                } else if let Some(agent_id) = event_agent_id.as_deref() {
                    format_claude_subagent_session_id(agent_id)
                } else {
                    fallback_session_id.clone()
                };
                let parent_id = subagent_meta
                    .as_ref()
                    .and_then(|meta| non_empty_trimmed(meta.parent_agent_id.as_deref()))
                    .map(|id| format_claude_subagent_session_id(&id))
                    .or_else(|| event_session_id.clone())
                    .or_else(|| path_info.root_session_id.clone())
                    .filter(|parent| parent != &subagent_session_id);
                let name = subagent_meta
                    .as_ref()
                    .and_then(|meta| non_empty_trimmed(meta.description.as_deref()))
                    .or_else(|| session_name_selector.selected_name().map(str::to_string))
                    .or_else(|| Some(subagent_session_id.clone()));
                let nickname = subagent_meta
                    .as_ref()
                    .and_then(|meta| non_empty_trimmed(meta.name.as_deref()))
                    .or_else(|| path_info.workflow_id.clone());
                let role = subagent_meta.as_ref().and_then(|meta| {
                    non_empty_trimmed(meta.workflow_phase.as_deref())
                        .or_else(|| non_empty_trimmed(meta.agent_type.as_deref()))
                });
                (subagent_session_id, parent_id, name, nickname, role)
            } else {
                let main_session_id =
                    event_session_id.unwrap_or_else(|| fallback_session_id.clone());
                let name = session_name_selector
                    .selected_name()
                    .map(str::to_string)
                    .or_else(|| Some(fallback_session_id.clone()));
                (main_session_id, None, name, None, None)
            };
        let cwd = event
            .get("cwd")
            .and_then(|cwd| cwd.as_str())
            .map(str::to_string)
            .or_else(|| session_cwd.clone());
        let version = event
            .get("version")
            .and_then(|version| version.as_str())
            .map(str::to_string)
            .or_else(|| session_version.clone());
        let idx = results.len();
        seen_response_indices.insert(response_key.to_string(), idx);

        results.push(UsageEntry {
            timestamp,
            session_id,
            session_name,
            transcript_path: Some(filepath.to_string_lossy().into_owned()),
            cwd,
            version,
            turn_no: (idx + 1) as u32,
            model: model.clone(),
            model_id: model,
            tokens: Some(tokens.clone()),
            delta_tokens: Some(tokens),
            context: None,
            cost: None,
            source_kind: None,
            source_dir_key: None,
            parent_session_id,
            agent_nickname,
            agent_role,
            reasoning_effort,
            session_pricing: None,
        });
    }

    if let Some(title) = custom_title {
        for entry in &mut results {
            if entry.parent_session_id.is_none() && !entry.session_id.starts_with("agent-") {
                entry.session_name = Some(title.clone());
            }
        }
    }

    Ok(results)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn temp_jsonl_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{unique}.jsonl", std::process::id()))
    }

    #[test]
    fn claude_session_name_uses_command_args_and_ignores_meta_and_local_output() {
        let path = temp_jsonl_path("claude-command-name");
        fs::write(&path, concat!(
            "{\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":\"Skill boilerplate\"}}\n",
            "{\"message\":{\"role\":\"user\",\"content\":\"<local-command-caveat>warning</local-command-caveat>\"}}\n",
            "{\"message\":{\"role\":\"user\",\"content\":\"<command-name>/model</command-name><command-args></command-args>\"}}\n",
            "{\"message\":{\"role\":\"user\",\"content\":\"<local-command-stdout>Changed model</local-command-stdout>\"}}\n",
            "{\"message\":{\"role\":\"user\",\"content\":\"<local-command-stderr>warning</local-command-stderr>\"}}\n",
            "{\"message\":{\"role\":\"user\",\"content\":\"<command-name>/plan</command-name><command-args> Build report </command-args>\"}}\n",
            "{\"isMeta\":true,\"message\":{\"role\":\"user\",\"content\":\"Another skill\"}}\n",
            "{\"requestId\":\"r1\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}}\n"
        )).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        assert_eq!(entries[0].session_name.as_deref(), Some("Build report"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn claude_streamed_usage_keeps_largest_totals_and_model_matched_effort() {
        let path = temp_jsonl_path("claude-streamed");
        let events = [
            serde_json::json!({"requestId":"r1","timestamp":"first","effort":"high","message":{"role":"assistant","model":"model-a","usage":{"input_tokens":10,"output_tokens":2}}}),
            serde_json::json!({"requestId":"r1","timestamp":"later","message":{"role":"assistant","model":"model-a","usage":{"input_tokens":10,"output_tokens":8}}}),
            serde_json::json!({"requestId":"r1","effort":"low","message":{"role":"assistant","model":"model-b","usage":{"input_tokens":10,"output_tokens":5}}}),
            serde_json::json!({"requestId":"r2","effort":"high","message":{"role":"assistant","model":"model-a","usage":{"input_tokens":1,"output_tokens":1}}}),
            serde_json::json!({"requestId":"r2","message":{"role":"assistant","model":"model-b","usage":{"input_tokens":1,"output_tokens":2}}}),
        ];
        fs::write(
            &path,
            events
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].turn_no, 1);
        assert_eq!(entries[0].timestamp, "first");
        assert_eq!(entries[0].tokens.as_ref().unwrap().total, 18);
        assert_eq!(entries[0].delta_tokens.as_ref().unwrap().total, 18);
        assert_eq!(entries[0].model.as_deref(), Some("model-a"));
        assert_eq!(entries[0].reasoning_effort.as_deref(), Some("high"));
        assert_eq!(entries[1].turn_no, 2);
        assert_eq!(entries[1].model.as_deref(), Some("model-b"));
        assert_eq!(entries[1].model_id.as_deref(), Some("model-b"));
        assert_eq!(entries[1].reasoning_effort, None);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn claude_subagents_preserve_hierarchy_metadata_and_path_safe_identity() {
        let root = temp_jsonl_path("claude-hierarchy").with_extension("");
        let subagents = root.join("projects/project/parent-session/subagents");
        let workflow = subagents.join("workflows/wf-test");
        fs::create_dir_all(&workflow).unwrap();
        let cases = [
            (
                subagents.join("agent-direct.jsonl"),
                serde_json::json!({"agentType":"Explore","description":"Survey tests"}),
                serde_json::json!({"sessionId":"parent-session","agentId":"direct"}),
                "parent-session",
                "Survey tests",
                None,
                "Explore",
            ),
            (
                subagents.join("agent-nested.jsonl"),
                serde_json::json!({"parentAgentId":"agent-direct","name":"nested","agentType":"general-purpose","description":"Nested work"}),
                serde_json::json!({"sessionId":"parent-session","agentId":"nested"}),
                "agent-direct",
                "Nested work",
                Some("nested"),
                "general-purpose",
            ),
            (
                workflow.join("agent-workflow.jsonl"),
                serde_json::json!({"agentType":"workflow","workflowPhase":"Verify","description":"Verify result"}),
                serde_json::json!({}),
                "parent-session",
                "Verify result",
                Some("wf-test"),
                "Verify",
            ),
        ];
        for (path, meta, mut event, parent, name, nickname, role) in cases {
            fs::write(path.with_extension("meta.json"), meta.to_string()).unwrap();
            event["requestId"] = "r1".into();
            event["message"] = serde_json::json!({"role":"assistant","usage":{"input_tokens":1,"output_tokens":2}});
            fs::write(&path, format!("{}\n{}\n", event, serde_json::json!({"type":"custom-title","customTitle":"Do not rename subagent"}))).unwrap();
            let entries = parse_claude_session_file(&path).unwrap();
            let entry = &entries[0];
            assert_eq!(
                entry.session_id,
                path.file_stem().unwrap().to_str().unwrap()
            );
            assert_eq!(entry.parent_session_id.as_deref(), Some(parent));
            assert_eq!(entry.session_name.as_deref(), Some(name));
            assert_eq!(entry.agent_nickname.as_deref(), nickname);
            assert_eq!(entry.agent_role.as_deref(), Some(role));
            assert!(crate::session_files::is_safe_session_id(&entry.session_id));
            let context = crate::session_files::SessionFileResolutionContext {
                claude_source_dir: Some(&root),
                ..Default::default()
            };
            assert_eq!(
                crate::session_files::resolve_session_file_path(
                    "claude",
                    &entry.session_id,
                    entry.transcript_path.as_deref(),
                    "claude-default",
                    context,
                )
                .unwrap(),
                path.canonicalize().unwrap(),
            );
            let wrong_root = root.join("other-profile");
            fs::create_dir_all(&wrong_root).unwrap();
            assert!(crate::session_files::resolve_session_file_path(
                "claude",
                &entry.session_id,
                entry.transcript_path.as_deref(),
                "claude-profile:other",
                crate::session_files::SessionFileResolutionContext {
                    claude_source_dir: Some(&wrong_root),
                    ..Default::default()
                },
            )
            .is_err());
        }
        fs::write(workflow.join("journal.jsonl"), "{}\n").unwrap();
        assert_eq!(find_claude_session_files(&root.join("projects")).len(), 3);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn claude_subagent_without_sidecar_keeps_filename_identity_and_prompt() {
        let root = temp_jsonl_path("claude-no-meta").with_extension("");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("agent-fallback.jsonl");
        fs::write(path.with_extension("meta.json"), "malformed").unwrap();
        fs::write(&path, concat!(
            "{\"message\":{\"role\":\"user\",\"content\":\"Research task\"}}\n",
            "{\"requestId\":\"r1\",\"sessionId\":\"parent\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}}\n"
        )).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        assert_eq!(entries[0].session_id, "agent-fallback");
        assert_eq!(entries[0].parent_session_id.as_deref(), Some("parent"));
        assert_eq!(entries[0].session_name.as_deref(), Some("Research task"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parse_claude_session_file_deduplicates_request_usage() {
        let path = temp_jsonl_path("claude-parser");

        let content = r#"{"type":"user","sessionId":"session-1","cwd":"/tmp/project","version":"2.1.201","timestamp":"2026-07-04T19:28:48.190Z","uuid":"u1","message":{"role":"user","content":"Build the report"}}
{"type":"user","sessionId":"session-1","cwd":"/tmp/project","version":"2.1.201","timestamp":"2026-07-04T19:28:49.190Z","uuid":"u2","message":{"role":"user","content":"Use monthly grouping"}}
{"type":"assistant","sessionId":"session-1","cwd":"/tmp/project","version":"2.1.201","timestamp":"2026-07-04T19:28:51.753Z","uuid":"a1","requestId":"req_1","message":{"id":"msg_1","role":"assistant","model":"claude-haiku-4-5-20251001","content":[{"type":"thinking","thinking":"working"}],"usage":{"input_tokens":10,"cache_creation_input_tokens":3,"cache_read_input_tokens":7,"output_tokens":2,"cache_creation":{"ephemeral_5m_input_tokens":1,"ephemeral_1h_input_tokens":2}}}}
{"type":"assistant","sessionId":"session-1","cwd":"/tmp/project","version":"2.1.201","timestamp":"2026-07-04T19:28:51.948Z","uuid":"a2","requestId":"req_1","message":{"id":"msg_1","role":"assistant","model":"claude-haiku-4-5-20251001","content":[{"type":"text","text":"Done"}],"usage":{"input_tokens":10,"cache_creation_input_tokens":3,"cache_read_input_tokens":7,"output_tokens":5,"cache_creation":{"ephemeral_5m_input_tokens":1,"ephemeral_1h_input_tokens":2}}}}
"#;

        fs::write(&path, content).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.session_id, "session-1");
        assert_eq!(entry.session_name.as_deref(), Some("Use monthly grouping"));
        assert_eq!(entry.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(entry.version.as_deref(), Some("2.1.201"));
        assert_eq!(entry.model.as_deref(), Some("claude-haiku-4-5-20251001"));

        let tokens = entry.tokens.as_ref().unwrap();
        assert_eq!(tokens.input, 10);
        assert_eq!(tokens.cache_write, Some(3));
        assert_eq!(tokens.cache_write_5m, Some(1));
        assert_eq!(tokens.cache_write_1h, Some(2));
        assert_eq!(tokens.cache_read, Some(7));
        assert_eq!(tokens.output, 5);
        assert_eq!(tokens.total, 25);
    }

    #[test]
    fn parse_claude_session_file_defaults_unclassified_cache_writes_to_5m() {
        let path = temp_jsonl_path("claude-cache-default");
        let content = r#"{"type":"assistant","sessionId":"session-cache-default","timestamp":"2026-07-04T19:28:51.753Z","uuid":"a1","requestId":"req_1","message":{"id":"msg_1","role":"assistant","model":"claude-haiku-4-5-20251001","content":[{"type":"text","text":"Done"}],"usage":{"input_tokens":10,"cache_creation_input_tokens":3,"cache_read_input_tokens":7,"output_tokens":5}}}
"#;

        fs::write(&path, content).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        let tokens = entries[0].tokens.as_ref().unwrap();
        assert_eq!(tokens.input, 10);
        assert_eq!(tokens.cache_write, Some(3));
        assert_eq!(tokens.cache_write_5m, Some(3));
        assert_eq!(tokens.cache_write_1h, Some(0));
        assert_eq!(tokens.total, 25);
    }

    #[test]
    fn parse_claude_session_file_uses_latest_custom_title_after_usage() {
        let path = temp_jsonl_path("claude-renamed");
        let content = r#"{"type":"user","sessionId":"renamed","message":{"role":"user","content":"Original prompt"}}
{"type":"assistant","sessionId":"renamed","timestamp":"2026-09-24T09:00:00Z","requestId":"req_1","message":{"id":"msg_1","role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2}}}
{"type":"custom-title","sessionId":"renamed","customTitle":"First name"}
{"type":"custom-title","sessionId":"renamed","customTitle":"Final name"}
"#;
        fs::write(&path, content).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_name.as_deref(), Some("Final name"));
        assert_eq!(entries[0].tokens.as_ref().unwrap().total, 3);
    }

    #[test]
    fn parse_claude_session_file_uses_response_effort_with_per_turn_precedence() {
        let path = temp_jsonl_path("claude-effort");
        let content = r#"{"type":"assistant","timestamp":"2026-09-24T09:00:00Z","requestId":"req_1","effort":"high","perTurnEffort":null,"message":{"id":"msg_1","role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2}}}
{"type":"assistant","timestamp":"2026-09-24T09:01:00Z","requestId":"req_2","effort":"low","perTurnEffort":"medium","message":{"id":"msg_2","role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2}}}
{"type":"assistant","timestamp":"2026-09-24T09:02:00Z","requestId":"req_3","message":{"id":"msg_3","role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2}}}
{"type":"assistant","timestamp":"2026-09-24T09:03:00Z","requestId":"req_4","effort":"high","perTurnEffort":"low","message":{"id":"msg_4","role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":1,"output_tokens":2}}}
"#;
        fs::write(&path, content).unwrap();

        let entries = parse_claude_session_file(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.reasoning_effort.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("high"), Some("medium"), None, Some("low")]
        );
    }
    #[test]
    fn apply_claude_speed_suffix_only_marks_fast_mode() {
        for speed in [Some("fast"), Some(" FAST ")] {
            assert_eq!(
                apply_claude_speed_suffix(Some("claude-opus-5-5".into()), speed),
                Some("claude-opus-5-5-fast".into())
            );
        }
        for speed in [Some("standard"), None, Some("")] {
            assert_eq!(
                apply_claude_speed_suffix(Some("claude-opus-5-5".into()), speed),
                Some("claude-opus-5-5".into())
            );
        }
        assert_eq!(
            apply_claude_speed_suffix(Some("claude-opus-5-5-fast".into()), Some("fast")),
            Some("claude-opus-5-5-fast".into())
        );
        assert_eq!(apply_claude_speed_suffix(None, Some("fast")), None);
    }

    #[test]
    fn parse_claude_session_file_marks_fast_mode_turns_for_pricing() {
        let path = temp_jsonl_path("claude-fast-mode");
        let content = r#"{"type":"assistant","sessionId":"session-fast","timestamp":"2026-10-10T01:00:00Z","requestId":"fast","effort":"high","message":{"role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":5,"speed":"fast"}}}
{"type":"assistant","sessionId":"session-fast","timestamp":"2026-10-10T01:00:01Z","requestId":"standard","message":{"role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":5,"speed":"standard"}}}
{"type":"assistant","sessionId":"session-fast","timestamp":"2026-10-10T01:00:02Z","requestId":"legacy","message":{"role":"assistant","model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":5}}}
"#;
        fs::write(&path, content).unwrap();
        let entries = parse_claude_session_file(&path).unwrap();
        fs::remove_file(path).unwrap();
        assert_eq!(entries.len(), 3);
        for (entry, model) in
            entries
                .iter()
                .zip(["claude-opus-5-5-fast", "claude-opus-5-5", "claude-opus-5-5"])
        {
            assert_eq!(entry.model.as_deref(), Some(model));
            assert_eq!(entry.model_id.as_deref(), Some(model));
            assert_eq!(entry.tokens.as_ref().unwrap().total, 15);
        }
        assert_eq!(entries[0].reasoning_effort.as_deref(), Some("high"));
    }
}
