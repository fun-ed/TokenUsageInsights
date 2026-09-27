use std::path::{Path, PathBuf};

/// Resolve a configured path without requiring it to exist yet.
///
/// Besides native absolute/relative paths, this accepts the common `~` and
/// `$HOME` prefixes so values copied from shell configuration work.
pub fn env_path(name: &str) -> Option<PathBuf> {
    let value = std::env::var_os(name)?;
    if value.is_empty() {
        return None;
    }

    Some(expand_common_prefix(PathBuf::from(value)))
}

fn expand_common_prefix(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    let home = dirs::home_dir();

    if raw == "~" || raw.eq_ignore_ascii_case("$HOME") {
        return home.unwrap_or(path);
    }

    for prefix in ["~/", "$HOME/"] {
        if let Some(rest) = raw.strip_prefix(prefix) {
            if let Some(home) = &home {
                return home.join(rest);
            }
        }
    }

    path
}

/// Locate a release resource independently of the process working directory.
pub fn find_resource(relative: impl AsRef<Path>) -> Option<PathBuf> {
    let relative = relative.as_ref();
    if relative.is_absolute() && relative.exists() {
        return Some(relative.to_path_buf());
    }

    let mut roots = Vec::new();
    if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        roots.push(PathBuf::from(manifest_dir));
    }
    if let Ok(executable) = std::env::current_exe() {
        // ~/.local/bin 內是指向安裝目錄的 symlink；先解析 symlink 才能找到
        // 與真實執行檔同層的資源目錄。
        let resolved = std::fs::canonicalize(&executable).unwrap_or_else(|_| executable.clone());
        let mut exes = vec![&resolved];
        if resolved != executable {
            exes.push(&executable);
        }
        for exe in exes {
            if let Some(executable_dir) = exe.parent() {
                roots.extend(executable_dir.ancestors().take(5).map(Path::to_path_buf));
            }
        }
    }
    // 安裝版先使用執行檔旁的資源，避免誤讀仍存在的編譯來源目錄。
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        roots.push(PathBuf::from(manifest_dir));
    }
    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir);
    }

    roots
        .into_iter()
        .map(|root| root.join(relative))
        .find(|candidate| candidate.exists())
}

/// Resolve the Copilot App (Tauri desktop) data directory.
///
/// Honors `COPILOT_APP_DIR` first, then falls back to `COPILOT_DIR`, then to
/// `~/.copilot`. The directory is expected to contain `data.db` and
/// `session-store.db` written by the Copilot App.
pub fn copilot_app_dir() -> PathBuf {
    if let Some(path) = env_path("COPILOT_APP_DIR") {
        return path;
    }
    if let Some(path) = env_path("COPILOT_DIR") {
        return path;
    }
    dirs::home_dir()
        .map(|h| h.join(".copilot"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_prefixes_expand_and_absolute_paths_are_preserved() {
        let home = dirs::home_dir().expect("home dir");

        assert_eq!(expand_common_prefix(PathBuf::from("~")), home);
        assert_eq!(
            expand_common_prefix(PathBuf::from("~/a/b")),
            home.join("a/b")
        );
        assert_eq!(
            expand_common_prefix(PathBuf::from("$HOME/a")),
            home.join("a")
        );
        assert_eq!(
            expand_common_prefix(PathBuf::from("/var/data")),
            PathBuf::from("/var/data")
        );
    }
}
