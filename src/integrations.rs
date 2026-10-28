use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Take},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_PROBE_BYTES: u64 = 256 * 1024;

#[derive(Default)]
pub struct OptionalIntegrationGate {
    enabled: AtomicBool,
}

impl OptionalIntegrationGate {
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Found,
    Missing,
    Invalid,
    Unverified,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactStatus {
    pub label: String,
    pub path_hint: String,
    pub state: ArtifactState,
    pub detail: String,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionalIntegrationStatus {
    pub id: String,
    pub state: ArtifactState,
    pub detail: String,
    pub import_status: String,
    pub artifacts: Vec<ArtifactStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionalIntegrationsStatusResponse {
    pub sources: Vec<OptionalIntegrationStatus>,
}

fn home_or_current() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn codeburn_cache_dir(home: &Path) -> PathBuf {
    crate::paths::env_path("CODEBURN_CACHE_DIR").unwrap_or_else(|| home.join(".cache/codeburn"))
}

fn magpie_config_dir(home: &Path) -> PathBuf {
    crate::paths::env_path("XDG_CONFIG_HOME")
        .unwrap_or_else(|| home.join(".config"))
        .join("magpie")
}

fn file_bytes(path: &Path) -> Result<(Vec<u8>, u64), String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("not a regular file".to_string());
    }
    if metadata.len() > MAX_PROBE_BYTES {
        return Err(format!(
            "file exceeds {} KiB; contents were not read",
            MAX_PROBE_BYTES / 1024
        ));
    }
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut reader: Take<fs::File> = file.take(MAX_PROBE_BYTES);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok((bytes, metadata.len()))
}

fn missing_file(label: &str, path_hint: &str) -> ArtifactStatus {
    ArtifactStatus {
        label: label.to_string(),
        path_hint: path_hint.to_string(),
        state: ArtifactState::Missing,
        detail: "file was not found".to_string(),
        bytes: None,
    }
}

fn inspect_json(path: &Path, label: &str, path_hint: &str) -> ArtifactStatus {
    if !path.exists() {
        return missing_file(label, path_hint);
    }
    match file_bytes(path) {
        Ok((bytes, size)) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(_) => ArtifactStatus {
                label: label.to_string(),
                path_hint: path_hint.to_string(),
                state: ArtifactState::Found,
                detail: "valid JSON; contents are not returned to the browser".to_string(),
                bytes: Some(size),
            },
            Err(error) => ArtifactStatus {
                label: label.to_string(),
                path_hint: path_hint.to_string(),
                state: ArtifactState::Invalid,
                detail: format!("invalid JSON: {error}"),
                bytes: Some(size),
            },
        },
        Err(detail) => ArtifactStatus {
            label: label.to_string(),
            path_hint: path_hint.to_string(),
            state: ArtifactState::Unverified,
            detail,
            bytes: None,
        },
    }
}

fn inspect_magpie_usage(path: &Path) -> ArtifactStatus {
    const PATH_HINT: &str =
        "$XDG_CONFIG_HOME/magpie/usage.jsonl（預設 ~/.config/magpie/usage.jsonl）";
    if !path.exists() {
        return missing_file("Magpie usage.jsonl", PATH_HINT);
    }
    match file_bytes(path) {
        Ok((bytes, size)) => {
            let record = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|text| text.lines().find(|line| !line.trim().is_empty()))
                .and_then(|line| serde_json::from_str::<Value>(line).ok());
            let is_usage_record = record.as_ref().is_some_and(|value| {
                ["t", "agent", "provider", "model", "rid"]
                    .iter()
                    .all(|field| value.get(field).and_then(Value::as_str).is_some())
            });
            if is_usage_record {
                ArtifactStatus {
                    label: "Magpie usage.jsonl".to_string(),
                    path_hint: PATH_HINT.to_string(),
                    state: ArtifactState::Found,
                    detail:
                        "the first non-empty row has Magpie's required request fields; record contents are not returned"
                            .to_string(),
                    bytes: Some(size),
                }
            } else {
                ArtifactStatus {
                    label: "Magpie usage.jsonl".to_string(),
                    path_hint: PATH_HINT.to_string(),
                    state: ArtifactState::Invalid,
                    detail: "the first non-empty row is missing one of t, agent, provider, model, or rid"
                        .to_string(),
                    bytes: Some(size),
                }
            }
        }
        Err(detail) => ArtifactStatus {
            label: "Magpie usage.jsonl".to_string(),
            path_hint: PATH_HINT.to_string(),
            state: ArtifactState::Unverified,
            detail,
            bytes: None,
        },
    }
}

