use chrono::Local;
use reqwest::{StatusCode, Url};
use std::time::Duration;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(600);
const FALLBACK_COMPUTER_NAME: &str = "unknown-host";
pub(crate) const MAX_DOWNLOAD_BYTES: usize = 200_000_000;

pub(crate) fn is_url(value: &str) -> bool {
    let value = value.trim_start();
    ["http://", "https://"].iter().any(|prefix| {
        value
            .get(..prefix.len())
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case(prefix))
    })
}

pub(crate) fn export_file_name() -> String {
    format!(
        "{}_{}.json",
        Local::now().format("%Y-%m-%d_%H-%M-%S"),
        computer_name()
    )
}

fn computer_name() -> String {
    os_hostname()
        .or_else(|| {
            std::env::var("HOSTNAME")
                .ok()
                .filter(|name| !name.trim().is_empty())
        })
        .map(|name| sanitize_computer_name(&name))
        .unwrap_or_else(|| FALLBACK_COMPUTER_NAME.to_string())
}

#[cfg(unix)]
fn os_hostname() -> Option<String> {
    let mut buf = [0u8; 256];
    // SAFETY: the supplied length is the actual buffer size.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|byte| *byte == 0).unwrap_or(buf.len());
    let name = String::from_utf8_lossy(&buf[..end]).trim().to_string();
    (!name.is_empty()).then_some(name)
}

fn sanitize_computer_name(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        FALLBACK_COMPUTER_NAME.to_string()
    } else {
        cleaned
    }
}

pub(crate) fn build_blob_url(sas_url: &str, file_name: &str) -> Result<Url, String> {
    let mut url = Url::parse(sas_url).map_err(|_| "無效的 Azure SAS URL".to_string())?;
    validate_https_url(&url, "SAS URL")?;
    if !url
        .query_pairs()
        .any(|(key, value)| key == "sig" && !value.is_empty())
    {
        return Err("不是有效的 SAS URL（缺少 sig 參數）".to_string());
    }

    let segments: Vec<_> = url
        .path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect())
        .unwrap_or_default();
    if segments.is_empty() {
        return Err("SAS URL 必須包含容器名稱".to_string());
    }
    let is_blob_sas = url
        .query_pairs()
        .any(|(key, value)| key == "sr" && value == "b");
    if is_blob_sas {
        if segments.len() < 2 {
            return Err("Blob SAS URL 必須包含 Blob 名稱".to_string());
        }
        return Ok(url);
    }

    url.path_segments_mut()
        .map_err(|_| "SAS URL 無法附加檔名".to_string())?
        .pop_if_empty()
        .push(file_name);
    Ok(url)
}

fn validate_https_url(url: &Url, label: &str) -> Result<(), String> {
    if url.scheme() != "https" {
        return Err(format!("{label} 必須使用 HTTPS"));
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(format!("{label} 格式不安全（不可包含帳密或 fragment）"));
    }
    Ok(())
}

fn display_url(url: &Url) -> String {
    let mut shown = url.clone();
    shown.set_query(None);
    shown.set_fragment(None);
    shown.to_string()
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(NETWORK_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("建立 HTTP 用戶端失敗: {}", error.without_url()))
}

pub(crate) async fn upload_json(url: &Url, body: String) -> Result<(), String> {
    validate_https_url(url, "SAS URL")?;
    let response = http_client()?
        .put(url.clone())
        .header("x-ms-blob-type", "BlockBlob")
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )
        .body(body)
        .send()
        .await
        .map_err(|error| format!("上傳失敗: {}", error.without_url()))?;
    let status = response.status();
    if status == StatusCode::CREATED {
        Ok(())
    } else {
        Err(format!("Azure Blob 回應 HTTP {status}"))
    }
}

pub(crate) fn export_success_message(count: usize, url: &Url) -> String {
    format!(
        "已上傳 {count} 筆至 Azure Blob：{}\n在另一台電腦匯入時，請透過安全管道傳送完整 SAS URL，並執行：\ntoken-usage-insights import --url '<完整 SAS Blob URL>'",
        display_url(url)
    )
}

pub(crate) fn sas_without_read_permission(url: &Url) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == "sp")
        .map(|(_, permissions)| permissions.into_owned())
        .filter(|permissions| !permissions.contains('r'))
}

pub(crate) fn parse_download_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "無效的匯入網址".to_string())?;
    validate_https_url(&url, "網址")?;

    let is_container_sas = url
        .query_pairs()
        .any(|(key, value)| key == "sr" && value == "c");
    let segment_count = url
        .path_segments()
        .map_or(0, |parts| parts.filter(|part| !part.is_empty()).count());
    if is_container_sas && segment_count < 2 {
        return Err("這是容器網址，請使用包含檔名的完整 Blob 網址".to_string());
    }
    if blob_file_name(&url).is_none() || url.path().ends_with('/') {
        return Err("網址必須指向 JSON 檔案".to_string());
    }
    Ok(url)
}

