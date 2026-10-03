use rusqlite::{params, Connection};

#[derive(Debug)]
pub enum SessionPricingError {
    InvalidIdentity(&'static str),
    InvalidModel,
    SessionNotFound,
    Database(String),
}

impl SessionPricingError {
    pub fn is_bad_request(&self) -> bool {
        matches!(self, Self::InvalidIdentity(_) | Self::InvalidModel)
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::SessionNotFound)
    }

    pub fn message(&self) -> String {
        match self {
            Self::InvalidIdentity(message) => (*message).to_string(),
            Self::InvalidModel => "不支援的定價模型".to_string(),
            Self::SessionNotFound => "找不到符合來源識別的工作階段".to_string(),
            Self::Database(message) => message.clone(),
        }
    }
}

pub fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS session_pricing_assignments (
            assistant_type TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_dir_key_is_null INTEGER NOT NULL CHECK (source_dir_key_is_null IN (0, 1)),
            source_dir_key TEXT NOT NULL,
            session_id TEXT NOT NULL,
            pricing_model TEXT NOT NULL,
            PRIMARY KEY (
                assistant_type,
                source_kind,
                source_dir_key_is_null,
                source_dir_key,
                session_id
            )
        )",
        [],
    )
    .map_err(|error| format!("建立 session_pricing_assignments 表失敗: {error}"))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_manifest_auto_session
         ON usage_entries(assistant_type, source_kind, source_dir_key, session_id)
         WHERE lower(trim(COALESCE(model, ''))) = 'manifest/auto'",
        [],
    )
    .map_err(|error| format!("建立 manifest/auto 工作階段索引失敗: {error}"))?;
    Ok(())
}

pub fn is_manifest_auto_model(model: Option<&str>) -> bool {
    model
        .map(str::trim)
        .is_some_and(|model| model.eq_ignore_ascii_case("manifest/auto"))
}

