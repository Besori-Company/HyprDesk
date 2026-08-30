// Display backend — controls brightness and color temperature via brightnessctl, hyprsunset or wlsunset.
// Backend de pantalla — controla brillo y temperatura de color mediante brightnessctl, hyprsunset o wlsunset.

use crate::config::Config;
use std::process::{Command, Stdio};
use std::time::Duration;

fn run(cmd: &[&str]) -> (i32, String) {
    match Command::new(cmd[0]).args(&cmd[1..]).output() {
        Ok(o) => (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Err(_) => (-1, String::new()),
    }
}

fn which(name: &str) -> Option<String> {
    let (rc, path) = run(&["which", name]);
    if rc == 0 && !path.is_empty() { Some(path) } else { None }
}

pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
}

pub fn is_hyprland() -> bool {
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || std::env::var("XDG_CURRENT_DESKTOP").as_deref().map(|s| s.to_lowercase()) == Ok("hyprland".into())
        || std::env::var("XDG_SESSION_DESKTOP").as_deref().map(|s| s.to_lowercase()) == Ok("hyprland".into())
}

pub fn has_backlight() -> bool {
    let (rc, out) = run(&["brightnessctl", "-l"]);
    rc == 0 && out.to_lowercase().contains("backlight")
}

pub fn get_brightness(config: &Config) -> u32 {
    if has_backlight() {
        let (_, cur) = run(&["brightnessctl", "get"]);
        let (_, max) = run(&["brightnessctl", "max"]);
        if let (Ok(c), Ok(m)) = (cur.parse::<u64>(), max.parse::<u64>()) {
            if m > 0 {
                return (c * 100 / m).clamp(1, 100) as u32;
            }
        }
    }
    config.brightness
}

pub fn set_brightness(pct: u32, config: &Config) {
    let pct = pct.clamp(10, 100);
    if has_backlight() {
        run(&["brightnessctl", "set", &format!("{pct}%")]);
        return;
    }
    let temp = if config.night_mode { config.night_temp } else { 6500 };
    apply_gamma_bg(pct, temp);
}

pub fn brightness_method() -> (&'static str, bool) {
    if has_backlight() {
        return ("backlight (brightnessctl)", true);
    }
    if let Some(tool) = gamma_tool() {
        let name = tool.split('/').last().unwrap_or(&tool);
        return (Box::leak(format!("gamma ({name})").into_boxed_str()), true);
    }
    ("no tool installed", false)
}

pub fn gamma_tool() -> Option<String> {
    if is_hyprland() {
        which("hyprsunset").or_else(|| which("wlsunset"))
    } else if !is_wayland() {
        which("gammastep").or_else(|| which("redshift"))
    } else {
        None
    }
}

pub fn night_tool_available() -> bool {
    gamma_tool().is_some()
}

fn kill_gamma_daemons() {
    for name in &["hyprsunset", "gammastep", "wlsunset", "redshift"] {
        let _ = Command::new("pkill").arg("-x").arg(name).output();
    }
}

fn apply_gamma(brightness_pct: u32, temp_k: u32) {
    let log = |msg: &str| {
        let _ = std::fs::OpenOptions::new().create(true).append(true)
            .open("/tmp/hyprdesk_debug.log")
            .map(|mut f| { use std::io::Write; let _ = writeln!(f, "{}", msg); });
    };

    let tool = match gamma_tool() {
        Some(t) => t,
        None => { log("gamma_tool() = None — no tool found"); return; }
    };
    let name = tool.split('/').last().unwrap_or(&tool).to_string();
    let b = (brightness_pct as f64 / 100.0).clamp(0.1, 1.0);
    log(&format!("apply_gamma: tool={tool} brightness={brightness_pct} temp={temp_k}"));

    match name.as_str() {
        "gammastep" | "redshift" => {
            kill_gamma_daemons();
            let _ = Command::new(&tool)
                .args(["-P", "-O", &temp_k.to_string(), "-b", &format!("{b:.2}:{b:.2}")])
                .output();
        }
        "hyprsunset" => {
            let env = wayland_env();
            let sig = env.get("HYPRLAND_INSTANCE_SIGNATURE").cloned().unwrap_or_default();
            let rt_dir = env.get("XDG_RUNTIME_DIR").cloned().unwrap_or_default();
            let socket = format!("{rt_dir}/hypr/{sig}/.hyprsunset.sock");
            let mut ipc_ok = false;

            if !sig.is_empty() && std::path::Path::new(&socket).exists() {
                if let Some(hyprctl) = which("hyprctl") {
                    log(&format!("  IPC path: hyprctl hyprsunset temperature {temp_k}"));
                    let r1 = Command::new(&hyprctl)
                        .args(["hyprsunset", "temperature", &temp_k.to_string()])
                        .envs(&env).output();
                    let r2 = Command::new(&hyprctl)
                        .args(["hyprsunset", "gamma", &brightness_pct.to_string()])
                        .envs(&env).output();
                    ipc_ok = r1.map(|o| o.status.success()).unwrap_or(false)
                        && r2.map(|o| o.status.success()).unwrap_or(false);
                    log(&format!("  IPC result: ok={ipc_ok}"));
                }
            }

            if !ipc_ok {
                log(&format!("  fallback: kill + hyprsunset -t {temp_k} -g {brightness_pct}"));
                kill_gamma_daemons();
                std::thread::sleep(Duration::from_millis(200));
                let result = Command::new(&tool)
                    .args(["-t", &temp_k.to_string(), "-g", &brightness_pct.to_string()])
                    .envs(&env)
                    .stdout(Stdio::null()).stderr(Stdio::null())
                    .spawn();
                log(&format!("  spawn result: {:?}", result.map(|c| c.id())));
            }
        }
        "wlsunset" => {
            kill_gamma_daemons();
            let env = wayland_env();
            let _ = Command::new(&tool)
                .args(["-T", &temp_k.to_string()])
                .envs(&env)
                .stdout(Stdio::null()).stderr(Stdio::null())
                .spawn();
        }
        _ => {}
    }
}