pub(crate) fn blob_file_name(url: &Url) -> Option<String> {
    let last = url.path_segments()?.rfind(|segment| !segment.is_empty())?;
    Some(percent_decode(last))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = |offset: usize| {
            bytes
                .get(index + offset)
                .and_then(|byte| (*byte as char).to_digit(16))
        };
        match (bytes[index], hex(1), hex(2)) {
            (b'%', Some(high), Some(low)) => {
                decoded.push((high * 16 + low) as u8);
                index += 3;
            }
            (byte, _, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn exceeds_download_limit(size: u64, max_bytes: usize) -> bool {
    size > max_bytes as u64
}

fn download_size_error(max_bytes: usize) -> String {
    format!("匯入檔案超過大小上限（{} MB）", max_bytes / 1_000_000)
}

pub(crate) async fn download_json(url: &Url, max_bytes: usize) -> Result<String, String> {
    validate_https_url(url, "網址")?;
    let mut response = http_client()?
        .get(url.clone())
        .send()
        .await
        .map_err(|error| format!("下載失敗: {}", error.without_url()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("下載失敗：HTTP {status}"));
    }
    if response
        .content_length()
        .is_some_and(|length| exceeds_download_limit(length, max_bytes))
    {
        return Err(download_size_error(max_bytes));
    }

    let mut data = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("下載失敗: {}", error.without_url()))?
    {
        let next_size = data.len().saturating_add(chunk.len());
        if exceeds_download_limit(u64::try_from(next_size).unwrap_or(u64::MAX), max_bytes) {
            return Err(download_size_error(max_bytes));
        }
        data.extend_from_slice(&chunk);
    }
    String::from_utf8(data).map_err(|_| "下載內容不是有效的 UTF-8 文字".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAS: &str = "sv=2025-07-05&sr=c&sp=racwl&sig=secret%3D";

    #[test]
    fn url_detection_and_container_upload_keep_credentials_private() {
        assert!(is_url("https://account.blob.core.windows.net/data"));
        assert!(is_url("HTTP://example.test/data"));
        assert!(is_url(" https://example.test/data?sig=secret"));
        assert!(!is_url("data.json"));

        let url = build_blob_url(
            &format!("https://account.blob.core.windows.net/data?{SAS}"),
            "2026-10-31_host.json",
        )
        .unwrap();
        assert_eq!(url.path(), "/data/2026-10-31_host.json");
        assert!(url.query().unwrap().contains("sig=secret%3D"));
        let message = export_success_message(3, &url);
        assert!(!message.contains("secret"));
        assert!(message.contains("<完整 SAS Blob URL>"));
    }

    #[test]
    fn sas_permission_warning_only_reports_permissions() {
        let writable = Url::parse("https://example.test/container?sp=wc&sig=secret").unwrap();
        let readable = Url::parse("https://example.test/container?sp=rc&sig=secret").unwrap();
        assert_eq!(
            sas_without_read_permission(&writable).as_deref(),
            Some("wc")
        );
        assert_eq!(sas_without_read_permission(&readable), None);
    }

    #[test]
    fn blob_sas_uses_existing_blob_and_rejects_unsafe_upload_urls() {
        let url = build_blob_url(
            "https://account.blob.core.windows.net/data/file.json?sr=b&sig=secret",
            "ignored.json",
        )
        .unwrap();
        assert_eq!(url.path(), "/data/file.json");

        for invalid in [
            "http://account.blob.core.windows.net/data?sig=secret",
            "https://account.blob.core.windows.net/data",
            "https://account.blob.core.windows.net/?sig=secret",
            "https://user:pass@account.blob.core.windows.net/data?sig=secret",
            "https://account.blob.core.windows.net/data?sig=secret#fragment",
        ] {
            let error = build_blob_url(invalid, "backup.json").unwrap_err();
            assert!(!error.contains("secret"));
        }
    }

    #[test]
    fn download_url_requires_https_and_a_file_path() {
        assert!(parse_download_url("https://example.test/usage.json?sig=secret").is_ok());
        assert!(parse_download_url("http://example.test/usage.json?sig=secret").is_err());
        assert!(parse_download_url("https://user:pass@example.test/usage.json").is_err());
        assert!(parse_download_url("https://example.test/usage.json?sig=secret#fragment").is_err());
        assert!(parse_download_url("https://example.test/container/?sr=c&sig=secret").is_err());
        assert!(parse_download_url("https://example.test/container?sr=c&sig=secret").is_err());
        let file_url = Url::parse("https://example.test/usage%20file.json?sig=secret").unwrap();
        assert_eq!(
            blob_file_name(&file_url).as_deref(),
            Some("usage file.json")
        );
    }

    #[test]
    fn download_limit_is_enforced_at_the_boundary() {
        assert!(!exceeds_download_limit(10, 10));
        assert!(exceeds_download_limit(11, 10));
        assert!(exceeds_download_limit(u64::MAX, 10));
        assert!(download_size_error(200_000_000).contains("200 MB"));
    }

    #[tokio::test]
    async fn signed_http_client_does_not_follow_redirects() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let target_url = format!(
            "http://{}/stolen?sig=private-token",
            target.local_addr().unwrap()
        );

        let source = TcpListener::bind("127.0.0.1:0").unwrap();
        let source_url = format!(
            "http://{}/upload?sig=private-token",
            source.local_addr().unwrap()
        );
        let server = thread::spawn(move || {
            let (mut stream, _) = source.accept().unwrap();
            let mut request = [0; 1024];
            assert!(stream.read(&mut request).unwrap() > 0);
            write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });

        let response = http_client().unwrap().get(source_url).send().await.unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        server.join().unwrap();
        assert!(matches!(
            target.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }
}
