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

// Waits for a daemon in the background so it leaves no zombie once it exits / Espera a un daemon en segundo plano para que no deje un zombi al terminar
fn reap(child: std::io::Result<std::process::Child>) {
    if let Ok(mut child) = child {
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

// Stops only the daemons of the session HyprDesk runs in; a nested or second Hyprland keeps its own / Para solo los daemons de la sesión donde corre HyprDesk; un Hyprland anidado o segundo conserva los suyos
fn kill_gamma_daemons() {
    const NAMES: [&str; 4] = ["hyprsunset", "gammastep", "wlsunset", "redshift"];
    let own = wayland_env();
    let Ok(procs) = std::fs::read_dir("/proc") else { return };
    for entry in procs.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        let comm = std::fs::read_to_string(entry.path().join("comm")).unwrap_or_default();
        if !NAMES.contains(&comm.trim()) {
            continue;
        }
        let environ = std::fs::read(entry.path().join("environ")).unwrap_or_default();
        if same_session(&own, &parse_environ(&environ)) {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }
    }
}

fn parse_environ(raw: &[u8]) -> std::collections::HashMap<String, String> {
    raw.split(|b| *b == 0)
        .filter_map(|var| std::str::from_utf8(var).ok()?.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// Same Hyprland instance when both know it, otherwise the same display in the same runtime dir / La misma instancia de Hyprland si ambos la conocen, si no el mismo display en el mismo runtime dir
fn same_session(
    own: &std::collections::HashMap<String, String>,
    other: &std::collections::HashMap<String, String>,
) -> bool {
    let get = |env: &std::collections::HashMap<String, String>, key: &str| env.get(key).cloned().unwrap_or_default();
    let (own_sig, other_sig) = (get(own, "HYPRLAND_INSTANCE_SIGNATURE"), get(other, "HYPRLAND_INSTANCE_SIGNATURE"));
    if !own_sig.is_empty() && !other_sig.is_empty() {
        return own_sig == other_sig;
    }
    if !get(own, "WAYLAND_DISPLAY").is_empty() {
        return get(own, "WAYLAND_DISPLAY") == get(other, "WAYLAND_DISPLAY")
            && get(own, "XDG_RUNTIME_DIR") == get(other, "XDG_RUNTIME_DIR");
    }
    get(own, "DISPLAY") == get(other, "DISPLAY")
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
                log(&format!("  spawn result: {:?}", result.as_ref().map(|c| c.id())));
                reap(result);
            }
        }
        "wlsunset" => {
            kill_gamma_daemons();
            std::thread::sleep(Duration::from_millis(200));
            let env = wayland_env();
            let low = temp_k.saturating_sub(1).max(1);
            reap(Command::new(&tool)
                .args(["-t", &low.to_string(), "-T", &temp_k.to_string()])
                .envs(&env)
                .stdout(Stdio::null()).stderr(Stdio::null())
                .spawn());
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

// Builds the restore commands / Construye los comandos de restauración
fn restore_commands(config: &Config, backlight: bool, tool: &str) -> String {
    let b = config.brightness;
    let name = tool.split('/').next_back().unwrap_or("");
    let t = if config.night_mode { config.night_temp } else { 6500 };
    let mut lines: Vec<String> = Vec::new();

    // Brightness: the hardware backlight when there is one / Brillo: la retroiluminación por hardware cuando la hay
    if backlight {
        lines.push(format!("    brightnessctl set {b}% &>/dev/null || true"));
    }

    // Colour temperature, and brightness by gamma when there is no backlight / Temperatura de color, y brillo por gamma cuando no hay retroiluminación
    if !tool.is_empty() {
        let bv = b as f64 / 100.0;
        lines.push(match (name, backlight) {
            ("redshift" | "gammastep", true) => format!("    {tool} -P -O {t} &>/dev/null || true"),
            ("redshift" | "gammastep", false) => {
                format!("    {tool} -P -O {t} -b {bv:.2}:{bv:.2} &>/dev/null || true")
            }
            ("hyprsunset", true) => format!("    {tool} -t {t} &"),
            ("hyprsunset", false) => {
                format!("    {tool} -t {t} & sleep 1 && hyprctl hyprsunset gamma {b} || true")
            }
            ("wlsunset", _) => format!("    {tool} -t {} -T {t} &", t.saturating_sub(1).max(1)),
            _ => String::new(),
        });
    }

    lines.retain(|l| !l.trim().is_empty());
    lines.join("\n")
}

// Writes the startup script and lists it in HyprDesk's own config file / Escribe el script de arranque y lo apunta en el fichero de config propio de HyprDesk
pub fn setup_autostart(config: &Config) {
    use crate::backend::hyprconf;
    use crate::config::startup_script;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let restore = restore_commands(config, has_backlight(), &gamma_tool().unwrap_or_default());

    let script = format!("#!/bin/bash\n# HyprDesk startup — generado automáticamente.\n{restore}\n");
    let path = startup_script();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&path, &script).is_ok() {
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o755));
    }

    // This runs with every saved value, so Hyprland is only touched when the entry is new / Esto corre con cada valor guardado, así que Hyprland solo se toca si la entrada es nueva
    let entry = path.display().to_string();
    if hyprconf::load_state().autostart.as_deref() != Some(entry.as_str()) {
        let _ = hyprconf::update(|state| state.autostart = Some(entry.clone()), |_, _| Vec::new(), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // A nested Hyprland must never stop the night light of the real session / Un Hyprland anidado nunca debe parar la luz nocturna de la sesión real
    #[test]
    fn gamma_daemons_of_another_session_are_spared() {
        let nested = env(&[("HYPRLAND_INSTANCE_SIGNATURE", "nested"), ("WAYLAND_DISPLAY", "wayland-1"), ("XDG_RUNTIME_DIR", "/run/user/1000/hdt")]);
        let real = env(&[("HYPRLAND_INSTANCE_SIGNATURE", "real"), ("WAYLAND_DISPLAY", "wayland-1"), ("XDG_RUNTIME_DIR", "/run/user/1000")]);
        assert!(!same_session(&nested, &real));
        assert!(same_session(&real, &real.clone()));

        // Started by systemd, without the signature: the display and runtime dir decide / Lanzado por systemd, sin la firma: deciden el display y el runtime dir
        let service = env(&[("WAYLAND_DISPLAY", "wayland-1"), ("XDG_RUNTIME_DIR", "/run/user/1000")]);
        assert!(same_session(&real, &service));
        assert!(!same_session(&nested, &service));

        // X11 tools only have DISPLAY / Las herramientas de X11 solo tienen DISPLAY
        assert!(same_session(&env(&[("DISPLAY", ":0")]), &env(&[("DISPLAY", ":0")])));
        assert!(!same_session(&env(&[("DISPLAY", ":0")]), &env(&[("DISPLAY", ":1")])));
    }

    #[test]
    fn proc_environ_is_split_on_nul() {
        let parsed = parse_environ(b"HYPRLAND_INSTANCE_SIGNATURE=abc\0XDG_RUNTIME_DIR=/run/user/1000\0BROKEN\0");
        assert_eq!(parsed.get("HYPRLAND_INSTANCE_SIGNATURE").map(String::as_str), Some("abc"));
        assert_eq!(parsed.len(), 2);
    }

    // Every hardware combination writes what it must / Cada combinación de hardware escribe lo que debe
    #[test]
    fn restore_covers_brightness_and_temperature() {
        let cfg = Config { brightness: 40, night_mode: true, night_temp: 3000, ..Config::default() };

        // Laptop: backlight for brightness, the tool only for temperature / Portátil: retroiluminación para el brillo, la herramienta solo para la temperatura
        let laptop = restore_commands(&cfg, true, "/usr/bin/hyprsunset");
        assert!(laptop.contains("brightnessctl set 40%"), "falta el brillo: {laptop}");
        assert!(laptop.contains("-t 3000"), "falta la temperatura: {laptop}");
        assert!(!laptop.contains("hyprsunset gamma"), "no debe bajar el brillo dos veces: {laptop}");

        // Desktop: no backlight, gamma does both / Sobremesa: sin retroiluminación, el gamma hace las dos
        let desktop = restore_commands(&cfg, false, "/usr/bin/hyprsunset");
        assert!(!desktop.contains("brightnessctl"), "no hay retroiluminación que usar: {desktop}");
        assert!(desktop.contains("-t 3000") && desktop.contains("gamma 40"), "{desktop}");

        // Same split for the other tools / El mismo reparto con las otras herramientas
        let gammastep = restore_commands(&cfg, true, "/usr/bin/gammastep");
        assert!(gammastep.contains("brightnessctl set 40%") && gammastep.contains("-O 3000"), "{gammastep}");
        assert!(!gammastep.contains(" -b "), "el brillo ya lo pone brightnessctl: {gammastep}");
        // wlsunset refuses to start unless -T > -t / wlsunset se niega a arrancar si -T no es mayor que -t
        let wlsunset = restore_commands(&cfg, true, "/usr/bin/wlsunset");
        assert!(wlsunset.contains("-T 3000") && wlsunset.contains("-t 2999"), "{wlsunset}");

        // Night mode off restores neutral light / Con el modo noche apagado se restaura luz neutra
        let day = Config { night_mode: false, ..cfg.clone() };
        assert!(restore_commands(&day, true, "/usr/bin/hyprsunset").contains("-t 6500"));

        // No tool at all: brightness only, and nothing at all without backlight / Sin herramienta: solo brillo, y nada sin retroiluminación
        assert!(restore_commands(&cfg, true, "").contains("brightnessctl"));
        assert_eq!(restore_commands(&cfg, false, ""), "");
    }

}