fn status_from_artifacts(artifacts: &[ArtifactStatus]) -> ArtifactState {
    if artifacts
        .iter()
        .any(|artifact| artifact.state == ArtifactState::Invalid)
    {
        ArtifactState::Invalid
    } else if artifacts
        .iter()
        .any(|artifact| artifact.state == ArtifactState::Found)
    {
        ArtifactState::Found
    } else if artifacts
        .iter()
        .all(|artifact| artifact.state == ArtifactState::Missing)
    {
        ArtifactState::Missing
    } else {
        ArtifactState::Unverified
    }
}

pub fn inspect_optional_sources() -> OptionalIntegrationsStatusResponse {
    let home = home_or_current();
    let codeburn_dir = codeburn_cache_dir(&home);
    let magpie_dir = magpie_config_dir(&home);
    let codeburn_artifacts = vec![
        inspect_json(
            &codeburn_dir.join("daily-cache.json"),
            "CodeBurn daily cache",
            "$CODEBURN_CACHE_DIR/daily-cache.json（預設 ~/.cache/codeburn/daily-cache.json）",
        ),
        ArtifactStatus {
            label: "CodeBurn cache directory".to_string(),
            path_hint: "$CODEBURN_CACHE_DIR（預設 ~/.cache/codeburn）".to_string(),
            state: if codeburn_dir.is_dir() {
                ArtifactState::Found
            } else {
                ArtifactState::Missing
            },
            detail: if codeburn_dir.is_dir() {
                "CodeBurn cache directory found; this page does not parse or import its history."
                    .to_string()
            } else {
                "CodeBurn cache directory was not found".to_string()
            },
            bytes: None,
        },
    ];
    let codeburn_state = match codeburn_artifacts[0].state {
        ArtifactState::Found => ArtifactState::Found,
        ArtifactState::Invalid => ArtifactState::Invalid,
        ArtifactState::Missing | ArtifactState::Unverified => ArtifactState::Unverified,
    };
    let magpie_artifacts = vec![
        inspect_magpie_usage(&magpie_dir.join("usage.jsonl")),
        inspect_json(
            &magpie_dir.join("quotas.json"),
            "Magpie quotas.json",
            "$XDG_CONFIG_HOME/magpie/quotas.json（預設 ~/.config/magpie/quotas.json）",
        ),
    ];

    OptionalIntegrationsStatusResponse {
        sources: vec![
            OptionalIntegrationStatus {
                id: "ccusage".to_string(),
                state: ArtifactState::Unverified,
                detail: "ccusage reads agent data directly and has no independently identifiable report file; it is not imported to avoid double counting existing Claude or Codex sessions."
                    .to_string(),
                import_status: "not_imported".to_string(),
                artifacts: Vec::new(),
            },
            OptionalIntegrationStatus {
                id: "codeburn".to_string(),
                state: codeburn_state,
                detail: "only CodeBurn cache metadata is checked; it is not imported and cannot be added to existing monthly reports.".to_string(),
                import_status: "not_imported".to_string(),
                artifacts: codeburn_artifacts,
            },
            OptionalIntegrationStatus {
                id: "magpie".to_string(),
                state: status_from_artifacts(&magpie_artifacts),
                detail: "only minimum usage and quota file formats are checked; settings, account, and credential files are not read.".to_string(),
                import_status: "not_imported".to_string(),
                artifacts: magpie_artifacts,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("token-usage-insights-{name}-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn json_probe_reports_found_and_invalid_without_returning_content() {
        let root = temp_dir("integration-json");
        let valid = root.join("valid.json");
        let invalid = root.join("invalid.json");
        fs::write(&valid, r#"{"secret":"must not appear"}"#).unwrap();
        fs::write(&invalid, "not json").unwrap();

        let found = inspect_json(&valid, "Valid", "valid.json");
        let invalid = inspect_json(&invalid, "Invalid", "invalid.json");
        assert_eq!(found.state, ArtifactState::Found);
        assert_eq!(invalid.state, ArtifactState::Invalid);
        assert!(!found.detail.contains("must not appear"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn magpie_probe_requires_the_documented_request_identity_fields() {
        let root = temp_dir("integration-magpie");
        let usage = root.join("usage.jsonl");
        fs::write(
            &usage,
            "{\"t\":\"2026-10-01T00:00:00Z\",\"agent\":\"omp\",\"provider\":\"x\",\"model\":\"m\",\"rid\":\"r\"}\n",
        )
        .unwrap();
        assert_eq!(inspect_magpie_usage(&usage).state, ArtifactState::Found);
        fs::write(&usage, "{\"agent\":\"omp\"}\n").unwrap();
        assert_eq!(inspect_magpie_usage(&usage).state, ArtifactState::Invalid);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn optional_gate_is_disabled_until_the_browser_enables_it() {
        let gate = OptionalIntegrationGate::default();
        assert!(!gate.is_enabled());
        gate.set_enabled(true);
        assert!(gate.is_enabled());
        gate.set_enabled(false);
        assert!(!gate.is_enabled());
    }
}