pub fn apply_gamma_bg(brightness_pct: u32, temp_k: u32) {
    std::thread::spawn(move || apply_gamma(brightness_pct, temp_k));
}

pub fn apply_night_mode(enabled: bool, temp: u32, brightness_pct: u32) {
    let t = if enabled { temp } else { 6500 };
    apply_gamma_bg(brightness_pct, t);
}

fn wayland_env() -> std::collections::HashMap<String, String> {
    let mut env = std::collections::HashMap::new();
    for var in &["WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE", "XDG_CURRENT_DESKTOP"] {
        if let Ok(val) = std::env::var(var) {
            env.insert(var.to_string(), val);
        }
    }
    env
}

// Writes the startup script and hooks it into the Hyprland config / Escribe el script de arranque y lo engancha en la configuración de Hyprland
pub fn setup_autostart(config: &Config) {
    use crate::config::{hypr_conf, startup_script};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let b = config.brightness;
    let temp = config.night_temp;
    let nm = config.night_mode;
    let tool = gamma_tool().unwrap_or_default();
    let name = tool.split('/').last().unwrap_or("").to_string();

    let restore = if has_backlight() {
        format!("    brightnessctl set {b}% &>/dev/null || true")
    } else if !tool.is_empty() {
        let t = if nm { temp } else { 6500 };
        let bv = b as f64 / 100.0;
        match name.as_str() {
            "redshift" | "gammastep" => format!("    {tool} -P -O {t} -b {bv:.2}:{bv:.2} &>/dev/null || true"),
            "hyprsunset" => format!("    {tool} -t {t} & sleep 1 && hyprctl hyprsunset gamma {b} || true"),
            "wlsunset" => format!("    {tool} -T {t} &"),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let script = format!("#!/bin/bash\n# HyprDesk startup — generado automáticamente.\n{restore}\n");
    let path = startup_script();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&path, &script).is_ok() {
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
    }

    use crate::config::{hypr_lua, is_lua_config};
    let (hypr, line) = if is_lua_config() {
        (
            hypr_lua(),
            format!("\n-- HyprDesk autostart\nhl.keyword(\"exec-once\", \"{}\")\n", path.display()),
        )
    } else {
        (
            hypr_conf(),
            format!("\n# HyprDesk autostart\nexec-once = {}\n", path.display()),
        )
    };
    if hypr.exists() {
        if let Ok(content) = fs::read_to_string(&hypr) {
            if !content.contains("hyprdesk-startup.sh") {
                let _ = fs::write(&hypr, content + &line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Writes and rewrites the autostart in a throwaway HOME / Escribe y reescribe el autostart en un HOME desechable
    #[test]
    fn autostart_is_written_once() {
        let tmp = std::env::temp_dir().join("hyprdesk-autostart-test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".config/hypr")).unwrap();
        let conf = tmp.join(".config/hypr/hyprland.conf");
        let original = "monitor = , preferred, auto, 1\nexec-once = waybar\n";
        fs::write(&conf, original).unwrap();
        unsafe { std::env::set_var("HOME", &tmp) };

        let cfg = Config { brightness: 42, ..Config::default() };
        setup_autostart(&cfg);

        let script = tmp.join(".config/hypr/hyprdesk-startup.sh");
        assert!(script.exists(), "no se creó el script de arranque");
        let text = fs::read_to_string(&conf).unwrap();
        assert!(text.contains("hyprdesk-startup.sh"), "no se añadió el exec-once");

        // Running it again must not duplicate the line / Ejecutarlo otra vez no debe duplicar la línea
        setup_autostart(&cfg);
        let text = fs::read_to_string(&conf).unwrap();
        assert_eq!(text.matches("hyprdesk-startup.sh").count(), 1, "se duplicó el exec-once");

        // Nothing else in the file may be touched / No se puede tocar nada más del fichero
        for line in original.lines() {
            assert!(text.contains(line), "se perdió una línea ajena: {line}");
        }

        let _ = fs::remove_dir_all(&tmp);
    }
}
