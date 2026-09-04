// Monitor backend — reads monitor info via hyprctl and writes configuration to hyprland.conf.
// Backend de monitores — lee información de monitores con hyprctl y escribe la configuración en hyprland.conf.

use crate::config::hypr_dir;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};

pub const TRANSFORMS_EN: &[&str] = &[
    "Normal", "90°", "180°", "270°", "Flipped", "Flipped 90°", "Flipped 180°", "Flipped 270°",
];
pub const TRANSFORMS_ES: &[&str] = &[
    "Normal", "90°", "180°", "270°", "Espejado", "Espejado 90°", "Espejado 180°", "Espejado 270°",
];
pub const SCALE_PRESETS: &[f64] = &[0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub name: String,
    // Part of the hyprctl JSON schema, not shown in the UI / Parte del esquema JSON de hyprctl, no se muestra en la interfaz
    #[allow(dead_code)]
    pub description: Option<String>,
    pub model: Option<String>,
    pub width: i32,
    pub height: i32,
    pub refresh_rate: f64,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: i32,
    #[serde(default)]
    pub available_modes: Vec<String>,
}

impl Monitor {
    pub fn eff_size(&self) -> (i32, i32) {
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        let w = (self.width as f64 / scale).round() as i32;
        let h = (self.height as f64 / scale).round() as i32;
        if self.transform == 1 || self.transform == 3 || self.transform == 5 || self.transform == 7 {
            (h, w)
        } else {
            (w, h)
        }
    }
}

pub fn overlapping(
    monitors: &[Monitor],
    positions: &HashMap<String, (i32, i32)>,
    name: &str,
    x: i32,
    y: i32,
) -> Vec<String> {
    let Some((mw, mh)) = monitors.iter().find(|m| m.name == name).map(|m| m.eff_size()) else {
        return Vec::new();
    };
    monitors
        .iter()
        .filter(|other| other.name != name)
        .filter(|other| {
            let (ox, oy) = positions.get(&other.name).copied().unwrap_or((other.x, other.y));
            let (ow, oh) = other.eff_size();
            !(x + mw <= ox || x >= ox + ow || y + mh <= oy || y >= oy + oh)
        })
        .map(|other| other.name.clone())
        .collect()
}

pub fn get_monitors() -> Vec<Monitor> {
    let Ok(out) = Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return Vec::new();
    };
    let s = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&s).unwrap_or_default()
}

pub fn mode_label(mode: &str) -> String {
    let parts: Vec<&str> = mode.split('@').collect();
    if parts.len() == 2 {
        let res = parts[0].replace('x', "×");
        let hz_str = parts[1].replace("Hz", "").trim().to_string();
        if let Ok(hz) = hz_str.parse::<f64>() {
            let hz_disp = if hz == hz.floor() { format!("{}", hz as i64) } else { format!("{hz:.2}") };
            return format!("{res} @ {hz_disp} Hz");
        }
    }
    mode.to_string()
}

// Pulls the resolution out of a mode string like 1920x1080@60.00Hz / Saca la resolución de una cadena de modo tipo 1920x1080@60.00Hz
pub fn mode_size(mode: &str) -> Option<(i32, i32)> {
    let (res, _) = mode.split_once('@')?;
    let (w, h) = res.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

pub fn current_mode(mon: &Monitor) -> String {
    format!("{}x{}@{:.2}Hz", mon.width, mon.height, mon.refresh_rate)
}

pub fn closest_scale_idx(value: f64) -> u32 {
    SCALE_PRESETS
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| ((*a - value).abs()).partial_cmp(&((*b - value).abs())).unwrap())
        .map(|(i, _)| i as u32)
        .unwrap_or(2)
}

fn monitor_config_file() -> (std::path::PathBuf, bool) {
    let hypr = hypr_dir();
    if crate::config::is_lua_config() {
        let candidates = [
            hypr.join("monitors.lua"),
            hypr.join("UserConfigs").join("monitors.lua"),
            hypr.join("hyprland.lua"),
        ];
        for path in &candidates {
            if path.exists() {
                if let Ok(s) = fs::read_to_string(path) {
                    if s.contains("monitor") { return (path.clone(), true); }
                }
            }
        }
        return (candidates[0].clone(), true);
    }
    let candidates = [
        hypr.join("monitors.conf"),
        hypr.join("UserConfigs").join("monitors.conf"),
        hypr.join("hyprland.conf"),
    ];
    for path in &candidates {
        if path.exists() {
            if let Ok(s) = fs::read_to_string(path) {
                if s.contains("monitor") { return (path.clone(), false); }
            }
        }
    }
    (candidates[0].clone(), false)
}

