// Opacity backend — reads and writes global and per-app window opacity in hyprland config files.
// Backend de opacidad — lee y escribe la opacidad global y por app de las ventanas en los ficheros de configuración de hyprland.

use crate::config::hypr_dir;
use regex::Regex;
use std::fs;
use std::process::{Command, Stdio};

pub fn hyprctl_available() -> bool {
    std::process::Command::new("which")
        .arg("hyprctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn hyprdesk_opacity() -> std::path::PathBuf {
    if crate::config::is_lua_config() {
        hypr_dir().join("hyprdesk-opacity.lua")
    } else {
        hypr_dir().join("hyprdesk-opacity.conf")
    }
}

// ── File discovery / Detección de archivos ────────────────────

fn glob_conf_files() -> Result<Vec<std::path::PathBuf>, ()> {
    let dir = hypr_dir();
    let mut paths = Vec::new();
    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() { collect(&p, out); }
                else if p.extension().and_then(|e| e.to_str()) == Some("conf") {
                    out.push(p);
                }
            }
        }
    }
    collect(&dir, &mut paths);
    paths.sort_by_key(|p| p.components().count());
    Ok(paths)
}

fn glob_lua_files() -> Result<Vec<std::path::PathBuf>, ()> {
    let dir = hypr_dir();
    let mut paths = Vec::new();
    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() { collect(&p, out); }
                else if p.extension().and_then(|e| e.to_str()) == Some("lua") {
                    out.push(p);
                }
            }
        }
    }
    collect(&dir, &mut paths);
    paths.sort_by_key(|p| p.components().count());
    Ok(paths)
}

fn opacity_file() -> Option<(std::path::PathBuf, bool)> {
    if crate::config::is_lua_config() {
        let main = crate::config::hypr_lua();
        if main.exists() {
            if let Ok(s) = fs::read_to_string(&main) {
                if s.contains("active_opacity") { return Some((main, true)); }
            }
        }
        if let Ok(entries) = glob_lua_files() {
            for path in entries {
                if path == crate::config::hypr_lua() { continue; }
                if let Ok(s) = fs::read_to_string(&path) {
                    if s.contains("active_opacity") { return Some((path, true)); }
                }
            }
        }
        return main.exists().then_some((main, true));
    }

    let main = hypr_dir().join("hyprland.conf");
    if main.exists() {
        if let Ok(s) = fs::read_to_string(&main) {
            if s.contains("active_opacity") { return Some((main, false)); }
        }
    }
    if let Ok(entries) = glob_conf_files() {
        for path in entries {
            if path == hypr_dir().join("hyprland.conf") { continue; }
            if let Ok(s) = fs::read_to_string(&path) {
                if s.contains("active_opacity") { return Some((path, false)); }
            }
        }
    }
    main.exists().then_some((main, false))
}

// ── Global opacity / Opacidad global ─────────────────────────

pub struct Opacities {
    pub active: f64,
    pub inactive: f64,
}

pub fn get_opacities() -> Opacities {
    let mut result = Opacities { active: 1.0, inactive: 0.9 };
    let Some((conf, is_lua)) = opacity_file() else { return result };
    if let Ok(content) = fs::read_to_string(conf) {
        if is_lua {
            if let Ok(re) = Regex::new(r#"hl\.keyword\s*\(\s*"decoration:active_opacity"\s*,\s*"([0-9.]+)"\s*\)"#) {
                if let Some(m) = re.captures(&content) {
                    result.active = m[1].parse().unwrap_or(1.0);
                }
            }
            if let Ok(re) = Regex::new(r#"hl\.keyword\s*\(\s*"decoration:inactive_opacity"\s*,\s*"([0-9.]+)"\s*\)"#) {
                if let Some(m) = re.captures(&content) {
                    result.inactive = m[1].parse().unwrap_or(0.9);
                }
            }
        } else {
            if let Ok(re) = Regex::new(r"active_opacity\s*=\s*([0-9.]+)") {
                if let Some(m) = re.captures(&content) {
                    result.active = m[1].parse().unwrap_or(1.0);
                }
            }
            if let Ok(re) = Regex::new(r"inactive_opacity\s*=\s*([0-9.]+)") {
                if let Some(m) = re.captures(&content) {
                    result.inactive = m[1].parse().unwrap_or(0.9);
                }
            }
        }
    }
    result
}

