use super::*;
use std::hash::{Hash, Hasher};

fn source_fingerprint(database: &Path) -> Result<u64, String> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for path in [database.to_path_buf(), database.with_extension("db-wal")] {
        match fs::metadata(path) {
            Ok(metadata) => {
                metadata.len().hash(&mut hasher);
                metadata
                    .modified()
                    .map_err(|error| error.to_string())?
                    .hash(&mut hasher);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0_u64.hash(&mut hasher),
            Err(error) => return Err(format!("無法讀取 Devin 來源狀態: {error}")),
        }
    }
    Ok(hasher.finish() & i64::MAX as u64)
}

pub(super) fn sync_usage_logs(
    conn: &mut Connection,
    root: &Path,
    should_cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    let root = crate::config::absolute_root(root);
    let source_key = additional_source_key(&root);
    let database = root.join("sessions.db");
    if database.exists() && database.canonicalize().ok().as_ref() != Some(&database) {
        return Err("Devin sessions.db 不可透過符號連結讀取其他來源。".to_string());
    }
    let state_prefix = format!("devin:source:{source_key}");
    let states = load_sync_state_for_prefix(conn, &state_prefix);
    let snapshot_key = format!("{state_prefix}:database");
    let snapshot = source_fingerprint(&database)?;
    if states
        .get(&snapshot_key)
        .is_some_and(|state| state.0 == snapshot)
    {
        return Ok(());
    }
    let sessions = crate::devin::read_sessions(&database, should_cancel)?;
    for session in sessions {
        if should_cancel() {
            return Ok(());
        }
        if session.entries.is_empty() {
            continue;
        }
        let state_key = format!("{state_prefix}:session:{}", session.session_id);
        let encoded = serde_json::to_vec(&session.entries)
            .map_err(|error| format!("無法建立 Devin 用量指紋: {error}"))?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        encoded.hash(&mut hasher);
        let fingerprint = hasher.finish() & i64::MAX as u64;
        if states
            .get(&state_key)
            .is_some_and(|state| state.0 == fingerprint)
        {
            continue;
        }
        let tx = conn
            .transaction()
            .map_err(|error| format!("無法開始 Devin 交易: {error}"))?;
        tx.execute(
            "DELETE FROM usage_entries WHERE assistant_type = 'devin' AND source_kind = ?1
             AND source_dir_key = ?2 AND session_id = ?3 AND import_source_id IS NULL",
            params![crate::devin::SOURCE_KIND, source_key, session.session_id],
        )
        .map_err(|error| format!("無法替換 Devin 用量: {error}"))?;
        insert_usage_entries(
            &tx,
            "devin",
            "Devin CLI",
            crate::devin::SOURCE_KIND,
            Some(&source_key),
            &session.entries,
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO sync_state (filename, last_synced_size, last_synced_time)
             VALUES (?1, ?2, 0)",
            params![state_key, fingerprint],
        )
        .map_err(|error| format!("無法更新 Devin 同步狀態: {error}"))?;
        tx.commit()
            .map_err(|error| format!("無法提交 Devin 交易: {error}"))?;
        bump_sync_data_revision();
    }
    if database.is_file() && !should_cancel() && source_fingerprint(&database)? == snapshot {
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (filename, last_synced_size, last_synced_time) VALUES (?1, ?2, 0)",
            params![snapshot_key, snapshot],
        ).map_err(|error| format!("無法儲存 Devin 來源狀態: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(root: &Path, input: u64) {
        fs::create_dir_all(root).unwrap();
        let source = Connection::open(root.join("sessions.db")).unwrap();
        source.execute_batch("CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, working_directory TEXT, model TEXT, title TEXT, created_at INTEGER);
            CREATE TABLE IF NOT EXISTS message_nodes (row_id INTEGER PRIMARY KEY, session_id TEXT, node_id INTEGER, chat_message TEXT, created_at INTEGER);
            DELETE FROM sessions; DELETE FROM message_nodes;
            INSERT INTO sessions VALUES ('shared-session', '/tmp/project', 'swe-2-high', 'Synthetic Devin session', 1794312000);").unwrap();
        let message = serde_json::json!({"message_id":"reply-1","role":"assistant","content":"Synthetic reply","metadata":{"request_id":"call-1","generation_model":"swe-2-high","created_at":"2026-11-10T12:00:00Z","metrics":{"input_tokens":input,"output_tokens":20,"cache_read_tokens":30}}});
        source.execute("INSERT INTO message_nodes (session_id, node_id, chat_message, created_at) VALUES ('shared-session', 1, ?1, 1794312000)", params![message.to_string()]).unwrap();
    }

    #[test]
    fn devin_sync_detects_wal_updates() {
        let root = std::env::temp_dir().join(format!(
            "insights-devin-wal-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fixture(&root, 100);
        let database = root.join("sessions.db");
        let writer = Connection::open(&database).unwrap();
        writer
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
            .unwrap();
        let mut destination = Connection::open_in_memory().unwrap();
        init_db(&destination).unwrap();
        sync_usage_logs(&mut destination, &root, &|| false).unwrap();
        let before = source_fingerprint(&database).unwrap();
        writer.execute("UPDATE message_nodes SET chat_message = json_set(chat_message, '$.metadata.metrics.input_tokens', 200)", []).unwrap();
        assert_ne!(source_fingerprint(&database).unwrap(), before);
        sync_usage_logs(&mut destination, &root, &|| false).unwrap();
        let input: u64 = destination
            .query_row(
                "SELECT SUM(tokens_input) FROM usage_entries WHERE assistant_type = 'devin'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(input, 200);
        drop(writer);
        assert_eq!(
            crate::devin::read_sessions(&database, &|| false)
                .unwrap()
                .len(),
            1
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn devin_sync_is_idempotent_isolates_sources_and_preserves_rows_on_read_failure() {
        let root = std::env::temp_dir().join(format!(
            "insights-devin-sync-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = root.join("first");
        let second = root.join("second");
        fixture(&first, 100);
        fixture(&second, 200);
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        sync_usage_logs(&mut conn, &first, &|| false).unwrap();
        let count = |conn: &Connection| {
            conn.query_row("SELECT COUNT(*), SUM(tokens_input) FROM usage_entries WHERE assistant_type = 'devin'", [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).unwrap()
        };
        assert_eq!(count(&conn), (1, 100));
        let changes: i64 = conn
            .query_row("SELECT total_changes()", [], |row| row.get(0))
            .unwrap();
        sync_usage_logs(&mut conn, &first, &|| false).unwrap();
        assert_eq!(
            conn.query_row("SELECT total_changes()", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            changes
        );
        sync_usage_logs(&mut conn, &second, &|| false).unwrap();
        assert_eq!(count(&conn), (2, 300));
        fixture(&first, 150);
        sync_usage_logs(&mut conn, &first, &|| false).unwrap();
        assert_eq!(count(&conn), (2, 350));
        fs::write(first.join("sessions.db"), "invalid database").unwrap();
        assert!(sync_usage_logs(&mut conn, &first, &|| false).is_err());
        assert_eq!(count(&conn), (2, 350));
        sync_usage_logs(&mut conn, &root.join("offline"), &|| false).unwrap();
        assert_eq!(count(&conn), (2, 350));
        fs::remove_dir_all(root).unwrap();
    }
}
