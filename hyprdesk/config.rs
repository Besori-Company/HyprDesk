// App configuration — loads and saves settings to ~/.config/hyprdesk/config.json.
// Configuración de la app — carga y guarda ajustes en ~/.config/hyprdesk/config.json.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub brightness: u32,
    pub night_mode: bool,
    pub night_temp: u32,
    pub app_lang: String,
    pub opacity_active: f64,
    pub opacity_inactive: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brightness: 100,
            night_mode: false,
            night_temp: 4000,
            app_lang: detect_system_lang(),
            opacity_active: 1.0,
            opacity_inactive: 0.9,
        }
    }
}

fn detect_system_lang() -> String {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LANGUAGE"))
        .unwrap_or_default();
    if lang.to_lowercase().starts_with("es") {
        "es".into()
    } else {
        "en".into()
    }
}

pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

pub fn config_dir() -> PathBuf {
    home_dir().join(".config").join("hyprdesk")
}

pub fn hypr_dir() -> PathBuf {
    home_dir().join(".config").join("hypr")
}

#[allow(dead_code)]
pub fn startup_script() -> PathBuf {
    hypr_dir().join("hyprdesk-startup.sh")
}

#[allow(dead_code)]
pub fn hypr_conf() -> PathBuf {
    hypr_dir().join("hyprland.conf")
}

pub fn hypr_lua() -> PathBuf {
    hypr_dir().join("hyprland.lua")
}

pub fn is_lua_config() -> bool {
    hypr_dir().join("hyprland.lua").exists()
}

pub fn load_config() -> Config {
    let path = config_dir().join("config.json");
    if path.exists() {
        if let Ok(s) = fs::read_to_string(&path) {
            if let Ok(c) = serde_json::from_str::<serde_json::Value>(&s) {
                let def = Config::default();
                return Config {
                    brightness: c["brightness"].as_u64().unwrap_or(def.brightness as u64) as u32,
                    night_mode: c["night_mode"].as_bool().unwrap_or(def.night_mode),
                    night_temp: c["night_temp"].as_u64().unwrap_or(def.night_temp as u64) as u32,
                    app_lang: c["app_lang"].as_str().unwrap_or(&def.app_lang).to_string(),
                    opacity_active: c["opacity_active"].as_f64().unwrap_or(def.opacity_active),
                    opacity_inactive: c["opacity_inactive"].as_f64().unwrap_or(def.opacity_inactive),
                };
            }
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(s) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(dir.join("config.json"), s);
    }
}