pub fn set_opacity(key: &str, value: f64) -> bool {
    if key != "active" && key != "inactive" { return false; }
    let value = (value * 100.0).round() / 100.0;
    let Some((conf, is_lua)) = opacity_file() else { return false };
    let content = fs::read_to_string(&conf).unwrap_or_default();

    let new_content = if is_lua {
        let lua_key = format!("decoration:{key}_opacity");
        let pat_str = format!(r#"(hl\.keyword\s*\(\s*"{lua_key}"\s*,\s*")[0-9.]+("\s*\))"#);
        if let Ok(re) = Regex::new(&pat_str) {
            if re.is_match(&content) {
                re.replace(&content, format!("${{1}}{value:.2}${{2}}")).to_string()
            } else {
                format!("{}\nhl.keyword(\"{lua_key}\", \"{value:.2}\")\n", content.trim_end())
            }
        } else { return false }
    } else {
        let pat_str = format!(r"({}_opacity\s*=\s*)[0-9.]+", key);
        if let Ok(re) = Regex::new(&pat_str) {
            if re.is_match(&content) {
                re.replace(&content, format!("${{1}}{value:.2}")).to_string()
            } else {
                format!("{}\ndecoration {{\n  {key}_opacity = {value:.2}\n}}\n", content.trim_end())
            }
        } else { return false }
    };

    if fs::write(&conf, new_content).is_err() { return false; }
    let _ = Command::new("hyprctl")
        .args(["keyword", &format!("decoration:{key}_opacity"), &format!("{value:.2}")])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    true
}

// ── Per-app opacity / Opacidad por app ───────────────────────

#[derive(Clone, Debug)]
pub struct AppOpacity {
    pub app: String,
    pub active: f64,
}

fn ensure_hyprdesk_opacity_file() {
    let path = hyprdesk_opacity();
    let is_lua = crate::config::is_lua_config();
    if !path.exists() {
        let _ = fs::write(&path, "");
    }
    let main = if is_lua { crate::config::hypr_lua() } else { hypr_dir().join("hyprland.conf") };
    if !main.exists() { return; }
    if let Ok(content) = fs::read_to_string(&main) {
        if !content.contains("hyprdesk-opacity") {
            let line = if is_lua {
                format!("\n-- HyprDesk per-app opacity overrides\ndofile(\"{}\")\n", path.display())
            } else {
                format!("\n# HyprDesk per-app opacity overrides\nsource = {}\n", path.display())
            };
            let _ = fs::write(&main, content + &line);
        }
    }
}

pub fn get_open_window_classes() -> Vec<String> {
    let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() else {
        return Vec::new();
    };
    let s = String::from_utf8_lossy(&out.stdout);
    let Ok(arr) = serde_json::from_str::<serde_json::Value>(&s) else {
        return Vec::new();
    };
    let mut classes: Vec<String> = arr.as_array()
        .map(|a| a.iter()
            .filter_map(|v| v["class"].as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect())
        .unwrap_or_default();
    classes.sort();
    classes.dedup();
    classes
}

fn parse_app_rules_conf(content: &str) -> Vec<AppOpacity> {
    let re = Regex::new(
        r"(?m)^[ \t]*(?:windowrule\s*=\s*match:class\s+\^\(([^)]+)\)\$[ \t]*,[ \t]*opacity\s+([\d.]+)|windowrule(?:v2)?\s*=\s*opacity\s+([\d.]+)(?:[ \t]+[\d.]+)?[ \t]*,[ \t]*class:\^\(([^)]+)\)\$)"
    ).unwrap();
    let mut seen = std::collections::HashMap::new();
    for cap in re.captures_iter(content) {
        if let (Some(app), Some(active)) = (cap.get(1), cap.get(2)) {
            let a: f64 = active.as_str().parse().unwrap_or(1.0);
            seen.insert(app.as_str().to_string(), (a * 100.0).round() / 100.0);
        } else if let (Some(active), Some(app)) = (cap.get(3), cap.get(4)) {
            let a: f64 = active.as_str().parse().unwrap_or(1.0);
            seen.insert(app.as_str().to_string(), (a * 100.0).round() / 100.0);
        }
    }
    seen.into_iter().map(|(app, active)| AppOpacity { app, active }).collect()
}

fn parse_app_rules_lua(content: &str) -> Vec<AppOpacity> {
    let re = Regex::new(
        r#"(?m)^[ \t]*hl\.keyword\s*\(\s*"windowrule"\s*,\s*"match:class \^\(([^)]+)\)\$\s*,\s*opacity\s+([\d.]+)"\s*\)"#
    ).unwrap();
    let mut seen = std::collections::HashMap::new();
    for cap in re.captures_iter(content) {
        let app = cap[1].to_string();
        let a: f64 = cap[2].parse().unwrap_or(1.0);
        seen.insert(app, (a * 100.0).round() / 100.0);
    }
    seen.into_iter().map(|(app, active)| AppOpacity { app, active }).collect()
}

pub fn get_app_opacities() -> Vec<AppOpacity> {
    let path = hyprdesk_opacity();
    if !path.exists() { return Vec::new(); }
    let content = fs::read_to_string(&path).unwrap_or_default();
    if crate::config::is_lua_config() {
        parse_app_rules_lua(&content)
    } else {
        parse_app_rules_conf(&content)
    }
}

pub fn set_app_opacity(app_class: &str, active: f64) -> bool {
    let active = (active * 100.0).round() / 100.0;
    ensure_hyprdesk_opacity_file();
    let conf = hyprdesk_opacity();
    let content = if conf.exists() { fs::read_to_string(&conf).unwrap_or_default() } else { String::new() };
    let escaped = regex::escape(app_class);

    let new_content = if crate::config::is_lua_config() {
        let pat_str = format!(
            r#"(?m)^[ \t]*hl\.keyword\s*\(\s*"windowrule"\s*,\s*"match:class \^\({escaped}\)\$\s*,\s*opacity\s+[\d.]+"\s*\)\s*$"#
        );
        let new_rule = format!(r#"hl.keyword("windowrule", "match:class ^({app_class})$, opacity {active:.2}")"#);
        if let Ok(re) = Regex::new(&pat_str) {
            if re.is_match(&content) {
                let mut first = true;
                let result = re.replace_all(&content, |_: &regex::Captures| {
                    if first { first = false; new_rule.clone() } else { String::new() }
                }).to_string();
                Regex::new(r"\n{3,}").unwrap().replace_all(&result, "\n\n").to_string()
            } else {
                format!("{}\n{new_rule}\n", content.trim_end())
            }
        } else { return false }
    } else {
        let pat_str = format!(
            r"(?m)^[ \t]*(?:windowrule\s*=\s*match:class\s+\^\({escaped}\)\$[ \t]*,[ \t]*opacity\s+[\d.]+(?:[ \t]+[\d.]+)?|windowrule(?:v2)?\s*=\s*opacity\s+[\d.]+(?:[ \t]+[\d.]+)?[ \t]*,[ \t]*class:\^\({escaped}\)\$)[ \t]*(?:#.*)?$"
        );
        let new_rule = format!("windowrule = match:class ^({app_class})$, opacity {active:.2}");
        if let Ok(re) = Regex::new(&pat_str) {
            if re.is_match(&content) {
                let mut first = true;
                let result = re.replace_all(&content, |_: &regex::Captures| {
                    if first { first = false; new_rule.clone() } else { String::new() }
                }).to_string();
                Regex::new(r"\n{3,}").unwrap().replace_all(&result, "\n\n").to_string()
            } else {
                format!("{}\n{new_rule}\n", content.trim_end())
            }
        } else { return false }
    };

    if fs::write(&conf, new_content).is_err() { return false; }
    let _ = Command::new("hyprctl").args(["reload", "config-only"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    true
}

pub fn remove_app_opacity(app_class: &str) -> bool {
    let conf = hyprdesk_opacity();
    if !conf.exists() { return false; }
    let content = fs::read_to_string(&conf).unwrap_or_default();
    let escaped = regex::escape(app_class);

    let pat_str = if crate::config::is_lua_config() {
        format!(
            r#"(?m)^[ \t]*hl\.keyword\s*\(\s*"windowrule"\s*,\s*"match:class \^\({escaped}\)\$\s*,\s*opacity\s+[\d.]+"\s*\)\s*\n?"#
        )
    } else {
        format!(
            r"(?m)^[ \t]*#?[ \t]*(?:windowrule\s*=\s*match:class\s+\^\({escaped}\)\$[ \t]*,[ \t]*opacity\s+[\d.]+(?:[ \t]+[\d.]+)?|windowrule(?:v2)?\s*=\s*opacity\s+[\d.]+(?:[ \t]+[\d.]+)?[ \t]*,[ \t]*class:\^\({escaped}\)\$)[ \t]*(?:#.*)?$\n?"
        )
    };

    let Ok(re) = Regex::new(&pat_str) else { return false };
    let new_content = re.replace_all(&content, "").to_string();
    if new_content == content { return false; }
    if fs::write(&conf, new_content).is_err() { return false; }
    let _ = Command::new("hyprctl").args(["reload", "config-only"])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    true
}
