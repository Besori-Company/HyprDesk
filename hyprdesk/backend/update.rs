// Update check — queries GitHub releases API and compares with the current version.
// Comprobación de actualizaciones — consulta la API de releases de GitHub y compara con la versión actual.

use std::time::Duration;

pub async fn check_latest_version() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("HyprDesk")
        .build()
        .ok()?;
    let json: serde_json::Value = client
        .get("https://api.github.com/repos/Besori-Company/HyprDesk/releases/latest")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let tag = json["tag_name"].as_str()?.trim_start_matches('v').to_string();
    if is_newer(&tag) { Some(tag) } else { None }
}

fn is_newer(latest: &str) -> bool {
    parse_ver(latest) > parse_ver(env!("CARGO_PKG_VERSION"))
}

fn parse_ver(v: &str) -> (u32, u32, u32) {
    let mut it = v.split('.').filter_map(|x| x.parse().ok());
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

pub fn open_releases_page() {
    let _ = std::process::Command::new("xdg-open")
        .arg("https://github.com/Besori-Company/HyprDesk/releases")
        .spawn();
}