fn workspace_config_file() -> (std::path::PathBuf, bool) {
    let hypr = hypr_dir();
    if crate::config::is_lua_config() {
        let candidates = [
            hypr.join("UserConfigs").join("UserSettings.lua"),
            hypr.join("UserConfigs").join("01-UserDefaults.lua"),
            hypr.join("hyprland.lua"),
        ];
        for path in &candidates {
            if path.exists() {
                if let Ok(s) = fs::read_to_string(path) {
                    if Regex::new(r#"(?m)hl\.keyword\s*\(\s*"workspace"\s*,\s*"1\s*,"#)
                        .map(|r| r.is_match(&s)).unwrap_or(false)
                    {
                        return (path.clone(), true);
                    }
                }
            }
        }
        return (candidates[2].clone(), true);
    }
    let candidates = [
        hypr.join("UserConfigs").join("UserSettings.conf"),
        hypr.join("UserConfigs").join("01-UserDefaults.conf"),
        hypr.join("hyprland.conf"),
    ];
    for path in &candidates {
        if path.exists() {
            if let Ok(s) = fs::read_to_string(path) {
                if Regex::new(r"(?m)^workspace\s*=\s*1\s*,").map(|r| r.is_match(&s)).unwrap_or(false) {
                    return (path.clone(), false);
                }
            }
        }
    }
    (candidates[2].clone(), false)
}

pub fn set_monitor_config(name: &str, mode: &str, x: i32, y: i32, scale: f64, transform: u32) -> bool {
    let conf_val = format!("{name},{mode},{x}x{y},{scale:.2},transform,{transform}");
    let _ = Command::new("hyprctl")
        .args(["keyword", "monitor", &conf_val])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();

    let (conf_file, is_lua) = monitor_config_file();
    let content = if conf_file.exists() { fs::read_to_string(&conf_file).unwrap_or_default() } else { String::new() };
    let new = if is_lua {
        let escaped = regex::escape(name);
        let pat_str = format!(r#"(?m)^[ \t]*hl\.keyword\s*\(\s*"monitor"\s*,\s*"{}[^"]*"\s*\)\s*$"#, escaped);
        let new_line = format!(r#"hl.keyword("monitor", "{conf_val}")"#);
        if let Ok(re) = Regex::new(&pat_str) {
            let cleaned = re.replace_all(&content, "").to_string();
            let cleaned = Regex::new(r"\n{3,}").unwrap().replace_all(&cleaned, "\n\n").trim_end().to_string();
            format!("{cleaned}\n{new_line}\n")
        } else {
            format!("{content}\n{new_line}\n")
        }
    } else {
        let pat_str = format!(r"(?m)^[ \t]*monitor\s*=\s*{}[ \t]*,.*$", regex::escape(name));
        let new_line = format!("monitor={conf_val}");
        if let Ok(re) = Regex::new(&pat_str) {
            let cleaned = re.replace_all(&content, "").to_string();
            let cleaned = Regex::new(r"\n{3,}").unwrap().replace_all(&cleaned, "\n\n").trim_end().to_string();
            format!("{cleaned}\n{new_line}\n")
        } else {
            format!("{content}\n{new_line}\n")
        }
    };
    if let Some(parent) = conf_file.parent() { let _ = fs::create_dir_all(parent); }
    fs::write(&conf_file, new).is_ok()
}

pub fn set_primary_monitor(name: &str) -> bool {
    let _ = Command::new("hyprctl")
        .args(["dispatch", "moveworkspacetomonitor", &format!("1 {name}")])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    let (conf_file, is_lua) = workspace_config_file();
    let content = if conf_file.exists() { fs::read_to_string(&conf_file).unwrap_or_default() } else { String::new() };
    let new = if is_lua {
        let ws_val = format!("1, monitor:{name}, default:true");
        let new_line = format!(r#"hl.keyword("workspace", "{ws_val}")"#);
        let pat = r#"(?m)^[ \t]*hl\.keyword\s*\(\s*"workspace"\s*,\s*"1[ \t]*,.*"\s*\)\s*$"#;
        if let Ok(re) = Regex::new(pat) {
            if re.is_match(&content) {
                re.replace(&content, new_line.as_str()).to_string()
            } else {
                format!("{}\n{new_line}\n", content.trim_end())
            }
        } else { return false }
    } else {
        let new_rule = format!("workspace = 1, monitor:{name}, default:true");
        if let Ok(re) = Regex::new(r"(?m)^[ \t]*workspace\s*=\s*1\s*,.*$") {
            if re.is_match(&content) {
                re.replace(&content, new_rule.as_str()).to_string()
            } else {
                format!("{}\n{new_rule}\n", content.trim_end())
            }
        } else { return false }
    };
    fs::write(&conf_file, new).is_ok()
}

pub fn get_primary_monitor_name() -> Option<String> {
    let (conf_file, is_lua) = workspace_config_file();
    let content = fs::read_to_string(&conf_file).ok()?;
    if is_lua {
        let re = Regex::new(r#"(?m)hl\.keyword\s*\(\s*"workspace"\s*,\s*"1[ \t]*,[ \t]*monitor:([\w-]+)"#).ok()?;
        re.captures(&content).map(|c| c[1].to_string())
    } else {
        let re = Regex::new(r"(?m)^[ \t]*workspace\s*=\s*1\s*,\s*monitor:([\w-]+)").ok()?;
        re.captures(&content).map(|c| c[1].to_string())
    }
}

#[cfg(test)]
mod overlap_tests {
    use super::*;

    fn mon(name: &str, x: i32, y: i32, w: i32, h: i32) -> Monitor {
        Monitor {
            name: name.to_string(),
            description: None,
            model: None,
            width: w,
            height: h,
            refresh_rate: 60.0,
            x,
            y,
            scale: 1.0,
            transform: 0,
            available_modes: Vec::new(),
        }
    }

    fn layout(monitors: Vec<Monitor>) -> (Vec<Monitor>, HashMap<String, (i32, i32)>) {
        let positions = monitors.iter().map(|m| (m.name.clone(), (m.x, m.y))).collect();
        (monitors, positions)
    }

    fn pair() -> (Vec<Monitor>, HashMap<String, (i32, i32)>) {
        layout(vec![mon("DP-1", 0, 0, 1920, 1080), mon("HDMI-A-1", 1920, 0, 1920, 1080)])
    }

    #[test]
    fn touching_edges_are_not_an_overlap() {
        let (monitors, positions) = pair();
        assert!(overlapping(&monitors, &positions, "DP-1", 0, 0).is_empty());
        assert!(overlapping(&monitors, &positions, "DP-1", 1920, 1080).is_empty());
    }

    #[test]
    fn a_single_pixel_of_overlap_is_reported() {
        let (monitors, positions) = pair();
        assert_eq!(overlapping(&monitors, &positions, "DP-1", 1919, 0), ["HDMI-A-1"]);
        assert_eq!(overlapping(&monitors, &positions, "DP-1", 2000, 500), ["HDMI-A-1"]);
    }

    #[test]
    fn a_rotated_monitor_uses_its_swapped_size() {
        let (mut monitors, positions) = pair();
        monitors[0].transform = 1;
        assert!(overlapping(&monitors, &positions, "DP-1", 840, 0).is_empty());
        assert_eq!(overlapping(&monitors, &positions, "DP-1", 841, 0), ["HDMI-A-1"]);
    }

    #[test]
    fn a_scaled_monitor_takes_its_logical_room() {
        let mut m = mon("DP-1", 0, 0, 3840, 2160);
        m.scale = 2.0;
        assert_eq!(m.eff_size(), (1920, 1080));
        m.transform = 1;
        assert_eq!(m.eff_size(), (1080, 1920));
    }

    #[test]
    fn scale_is_taken_into_account_before_reporting_a_clash() {
        let mut big = mon("DP-1", 0, 0, 3840, 2160);
        big.scale = 2.0;
        let (monitors, positions) = layout(vec![big, mon("HDMI-A-1", 1920, 0, 1920, 1080)]);
        assert!(overlapping(&monitors, &positions, "DP-1", 0, 0).is_empty());
        assert_eq!(overlapping(&monitors, &positions, "DP-1", 1, 0), ["HDMI-A-1"]);
    }

    #[test]
    fn a_mode_string_gives_up_its_resolution() {
        assert_eq!(mode_size("1920x1080@60.00Hz"), Some((1920, 1080)));
        assert_eq!(mode_size("preferred"), None);
    }

    #[test]
    fn every_monitor_hit_at_once_is_named() {
        let (monitors, positions) = layout(vec![
            mon("DP-1", 0, 0, 1920, 1080),
            mon("HDMI-A-1", 1920, 0, 1920, 1080),
            mon("DP-2", 3840, 0, 1920, 1080),
        ]);
        assert_eq!(
            overlapping(&monitors, &positions, "DP-1", 3000, 0),
            ["HDMI-A-1", "DP-2"]
        );
    }
}
