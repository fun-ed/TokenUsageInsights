use chrono::{Datelike, Duration, FixedOffset, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read, Take},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_PROBE_BYTES: u64 = 256 * 1024;
const MAX_MAGPIE_SCAN_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MAGPIE_LINE_BYTES: usize = 256 * 1024;

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
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => match fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                let _ = file.take(MAX_PROBE_BYTES).read_to_end(&mut bytes);
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
                        detail: "the first non-empty row has Magpie's required request fields; record contents are not returned".to_string(),
                        bytes: Some(metadata.len()),
                    }
                } else {
                    ArtifactStatus {
                        label: "Magpie usage.jsonl".to_string(),
                        path_hint: PATH_HINT.to_string(),
                        state: ArtifactState::Invalid,
                        detail: "the first non-empty row is missing one of t, agent, provider, model, or rid".to_string(),
                        bytes: Some(metadata.len()),
                    }
                }
            }
            Err(error) => ArtifactStatus {
                label: "Magpie usage.jsonl".to_string(),
                path_hint: PATH_HINT.to_string(),
                state: ArtifactState::Unverified,
                detail: error.to_string(),
                bytes: None,
            },
        },
        Ok(_) => ArtifactStatus {
            label: "Magpie usage.jsonl".to_string(),
            path_hint: PATH_HINT.to_string(),
            state: ArtifactState::Unverified,
            detail: "not a regular file".to_string(),
            bytes: None,
        },
        Err(error) => ArtifactStatus {
            label: "Magpie usage.jsonl".to_string(),
            path_hint: PATH_HINT.to_string(),
            state: ArtifactState::Unverified,
            detail: error.to_string(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagpiePeriod {
    Today,
    Days7,
    Days30,
    Month,
    All,
}

impl MagpiePeriod {
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("30d") {
            "today" => Some(Self::Today),
            "7d" => Some(Self::Days7),
            "30d" => Some(Self::Days30),
            "month" => Some(Self::Month),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    fn includes(self, date: NaiveDate, today: NaiveDate) -> bool {
        match self {
            Self::Today => date == today,
            Self::Days7 => date >= today - Duration::days(6) && date <= today,
            Self::Days30 => date >= today - Duration::days(29) && date <= today,
            Self::Month => date.year() == today.year() && date.month() == today.month(),
            Self::All => true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::Month => "month",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MagpieDailyUsage {
    pub date: String,
    pub events: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub computed_api_equivalent_usd: Option<f64>,
    pub priced_events: u64,
    pub unpriced_events: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagpieAgentUsage {
    pub agent: String,
    pub events: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MagpieModelUsage {
    pub model: String,
    pub events: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub computed_api_equivalent_usd: Option<f64>,
    pub priced_events: u64,
    pub unpriced_events: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MagpieUsageResponse {
    pub period: String,
    pub timezone: &'static str,
    pub agent_dimension: &'static str,
    pub selected_agent: Option<String>,
    pub coverage_start: Option<String>,
    pub coverage_end: Option<String>,
    pub events: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reported_cost_usd: Option<f64>,
    pub malformed_rows: u64,
    pub skipped_rows: u64,
    pub duplicate_rows: u64,
    pub truncated: bool,
    pub daily: Vec<MagpieDailyUsage>,
    pub agents: Vec<MagpieAgentUsage>,
    pub available_agents: Vec<MagpieAgentUsage>,
    pub models: Vec<MagpieModelUsage>,
}

fn value_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn magpie_usage_path() -> PathBuf {
    magpie_config_dir(&home_or_current()).join("usage.jsonl")
}

fn parse_magpie_usage_reader<R: Read>(
    reader: R,
    period: MagpiePeriod,
    selected_agent: Option<&str>,
    today: NaiveDate,
    offset: FixedOffset,
    pricing_rules: &crate::pricing::PreparedPricingRules,
) -> Result<MagpieUsageResponse, String> {
    let mut reader = BufReader::new(reader.take(MAX_MAGPIE_SCAN_BYTES + 1));
    let mut daily = BTreeMap::<NaiveDate, MagpieDailyUsage>::new();
    let mut agent_counts = BTreeMap::<String, u64>::new();
    let mut available_agent_counts = BTreeMap::<String, u64>::new();
    let mut models = BTreeMap::<String, MagpieModelUsage>::new();
    let mut seen_request_ids = HashSet::new();
    let mut events = 0;
    let mut input_tokens = 0;
    let mut output_tokens = 0;
    let mut cache_read_tokens = 0;
    let mut cache_write_tokens = 0;
    let mut reported_cost = 0.0;
    let mut reported_cost_rows = 0;
    let mut malformed_rows = 0;
    let mut skipped_rows = 0;
    let mut duplicate_rows = 0;
    let truncated = false;
    let mut coverage_start = None::<NaiveDate>;
    let mut coverage_end = None::<NaiveDate>;
    let mut line = Vec::new();

    loop {
        line.clear();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        if line.len() > MAX_MAGPIE_LINE_BYTES {
            skipped_rows += 1;
            continue;
        }
        let line = match std::str::from_utf8(&line) {
            Ok(line) if !line.trim().is_empty() => line,
            Ok(_) => continue,
            Err(_) => {
                malformed_rows += 1;
                continue;
            }
        };
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                malformed_rows += 1;
                continue;
            }
        };
        let Some(timestamp) = value.get("t").and_then(Value::as_str) else {
            malformed_rows += 1;
            continue;
        };
        let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp) else {
            malformed_rows += 1;
            continue;
        };
        let Some(agent) = value
            .get("agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            malformed_rows += 1;
            continue;
        };
        let Some(request_id) = value
            .get("rid")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            malformed_rows += 1;
            continue;
        };
        let date = timestamp.with_timezone(&offset).date_naive();
        if !period.includes(date, today) {
            continue;
        }
        if selected_agent.is_some_and(|selected| selected != agent) {
            continue;
        }
        if !seen_request_ids.insert(request_id.to_string()) {
            duplicate_rows += 1;
            continue;
        }
        *available_agent_counts.entry(agent.to_string()).or_default() += 1;
        let row_input = value_u64(&value, "in");
        let row_output = value_u64(&value, "out");
        let row_cache_read = value_u64(&value, "cache_read");
        let row_cache_write = value_u64(&value, "cache_write");
        let model = value
            .get("model")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .unwrap_or("unknown");
        let bucket = daily.entry(date).or_insert_with(|| MagpieDailyUsage {
            date: date.to_string(),
            ..Default::default()
        });
        bucket.events += 1;
        bucket.input_tokens += row_input;
        bucket.output_tokens += row_output;
        bucket.cache_read_tokens += row_cache_read;
        bucket.cache_write_tokens += row_cache_write;
        events += 1;
        input_tokens += row_input;
        output_tokens += row_output;
        cache_read_tokens += row_cache_read;
        cache_write_tokens += row_cache_write;
        *agent_counts.entry(agent.to_string()).or_default() += 1;
        let model_usage = models
            .entry(model.to_string())
            .or_insert_with(|| MagpieModelUsage {
                model: model.to_string(),
                ..Default::default()
            });
        model_usage.events += 1;
        model_usage.input_tokens += row_input;
        model_usage.output_tokens += row_output;
        model_usage.cache_read_tokens += row_cache_read;
        model_usage.cache_write_tokens += row_cache_write;
        if row_cache_write == 0 && (row_input > 0 || row_output > 0 || row_cache_read > 0) {
            match pricing_rules.calculate_usage_cost(
                Some(model),
                row_input,
                row_output,
                row_cache_read,
                0,
                0,
            ) {
                Ok(cost) => {
                    model_usage.computed_api_equivalent_usd =
                        Some(model_usage.computed_api_equivalent_usd.unwrap_or(0.0) + cost);
                    model_usage.priced_events += 1;
                    bucket.computed_api_equivalent_usd =
                        Some(bucket.computed_api_equivalent_usd.unwrap_or(0.0) + cost);
                    bucket.priced_events += 1;
                }
                Err(_) => {
                    model_usage.unpriced_events += 1;
                    bucket.unpriced_events += 1;
                }
            }
        } else {
            model_usage.unpriced_events += 1;
            bucket.unpriced_events += 1;
        }
        coverage_start = Some(coverage_start.map_or(date, |current| current.min(date)));
        coverage_end = Some(coverage_end.map_or(date, |current| current.max(date)));
        if let Some(cost) = value
            .get("cost")
            .and_then(Value::as_f64)
            .filter(|cost| cost.is_finite() && *cost >= 0.0)
        {
            reported_cost += cost;
            reported_cost_rows += 1;
        }
    }
    let daily = daily.into_values().collect();
    let agents = agent_counts
        .into_iter()
        .map(|(agent, events)| MagpieAgentUsage { agent, events })
        .collect::<Vec<_>>();
    let available_agents = available_agent_counts
        .into_iter()
        .map(|(agent, events)| MagpieAgentUsage { agent, events })
        .collect::<Vec<_>>();
    let models = models.into_values().collect::<Vec<_>>();
    Ok(MagpieUsageResponse {
        period: period.label().to_string(),
        timezone: "GMT+8 (+08:00)",
        agent_dimension: "Magpie event.agent (initiating agent, not provider or route)",
        selected_agent: selected_agent.map(str::to_string),
        coverage_start: coverage_start.map(|date| date.to_string()),
        coverage_end: coverage_end.map(|date| date.to_string()),
        events,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        reported_cost_usd: (events > 0 && reported_cost_rows == events).then_some(reported_cost),
        malformed_rows,
        skipped_rows,
        duplicate_rows,
        truncated,
        daily,
        agents,
        available_agents,
        models,
    })
}

pub fn load_magpie_usage(
    period: MagpiePeriod,
    selected_agent: Option<&str>,
) -> Result<MagpieUsageResponse, String> {
    let path = magpie_usage_path();
    let file =
        fs::File::open(&path).map_err(|error| format!("無法讀取 Magpie usage.jsonl：{error}"))?;
    let offset = FixedOffset::east_opt(8 * 60 * 60).expect("valid fixed offset");
    let pricing_rules = crate::pricing::load_prepared_pricing_rules();
    let mut usage = parse_magpie_usage_reader(
        file,
        period,
        selected_agent,
        Utc::now().with_timezone(&offset).date_naive(),
        offset,
        &pricing_rules,
    )?;
    usage.truncated = fs::metadata(path)
        .map(|metadata| metadata.len() > MAX_MAGPIE_SCAN_BYTES)
        .unwrap_or(false);
    Ok(usage)
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

    #[test]
    fn magpie_usage_uses_agent_and_utc_day_without_mixing_provider_or_cache_into_output() {
        let log = concat!(
            "{\"t\":\"2026-10-28T23:30:00-02:00\",\"agent\":\"omp\",\"provider\":\"anthropic\",\"model\":\"m\",\"rid\":\"1\",\"in\":2,\"out\":3,\"cache_read\":5}\n",
            "not-json\n",
            "{\"t\":\"2026-10-29T01:00:00Z\",\"agent\":\"claude-code\",\"provider\":\"anthropic\",\"model\":\"m\",\"rid\":\"2\",\"in\":7,\"out\":11}\n",
            "{\"t\":\"2026-10-29T02:00:00Z\",\"agent\":\"omp\",\"provider\":\"openai\",\"model\":\"other\",\"rid\":\"1\",\"in\":99}\n",
        );
        let usage = parse_magpie_usage_reader(
            log.as_bytes(),
            MagpiePeriod::All,
            Some("omp"),
            NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(),
            FixedOffset::east_opt(8 * 60 * 60).unwrap(),
            &crate::pricing::PreparedPricingRules::from_rules(Vec::new()),
        )
        .unwrap();
        assert_eq!(usage.events, 1);
        assert_eq!(usage.daily[0].date, "2026-10-29");
        assert_eq!(usage.output_tokens, 3);
        assert_eq!(usage.cache_read_tokens, 5);
        assert_eq!(usage.malformed_rows, 1);
        assert_eq!(usage.duplicate_rows, 1);
        assert_eq!(usage.agents[0].agent, "omp");
        assert_eq!(usage.timezone, "GMT+8 (+08:00)");
    }

    #[test]
    fn magpie_month_period_uses_gmt_plus_eight_and_keeps_unknown_or_cache_write_pricing_unpriced() {
        let log = concat!(
            "{\"t\":\"2026-09-30T16:30:00Z\",\"agent\":\"omp\",\"model\":\"priced\",\"rid\":\"a\",\"in\":1000000}\n",
            "{\"t\":\"2026-10-01T00:30:00Z\",\"agent\":\"omp\",\"model\":\"unknown\",\"rid\":\"b\",\"in\":1}\n",
            "{\"t\":\"2026-10-01T01:30:00Z\",\"agent\":\"omp\",\"model\":\"priced\",\"rid\":\"c\",\"cache_write\":2}\n",
            "{\"t\":\"2026-10-01T02:30:00Z\",\"agent\":\"omp\",\"model\":\"zero-unknown\",\"rid\":\"d\"}\n",
            "{\"t\":\"2026-10-02T02:30:00Z\",\"agent\":\"omp\",\"model\":\"unknown\",\"rid\":\"e\",\"in\":1}\n",
        );
        let rules =
            crate::pricing::PreparedPricingRules::from_rules(vec![crate::pricing::PricingRule {
                model_name: "priced".to_string(),
                input_price: 2.0,
                cache_input_price: 0.2,
                output_price: 4.0,
            }]);
        let usage = parse_magpie_usage_reader(
            log.as_bytes(),
            MagpiePeriod::Month,
            None,
            NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            FixedOffset::east_opt(8 * 60 * 60).unwrap(),
            &rules,
        )
        .unwrap();
        assert_eq!(usage.events, 5);
        assert_eq!(usage.coverage_start.as_deref(), Some("2026-10-01"));
        let priced = usage
            .models
            .iter()
            .find(|model| model.model == "priced")
            .unwrap();
        assert_eq!(priced.priced_events, 1);
        assert_eq!(priced.unpriced_events, 1);
        assert_eq!(priced.computed_api_equivalent_usd, Some(2.0));
        let priced_day = usage
            .daily
            .iter()
            .find(|day| day.date == "2026-10-01")
            .unwrap();
        assert_eq!(priced_day.computed_api_equivalent_usd, Some(2.0));
        assert_eq!(priced_day.priced_events, 1);
        assert_eq!(priced_day.unpriced_events, 3);
        let unknown_day = usage
            .daily
            .iter()
            .find(|day| day.date == "2026-10-02")
            .unwrap();
        assert_eq!(unknown_day.computed_api_equivalent_usd, None);
        assert_eq!(unknown_day.priced_events, 0);
        assert_eq!(unknown_day.unpriced_events, 1);
        let unknown = usage
            .models
            .iter()
            .find(|model| model.model == "unknown")
            .unwrap();
        assert_eq!(unknown.computed_api_equivalent_usd, None);
        assert_eq!(unknown.unpriced_events, 2);
        let zero_unknown = usage
            .models
            .iter()
            .find(|model| model.model == "zero-unknown")
            .unwrap();
        assert_eq!(zero_unknown.priced_events, 0);
        assert_eq!(zero_unknown.unpriced_events, 1);
    }
}
