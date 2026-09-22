// Opacity backend — global and per-app window opacity, applied live and kept in HyprDesk's own config file.
// Backend de opacidad — opacidad global y por app de las ventanas, aplicada en vivo y guardada en el fichero de config propio de HyprDesk.

use crate::backend::hyprconf;
use std::process::Command;

pub use crate::backend::hyprconf::AppOpacity;

pub fn hyprctl_available() -> bool {
    Command::new("which")
        .arg("hyprctl")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Global opacity / Opacidad global ─────────────────────────

pub struct Opacities {
    pub active: f64,
    pub inactive: f64,
}

// Reads what the session really uses, whoever set it / Lee lo que la sesión usa de verdad, lo haya puesto quien lo haya puesto
pub fn get_opacities() -> Opacities {
    Opacities {
        active: hyprconf::get_float_option("decoration:active_opacity").unwrap_or(1.0),
        inactive: hyprconf::get_float_option("decoration:inactive_opacity").unwrap_or(0.9),
    }
}

pub fn set_opacity(key: &str, value: f64) -> Result<(), String> {
    let value = (value * 100.0).round() / 100.0;
    let active = match key {
        "active" => true,
        "inactive" => false,
        _ => return Err(format!("unknown opacity: {key}")),
    };
    hyprconf::update(
        |state| {
            if active {
                state.opacity_active = Some(value);
            } else {
                state.opacity_inactive = Some(value);
            }
        },
        hyprconf::opacity_lines,
        false,
    )
}

// ── Per-app opacity / Opacidad por app ───────────────────────

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

pub fn get_app_opacities() -> Vec<AppOpacity> {
    hyprconf::load_state().app_opacity
}

// Rules take effect on the reload that follows the write / Las reglas se aplican con la recarga que sigue a la escritura
pub fn set_app_opacity(app_class: &str, active: f64) -> Result<(), String> {
    if !hyprconf::valid_class(app_class) {
        return Err(format!("invalid window class: {app_class}"));
    }
    let active = (active * 100.0).round() / 100.0;
    hyprconf::update(|state| state.set_app_opacity(app_class, active), |_, _| Vec::new(), false)
}

pub fn remove_app_opacity(app_class: &str) -> Result<(), String> {
    hyprconf::update(|state| state.app_opacity.retain(|a| a.app != app_class), |_, _| Vec::new(), false)
}
