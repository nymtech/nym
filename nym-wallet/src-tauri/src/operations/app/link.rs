use tauri_plugin_opener::OpenerExt;
use url::Url;

/// Validates URL for system opener: `http` and `https` only (no `file:`, `javascript:`, `tauri:`, etc.).
pub(crate) fn validate_open_url_scheme(url: &str) -> Result<Url, String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    match parsed.scheme() {
        "https" | "http" => Ok(parsed),
        other => Err(format!("URL scheme not allowed: {other}")),
    }
}

#[tauri::command]
pub async fn open_url(url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    validate_open_url_scheme(&url)?;

    match app_handle.opener().open_url(&url, None::<&str>) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to open URL: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_open_url_scheme;

    #[test]
    fn allows_http_https() {
        assert!(validate_open_url_scheme("https://nym.com/").is_ok());
        assert!(validate_open_url_scheme("http://127.0.0.1:8080/").is_ok());
    }

    #[test]
    fn rejects_other_schemes() {
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "tauri://localhost/",
            "data:text/html,hi",
            "ftp://example.com/",
        ] {
            let res = validate_open_url_scheme(url);
            assert!(res.is_err(), "expected reject: {url}");
            let msg = res.unwrap_err();
            assert!(
                msg.contains("not allowed") || msg.contains("Invalid URL"),
                "{url}: {msg}"
            );
        }
    }
}