pub fn set_session_pricing_assignment(
    conn: &mut Connection,
    assistant_type: &str,
    source_kind: &str,
    source_dir_key: Option<&str>,
    session_id: &str,
    pricing_model: Option<&str>,
) -> Result<(), SessionPricingError> {
    if assistant_type.trim().is_empty()
        || assistant_type.eq_ignore_ascii_case("all")
        || source_kind.trim().is_empty()
        || source_kind.len() > 128
    {
        return Err(SessionPricingError::InvalidIdentity("助理或來源識別無效"));
    }
    if session_id.trim().is_empty() || session_id.len() > 512 {
        return Err(SessionPricingError::InvalidIdentity("工作階段識別無效"));
    }
    if source_dir_key.is_some_and(|key| key.is_empty() || key.len() > 512) {
        return Err(SessionPricingError::InvalidIdentity("來源目錄識別無效"));
    }
    if pricing_model.is_some_and(|model| !crate::pricing::is_supported_session_pricing_model(model))
    {
        return Err(SessionPricingError::InvalidModel);
    }

    let (source_dir_key_is_null, source_dir_key_value) = match source_dir_key {
        Some(key) => (0_i64, key),
        None => (1_i64, ""),
    };
    let tx = conn
        .transaction()
        .map_err(|error| SessionPricingError::Database(format!("開始定價交易失敗: {error}")))?;
    let session_exists: bool = tx
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM usage_entries
                WHERE assistant_type = ?1
                  AND source_kind = ?2
                  AND source_dir_key IS ?3
                  AND session_id = ?4
                  AND lower(trim(COALESCE(model, ''))) = 'manifest/auto'
            )",
            params![assistant_type, source_kind, source_dir_key, session_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            SessionPricingError::Database(format!("驗證工作階段來源識別失敗: {error}"))
        })?;
    if !session_exists {
        return Err(SessionPricingError::SessionNotFound);
    }

    if let Some(pricing_model) = pricing_model {
        tx.execute(
            "INSERT INTO session_pricing_assignments (
                assistant_type, source_kind, source_dir_key_is_null,
                source_dir_key, session_id, pricing_model
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT (
                assistant_type, source_kind, source_dir_key_is_null,
                source_dir_key, session_id
            ) DO UPDATE SET pricing_model = excluded.pricing_model",
            params![
                assistant_type,
                source_kind,
                source_dir_key_is_null,
                source_dir_key_value,
                session_id,
                pricing_model
            ],
        )
        .map_err(|error| SessionPricingError::Database(format!("儲存工作階段定價失敗: {error}")))?;
    } else {
        tx.execute(
            "DELETE FROM session_pricing_assignments
             WHERE assistant_type = ?1
               AND source_kind = ?2
               AND source_dir_key_is_null = ?3
               AND source_dir_key = ?4
               AND session_id = ?5",
            params![
                assistant_type,
                source_kind,
                source_dir_key_is_null,
                source_dir_key_value,
                session_id
            ],
        )
        .map_err(|error| SessionPricingError::Database(format!("清除工作階段定價失敗: {error}")))?;
    }

    tx.commit()
        .map_err(|error| SessionPricingError::Database(format!("提交工作階段定價失敗: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_omp_session(
        conn: &Connection,
        session_id: &str,
        source_kind: &str,
        source_dir_key: Option<&str>,
        date: &str,
    ) {
        conn.execute(
            "INSERT INTO usage_entries (
                assistant_type, source_kind, source_dir_key, timestamp, date,
                session_id, turn_no, model
            ) VALUES ('omp', ?1, ?2, ?3, ?4, ?5, 1, 'manifest/auto')",
            params![
                source_kind,
                source_dir_key,
                format!("{date}T12:00:00Z"),
                date,
                session_id
            ],
        )
        .unwrap();
    }

    #[test]
    fn assignments_require_a_real_manifest_auto_session_and_valid_model() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        add_omp_session(&conn, "present", "omp-session", None, "2026-03-01");

        assert!(matches!(
            set_session_pricing_assignment(
                &mut conn,
                "omp",
                "omp-session",
                None,
                "missing",
                Some("glm-5.3")
            ),
            Err(SessionPricingError::SessionNotFound)
        ));
        assert!(matches!(
            set_session_pricing_assignment(
                &mut conn,
                "omp",
                "omp-session",
                None,
                "present",
                Some("unknown-model")
            ),
            Err(SessionPricingError::InvalidModel)
        ));
        assert!(matches!(
            set_session_pricing_assignment(
                &mut conn,
                "omp",
                "copilot-cli",
                None,
                "present",
                Some("glm-5.3")
            ),
            Err(SessionPricingError::SessionNotFound)
        ));

        let assignments: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM session_pricing_assignments",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(assignments, 0);
    }

    #[test]
    fn assignment_is_source_scoped_cleared_and_survives_usage_replacement() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        add_omp_session(&conn, "shared", "omp-session", None, "2026-03-01");
        add_omp_session(&conn, "shared", "omp-session", Some("a1b2"), "2026-03-02");

        set_session_pricing_assignment(
            &mut conn,
            "omp",
            "omp-session",
            None,
            "shared",
            Some("glm-5.3"),
        )
        .unwrap();
        set_session_pricing_assignment(
            &mut conn,
            "omp",
            "omp-session",
            Some("a1b2"),
            "shared",
            Some("deepseek-v4.1-flash"),
        )
        .unwrap();

        conn.execute(
            "DELETE FROM usage_entries WHERE assistant_type = 'omp' AND session_id = 'shared'",
            [],
        )
        .unwrap();
        add_omp_session(&conn, "shared", "omp-session", None, "2026-04-01");
        add_omp_session(&conn, "shared", "omp-session", Some("a1b2"), "2026-04-02");

        conn.execute(
            "INSERT INTO usage_entries (
                assistant_type, source_kind, source_dir_key, timestamp, date,
                session_id, turn_no, model
            ) VALUES ('omp', 'omp-session', NULL, '2026-04-03T12:00:00Z',
                      '2026-04-03', 'shared', 2, 'actual-model')",
            [],
        )
        .unwrap();
        let later_day = crate::db::get_usage_entries_by_date(&conn, "2026-04-03", "omp").unwrap();
        assert_eq!(later_day.len(), 1);
        assert_eq!(
            later_day[0].record.entry.model.as_deref(),
            Some("actual-model")
        );
        let overlay = later_day[0].record.entry.session_pricing.as_ref().unwrap();
        assert!(overlay.has_manifest_auto);
        assert_eq!(overlay.pricing_model.as_deref(), Some("glm-5.3"));
        for (date, expected_model) in [
            ("2026-04-01", "glm-5.3"),
            ("2026-04-02", "deepseek-v4.1-flash"),
        ] {
            let entries = crate::db::get_usage_entries_by_date(&conn, date, "omp").unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0]
                    .record
                    .entry
                    .session_pricing
                    .as_ref()
                    .and_then(|pricing| pricing.pricing_model.as_deref()),
                Some(expected_model)
            );
        }
        let assignments = conn
            .prepare(
                "SELECT source_dir_key_is_null, source_dir_key, pricing_model
                 FROM session_pricing_assignments ORDER BY source_dir_key_is_null DESC",
            )
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], (1, String::new(), "glm-5.3".to_string()));
        assert_eq!(
            assignments[1],
            (0, "a1b2".to_string(), "deepseek-v4.1-flash".to_string())
        );

        set_session_pricing_assignment(&mut conn, "omp", "omp-session", None, "shared", None)
            .unwrap();
        let later_day = crate::db::get_usage_entries_by_date(&conn, "2026-04-03", "omp").unwrap();
        let overlay = later_day[0].record.entry.session_pricing.as_ref().unwrap();
        assert!(overlay.has_manifest_auto);
        assert_eq!(overlay.pricing_model, None);
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM session_pricing_assignments",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 1);
    }

    #[test]
    fn session_pricing_eligibility_uses_source_scoped_index() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_db(&conn).unwrap();
        let mut statement = conn
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT EXISTS (
                    SELECT 1 FROM usage_entries AS eligible
                    WHERE eligible.assistant_type = usage_entries.assistant_type
                      AND eligible.source_kind = usage_entries.source_kind
                      AND eligible.source_dir_key IS usage_entries.source_dir_key
                      AND eligible.session_id = usage_entries.session_id
                      AND lower(trim(COALESCE(eligible.model, ''))) = 'manifest/auto'
                 ) FROM usage_entries WHERE date = '2026-03-01'",
            )
            .unwrap();
        let plan = statement
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(plan
            .iter()
            .any(|step| step.contains("idx_manifest_auto_session")));
    }
}
