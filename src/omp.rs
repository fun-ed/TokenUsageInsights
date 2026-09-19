//! OMP (<https://omp.sh/>) is an open-source fork of the Pi Coding Agent
//! (<https://pi.dev/>, source: <https://github.com/can1357/oh-my-pi>) and
//! persists sessions using the exact same tree-structured JSONL format under
//! `<dir>/agent/sessions/`. Parsing logic is fully shared with `crate::pi`;
//! this module only carries OMP-specific identifiers.
use crate::db::UsageEntry;
use std::path::{Path, PathBuf};

pub(crate) const SOURCE_KIND: &str = "omp-session";

pub(crate) fn find_session_files(dir: &Path) -> Vec<PathBuf> {
    crate::pi::find_session_files(dir)
}

pub(crate) fn parse_session_usage_file(path: &Path) -> Result<Vec<UsageEntry>, String> {
    crate::pi::parse_session_usage_file(path, SOURCE_KIND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn parse_session_usage_file_collects_latest_omp_usage_categories() {
        let root = std::env::temp_dir().join(format!(
            "token-usage-insights-omp-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let parent_stem = "2024-12-03T14-00-00_parent-id";
        let parent_path = root.join(format!("{parent_stem}.jsonl"));
        fs::write(
            &parent_path,
            r#"{"type":"session","version":3,"id":"parent-id","title":"Parent title","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}"#,
        )
        .unwrap();

        let path = root.join("session.jsonl");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session","version":3,"id":"omp-sess-1","parentSession":"{}","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}}"#,
            parent_path.display()
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"session_init","id":"init","parentId":null,"timestamp":"2024-12-03T14:00:01.000Z","systemPrompt":"You are a reviewer.","task":"Inspect code","tools":["read"],"agent":"reviewer","resolvedModel":"openai-codex/gpt-5.6-terra"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"message","id":"m2","parentId":null,"timestamp":"2024-12-03T14:00:02.000Z","message":{{"role":"assistant","content":[{{"type":"text","text":"Hi!"}}],"provider":"openai-codex","model":"openai-codex/gpt-5.6-terra","usage":{{"input":10,"output":5,"reasoningTokens":16,"totalTokens":15,"cost":{{"total":0.0005}}}},"stopReason":"stop"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"model_usage","id":"m3","parentId":"m2","timestamp":"2024-12-03T14:00:03.000Z","purpose":"preflight","role":"tiny","api":"openai-codex-responses","provider":"openai-codex","model":"openai-codex/gpt-5.6-terra","usage":{{"input":4,"output":1,"totalTokens":5,"cost":{{"total":0.0002}}}},"stopReason":"stop"}}"#
        )
        .unwrap();

        let entries = parse_session_usage_file(&path).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id, "omp-sess-1");
        assert_eq!(entries[0].session_name.as_deref(), Some("Inspect code"));
        assert_eq!(entries[0].model.as_deref(), Some("openai/gpt-5.6-terra"));
        assert_eq!(entries[0].parent_session_id.as_deref(), Some("parent-id"));
        assert_eq!(entries[0].agent_nickname.as_deref(), Some("reviewer"));
        assert_eq!(entries[0].agent_role.as_deref(), Some("subagent"));
        assert_eq!(
            entries[0]
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.reasoning),
            Some(16)
        );
        assert_eq!(entries[1].agent_role.as_deref(), Some("subagent:preflight"));
        assert_eq!(entries[1].source_kind.as_deref(), Some(SOURCE_KIND));

        let advisor_dir = root.join(parent_stem);
        fs::create_dir_all(&advisor_dir).unwrap();
        let advisor_path = advisor_dir.join("__advisor.jsonl");
        fs::write(
            &advisor_path,
            r#"{"type":"message","id":"advisor","parentId":null,"timestamp":"2024-12-03T14:00:04.000Z","message":{"role":"assistant","content":[{"type":"text","text":"Advisory result"}],"provider":"openai-codex","model":"gpt-5.6-terra","usage":{"input":3,"output":2,"totalTokens":5}}}"#,
        )
        .unwrap();
        let advisor_entries = parse_session_usage_file(&advisor_path).unwrap();
        assert_eq!(advisor_entries.len(), 1);
        assert_eq!(
            advisor_entries[0].parent_session_id.as_deref(),
            Some("parent-id")
        );
        assert_eq!(
            advisor_entries[0].agent_nickname.as_deref(),
            Some("advisor")
        );
        assert_eq!(advisor_entries[0].agent_role.as_deref(), Some("advisor"));

        let agent_path = advisor_dir.join("CoreSourceResearch.jsonl");
        fs::write(
            &agent_path,
            r#"{"type":"session","version":3,"id":"agent-id","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}
{"type":"message","id":"agent","parentId":null,"timestamp":"2024-12-03T14:00:05.000Z","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-terra","usage":{"input":3,"output":2,"totalTokens":5}}}"#,
        )
        .unwrap();
        let agent_entries = parse_session_usage_file(&agent_path).unwrap();
        assert_eq!(
            agent_entries[0].parent_session_id.as_deref(),
            Some("parent-id")
        );
        assert_eq!(
            agent_entries[0].agent_nickname.as_deref(),
            Some("CoreSourceResearch")
        );
        assert_eq!(agent_entries[0].agent_role.as_deref(), Some("subagent"));

        let nested_advisor_dir = advisor_dir.join("CoreSourceResearch");
        fs::create_dir_all(&nested_advisor_dir).unwrap();
        let nested_advisor_path = nested_advisor_dir.join("__advisor.arch.jsonl");
        fs::write(
            &nested_advisor_path,
            r#"{"type":"message","id":"nested-advisor","parentId":null,"timestamp":"2024-12-03T14:00:05.500Z","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-terra","usage":{"input":3,"output":2,"totalTokens":5}}}"#,
        )
        .unwrap();
        let nested_advisor_entries = parse_session_usage_file(&nested_advisor_path).unwrap();
        assert_eq!(
            nested_advisor_entries[0].parent_session_id.as_deref(),
            Some("agent-id")
        );
        assert_eq!(
            nested_advisor_entries[0].agent_nickname.as_deref(),
            Some("advisor:arch")
        );
        assert_eq!(
            nested_advisor_entries[0].agent_role.as_deref(),
            Some("advisor")
        );

        let reserved_name_path = advisor_dir.join("__advisor-2.jsonl");
        fs::write(
            &reserved_name_path,
            r#"{"type":"session","version":3,"id":"reserved-agent","timestamp":"2024-12-03T14:00:05.750Z","cwd":"/tmp/project"}
{"type":"message","id":"reserved","parentId":null,"timestamp":"2024-12-03T14:00:05.900Z","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-terra","usage":{"input":3,"output":2,"totalTokens":5}}}"#,
        )
        .unwrap();
        let reserved_name_entries = parse_session_usage_file(&reserved_name_path).unwrap();
        assert_eq!(
            reserved_name_entries[0].agent_nickname.as_deref(),
            Some("__advisor-2")
        );
        assert_eq!(
            reserved_name_entries[0].agent_role.as_deref(),
            Some("subagent")
        );

        let main_path = root.join("main.jsonl");
        fs::write(
            &main_path,
            r#"{"type":"title","v":1,"title":"Main title"}
{"type":"session","version":3,"id":"main-id","title":"Header title","timestamp":"2024-12-03T14:00:00.000Z","cwd":"/tmp/project"}
{"type":"message","id":"main","parentId":null,"timestamp":"2024-12-03T14:00:06.000Z","message":{"role":"assistant","provider":"openai-codex","model":"gpt-5.6-terra","usage":{"input":3,"output":2,"totalTokens":5}}}"#,
        )
        .unwrap();
        let main_entries = parse_session_usage_file(&main_path).unwrap();
        assert_eq!(main_entries[0].session_name.as_deref(), Some("Main title"));

        fs::remove_dir_all(&root).ok();
    }
}
