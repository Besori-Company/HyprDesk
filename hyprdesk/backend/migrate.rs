// Migration — moves what HyprDesk 1.0.x wrote inside the user's files over to HyprDesk's own config file.
// Migración — lleva lo que HyprDesk 1.0.x escribió dentro de los ficheros del usuario al fichero de config propio de HyprDesk.

use crate::backend::hyprconf::{self, HyprState, MonitorRule, LAYOUT_VERSION};
use crate::config::{config_dir, hypr_dir};
use regex::Regex;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct Report {
    pub backup: PathBuf,
    pub moved: usize,
}

// Files 1.0.x chose for monitors and workspace 1 / Ficheros que 1.0.x elegía para monitores y el espacio 1
const WRITTEN_BY_1_0: &[&str] = &[
    "hyprland.conf",
    "monitors.conf",
    "UserConfigs/monitors.conf",
    "UserConfigs/UserSettings.conf",
    "UserConfigs/01-UserDefaults.conf",
    "hyprland.lua",
    "monitors.lua",
    "UserConfigs/monitors.lua",
    "UserConfigs/UserSettings.lua",
    "UserConfigs/01-UserDefaults.lua",
];

struct Patterns {
    monitor_conf: Regex,
    workspace_conf: Regex,
    monitor_lua: Regex,
    workspace_lua: Regex,
    opacity_lua: Regex,
    block_opacity: Regex,
    rule_conf: Regex,
    rule_lua: Regex,
}

impl Patterns {
    fn new() -> Self {
        Self {
            // Only the exact shape 1.0.x wrote: no spaces, Hz and a two-decimal scale / Solo la forma exacta que escribía 1.0.x: sin espacios, Hz y escala con dos decimales
            monitor_conf: Regex::new(r"^monitor=([^,\s]+),(\d+x\d+@[\d.]+Hz),(-?\d+)x(-?\d+),(\d+\.\d{2}),transform,(\d)$").unwrap(),
            workspace_conf: Regex::new(r"^workspace = 1, monitor:([\w.-]+), default:true$").unwrap(),
            monitor_lua: Regex::new(r#"^hl\.keyword\("monitor", "([^,"\s]+),(\d+x\d+@[\d.]+Hz),(-?\d+)x(-?\d+),(\d+\.\d{2}),transform,(\d)"\)$"#).unwrap(),
            workspace_lua: Regex::new(r#"^hl\.keyword\("workspace", "1, monitor:([\w.-]+), default:true"\)$"#).unwrap(),
            opacity_lua: Regex::new(r#"^hl\.keyword\("decoration:(active|inactive)_opacity", "([\d.]+)"\)$"#).unwrap(),
            block_opacity: Regex::new(r"^  (active|inactive)_opacity = ([\d.]+)$").unwrap(),
            rule_conf: Regex::new(r"^\s*(?:windowrule\s*=\s*match:class\s+\^\(([^)]+)\)\$\s*,\s*opacity\s+([\d.]+)|windowrule(?:v2)?\s*=\s*opacity\s+([\d.]+)(?:\s+[\d.]+)?\s*,\s*class:\^\(([^)]+)\)\$)").unwrap(),
            rule_lua: Regex::new(r#"^\s*hl\.keyword\s*\(\s*"windowrule"\s*,\s*"match:class \^\(([^)]+)\)\$\s*,\s*opacity\s+([\d.]+)"\s*\)"#).unwrap(),
        }
    }
}

fn set_opacity(state: &mut HyprState, key: &str, value: &str) {
    let Ok(v) = value.parse::<f64>() else { return };
    if key == "active" {
        state.opacity_active = Some(v);
    } else {
        state.opacity_inactive = Some(v);
    }
}

fn monitor_from(c: &regex::Captures) -> Option<MonitorRule> {
    Some(MonitorRule {
        name: c[1].to_string(),
        mode: c[2].to_string(),
        x: c[3].parse().ok()?,
        y: c[4].parse().ok()?,
        scale: c[5].parse().ok()?,
        transform: c[6].parse().ok()?,
    })
}

// Old include and autostart lines, HyprDesk's own and safe to delete / Líneas antiguas de include y autostart, propias de HyprDesk y seguras de borrar
fn is_old_hook(t: &str) -> bool {
    ["# HyprDesk autostart", "-- HyprDesk autostart", "# HyprDesk per-app opacity", "-- HyprDesk per-app opacity"]
        .iter()
        .any(|mark| t.starts_with(mark))
        || (t.contains("hyprdesk-startup.sh") && (t.starts_with("exec-once") || t.starts_with("hl.keyword(")))
        || (t.contains("hyprdesk-opacity.") && (t.starts_with("source") || t.starts_with("dofile(")))
}

fn lua_autostart_len(lines: &[&str], i: usize) -> usize {
    let t = lines[i].trim();
    if !t.starts_with("hl.on(\"hyprland.start\"") {
        return 0;
    }
    if t.contains("hyprdesk-startup.sh") && t.ends_with("end)") {
        return 1;
    }
    let body = lines.get(i + 1).map_or("", |l| l.trim());
    let close = lines.get(i + 2).map_or("", |l| l.trim());
    if t.ends_with("function()") && body.starts_with("hl.exec_cmd(") && body.contains("hyprdesk-startup.sh") && close == "end)" {
        return 3;
    }
    0
}

fn without_stray_autostart(text: &str, lua: bool) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        let n = if lua {
            lua_autostart_len(&lines, i)
        } else {
            usize::from(t.starts_with("exec-once") && t.contains("hyprdesk-startup.sh"))
        };
        if n == 0 {
            out.push(lines[i]);
            i += 1;
            continue;
        }
        if out.last().is_some_and(|l| l.trim().is_empty()) {
            out.pop();
        }
        i += n;
    }
    if out.len() == lines.len() {
        return None;
    }
    let mut result = out.join("\n");
    if text.ends_with('\n') {
        result.push('\n');
    }
    Some(result)
}

// A hook left in the entry file runs the startup script a second time, and two gamma daemons at once lock Hyprland's gamma for the session / Un enganche olvidado en el fichero de entrada lanza el script de arranque una segunda vez, y dos daemons de gamma a la vez bloquean el gamma de Hyprland toda la sesión
fn drop_stray_autostart(entry: &Path, backup_root: &Path) -> io::Result<()> {
    let Ok(text) = fs::read_to_string(entry) else { return Ok(()) };
    let lua = entry.extension().is_some_and(|e| e == "lua");
    let Some(clean) = without_stray_autostart(&text, lua) else { return Ok(()) };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = backup_root.join(stamp.to_string());
    fs::create_dir_all(&backup)?;
    fs::copy(entry, backup.join(entry.file_name().unwrap_or_default()))?;
    hyprconf::write_atomic(entry, &clean)
}

// Returns the rewritten text and how many HyprDesk entries it moved / Devuelve el texto reescrito y cuántas entradas de HyprDesk movió
fn rewrite(text: &str, lua: bool, pats: &Patterns, state: &mut HyprState) -> (String, usize) {
    let tag = if lua { "-- [HyprDesk → hyprdesk.lua]" } else { "# [HyprDesk → hyprdesk.conf]" };
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut moved = 0;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();

        let hook_len = if is_old_hook(t) { 1 } else if lua { lua_autostart_len(&lines, i) } else { 0 };
        if hook_len > 0 {
            if out.last().is_some_and(|l| l.trim().is_empty()) {
                out.pop();
            }
            moved += 1;
            i += hook_len;
            continue;
        }

        // The decoration block 1.0.x appended when no file had an opacity yet / El bloque decoration que 1.0.x añadía si ningún fichero tenía opacidad
        if !lua && t == "decoration {" && i + 2 < lines.len() && lines[i + 2].trim() == "}" {
            if let Some(c) = pats.block_opacity.captures(lines[i + 1]) {
                set_opacity(state, &c[1], &c[2]);
                out.extend(lines[i..=i + 2].iter().map(|l| format!("{tag} {l}")));
                moved += 1;
                i += 3;
                continue;
            }
        }

        let imported = if lua {
            if let Some(c) = pats.monitor_lua.captures(t) {
                monitor_from(&c).map(|m| state.set_monitor(m)).is_some()
            } else if let Some(c) = pats.workspace_lua.captures(t) {
                state.primary = Some(c[1].to_string());
                true
            } else if let Some(c) = pats.opacity_lua.captures(t) {
                set_opacity(state, &c[1], &c[2]);
                true
            } else {
                // hl.keyword never existed in Hyprland, whatever is left was ours and broken / hl.keyword nunca existió en Hyprland, lo que quede era nuestro y roto
                t.starts_with("hl.keyword(")
            }
        } else if let Some(c) = pats.monitor_conf.captures(t) {
            monitor_from(&c).map(|m| state.set_monitor(m)).is_some()
        } else if let Some(c) = pats.workspace_conf.captures(t) {
            state.primary = Some(c[1].to_string());
            true
        } else {
            false
        };

        if imported {
            out.push(format!("{tag} {line}"));
            moved += 1;
        } else {
            out.push(line.to_string());
        }
        i += 1;
    }
    let mut result = out.join("\n");
    if text.ends_with('\n') {
        result.push('\n');
    }
    (result, moved)
}

fn legacy_rules(text: &str, lua: bool, pats: &Patterns) -> Vec<(String, f64)> {
    text.lines()
        .filter_map(|l| {
            if lua {
                let c = pats.rule_lua.captures(l)?;
                return Some((c[1].to_string(), c[2].parse().ok()?));
            }
            let c = pats.rule_conf.captures(l)?;
            let (app, value) = match (c.get(1), c.get(2), c.get(3), c.get(4)) {
                (Some(app), Some(value), _, _) => (app, value),
                (_, _, Some(value), Some(app)) => (app, value),
                _ => return None,
            };
            Some((app.as_str().to_string(), value.as_str().parse().ok()?))
        })
        .collect()
}

fn lua_files_with_keyword(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            lua_files_with_keyword(&path, out);
        } else if path.extension().is_some_and(|e| e == "lua")
            && fs::read_to_string(&path).is_ok_and(|s| s.contains("hl.keyword("))
        {
            out.push(path);
        }
    }
}

// Imports 1.0.x entries into the state and disables them in place, after copying every touched file / Importa las entradas de 1.0.x al estado y las desactiva en su sitio, tras copiar cada fichero tocado
pub fn migrate_in(hypr: &Path, backup_root: &Path, state: &mut HyprState) -> io::Result<Option<Report>> {
    let pats = Patterns::new();
    let mut moved = 0;
    let mut rewrites: Vec<(PathBuf, String)> = Vec::new();
    let mut stale: Vec<PathBuf> = Vec::new();

    for (name, lua) in [("hyprdesk-opacity.conf", false), ("hyprdesk-opacity.lua", true)] {
        let path = hypr.join(name);
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (app, value) in legacy_rules(&text, lua, &pats) {
            state.set_app_opacity(&app, value);
            moved += 1;
        }
        stale.push(path);
    }

    let mut files: Vec<PathBuf> = WRITTEN_BY_1_0.iter().map(|f| hypr.join(f)).filter(|p| p.is_file()).collect();
    let mut with_keyword = Vec::new();
    lua_files_with_keyword(hypr, &mut with_keyword);
    for path in with_keyword {
        let ours = path.file_name().is_some_and(|n| n == "hyprdesk.lua" || n == "hyprdesk-opacity.lua");
        if !ours && !files.contains(&path) {
            files.push(path);
        }
    }

    for path in files {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        let lua = path.extension().is_some_and(|e| e == "lua");
        let (new_text, n) = rewrite(&text, lua, &pats, state);
        if n > 0 {
            rewrites.push((path, new_text));
            moved += n;
        }
    }

    if rewrites.is_empty() && stale.is_empty() {
        return Ok(None);
    }

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = backup_root.join(stamp.to_string());
    for path in rewrites.iter().map(|(p, _)| p).chain(stale.iter()) {
        let dest = backup.join(path.strip_prefix(hypr).unwrap_or(path));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &dest)?;
    }
    for (path, text) in &rewrites {
        hyprconf::write_atomic(path, text)?;
    }
    for path in &stale {
        fs::remove_file(path)?;
    }
    Ok(Some(Report { backup, moved }))
}

// Runs on every start: migrates 1.0.x once, then makes sure file and hook match the state / Corre en cada arranque: migra 1.0.x una vez y luego asegura que fichero y enganche coinciden con el estado
pub fn on_startup() -> Option<Report> {
    let hypr = hypr_dir();
    let state_dir = config_dir();
    let saved = hyprconf::load_state_in(&state_dir);
    let mut state = saved.clone();
    let mut report = None;
    if state.layout_version < LAYOUT_VERSION {
        match migrate_in(&hypr, &state_dir.join("backups"), &mut state) {
            Ok(r) => {
                report = r;
                state.layout_version = LAYOUT_VERSION;
            }
            Err(_) => return None,
        }
    }

    // Also covers a switch between hyprlang and Lua since the last start / También cubre un cambio entre hyprlang y Lua desde el último arranque
    let p = hyprconf::provider();
    let file_ok = fs::read_to_string(hypr.join(p.managed_name())).is_ok_and(|s| s == hyprconf::render(p, &state));
    let hook_ok = fs::read_to_string(hypr.join(p.entry_name())).map_or(true, |s| hyprconf::has_hook(p, &s));
    let in_place = if state != saved || !file_ok || !hook_ok {
        hyprconf::commit_in(&hypr, &state_dir, p, &state, &[], true, &hyprconf::Hyprctl).is_ok()
    } else {
        true
    };

    // Also fixes installs that already had the old file / También arregla las instalaciones que ya tenían el fichero antiguo
    if in_place {
        let _ = hyprconf::quiet_twin(p, &hypr, &state);
    }

    if in_place && state.autostart.is_some() {
        let _ = drop_stray_autostart(&hypr.join(p.entry_name()), &state_dir.join("backups"));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hyprdesk-migrate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("hypr/UserConfigs")).unwrap();
        dir
    }

    // Shaped after a real 1.0.x install / Con la forma de una instalación real de 1.0.x
    const HYPRLAND_CONF: &str = r"source= $HOME/.config/hypr/monitors.conf
workspace = 1, monitor:HDMI-A-2, default:true

# HyprDesk per-app opacity overrides (loaded last to take priority)
source = /home/u/.config/hypr/hyprdesk-opacity.conf

# HyprDesk autostart
exec-once = /home/u/.config/hypr/hyprdesk-startup.sh
";

    const MONITORS_CONF: &str = r"# default Monitor config
#monitor = eDP-1, preferred, auto, 1
monitor = DP-9, preferred, auto, 1
monitor=HDMI-A-1,1920x1080@60.00Hz,0x0,1.00,transform,3
monitor=HDMI-A-2,1920x1080@60.00Hz,-1920x0,1.00,transform,0
";

    const OPACITY_CONF: &str = r"
windowrule = match:class ^(org.gnome.Nautilus)$, opacity 1.00
windowrule = match:class ^(code)$, opacity 1.00
windowrule = match:class ^(org.gnome.Loupe)$, opacity 1.00
windowrule = match:class ^(google-chrome)$, opacity 0.97
windowrule = match:class ^(kitty)$, opacity 0.95
";

    fn install_1_0(root: &Path) -> PathBuf {
        let hypr = root.join("hypr");
        fs::write(hypr.join("hyprland.conf"), HYPRLAND_CONF).unwrap();
        fs::write(hypr.join("monitors.conf"), MONITORS_CONF).unwrap();
        fs::write(hypr.join("hyprdesk-opacity.conf"), OPACITY_CONF).unwrap();
        hypr
    }

    #[test]
    fn moves_what_1_0_wrote_and_backs_it_up() {
        let root = scratch("conf");
        let hypr = install_1_0(&root);
        let mut state = HyprState::default();

        let report = migrate_in(&hypr, &root.join("backups"), &mut state).unwrap().unwrap();

        assert_eq!(report.moved, 12);
        assert_eq!(state.monitors.len(), 2);
        assert_eq!((state.monitors[1].name.as_str(), state.monitors[1].x, state.monitors[1].transform), ("HDMI-A-2", -1920, 0));
        assert_eq!(state.monitors[0].transform, 3);
        assert_eq!(state.primary.as_deref(), Some("HDMI-A-2"));
        assert_eq!(state.app_opacity.len(), 5);
        assert!(state.app_opacity.iter().any(|a| a.app == "kitty" && a.active == 0.95));

        assert_eq!(
            fs::read_to_string(hypr.join("hyprland.conf")).unwrap(),
            "source= $HOME/.config/hypr/monitors.conf\n# [HyprDesk → hyprdesk.conf] workspace = 1, monitor:HDMI-A-2, default:true\n"
        );
        let monitors = fs::read_to_string(hypr.join("monitors.conf")).unwrap();
        assert!(monitors.contains("\n#monitor = eDP-1, preferred, auto, 1\nmonitor = DP-9, preferred, auto, 1\n"), "{monitors}");
        assert_eq!(monitors.matches("# [HyprDesk → hyprdesk.conf] monitor=HDMI-A-").count(), 2, "{monitors}");
        assert!(!hypr.join("hyprdesk-opacity.conf").exists());

        assert_eq!(fs::read_to_string(report.backup.join("hyprland.conf")).unwrap(), HYPRLAND_CONF);
        assert_eq!(fs::read_to_string(report.backup.join("monitors.conf")).unwrap(), MONITORS_CONF);
        assert_eq!(fs::read_to_string(report.backup.join("hyprdesk-opacity.conf")).unwrap(), OPACITY_CONF);
    }

    #[test]
    fn stray_autostart_hooks_are_dropped() {
        let lua = "load_module(\"workspaces\")\nhl.on(\"hyprland.start\", function()\n  hl.exec_cmd(\"/home/u/.config/hypr/hyprdesk-startup.sh\")\nend)\n\n-- HyprDesk — keep this last / déjalo al final\npcall(require, \"hyprdesk\")\n";
        let clean = without_stray_autostart(lua, true).unwrap();
        assert_eq!(clean, "load_module(\"workspaces\")\n\n-- HyprDesk — keep this last / déjalo al final\npcall(require, \"hyprdesk\")\n");
        assert_eq!(without_stray_autostart(&clean, true), None);

        let one_line = "hl.on(\"hyprland.start\", function() hl.exec_cmd(\"/home/u/.config/hypr/hyprdesk-startup.sh\") end)\n";
        assert_eq!(without_stray_autostart(one_line, true).as_deref(), Some("\n"));

        let theirs = "hl.on(\"hyprland.start\", function()\n  hl.exec_cmd(\"waybar\")\nend)\n";
        assert_eq!(without_stray_autostart(theirs, true), None);

        let conf = "source = a.conf\nexec-once = /home/u/.config/hypr/hyprdesk-startup.sh\nexec-once = waybar\n";
        assert_eq!(without_stray_autostart(conf, false).as_deref(), Some("source = a.conf\nexec-once = waybar\n"));
        
        let mut state = HyprState::default();
        let labelled = "load_module(\"workspaces\")\n\n-- HyprDesk autostart\nhl.on(\"hyprland.start\", function()\n  hl.exec_cmd(\"/home/u/.config/hypr/hyprdesk-startup.sh\")\nend)\n";
        let (text, moved) = rewrite(labelled, true, &Patterns::new(), &mut state);
        assert_eq!(text, "load_module(\"workspaces\")\n");
        assert_eq!(moved, 2);
    }

    #[test]
    fn a_second_run_finds_nothing_to_move() {
        let root = scratch("twice");
        let hypr = install_1_0(&root);
        let mut state = HyprState::default();
        migrate_in(&hypr, &root.join("backups"), &mut state).unwrap();
        let after_first = state.clone();

        assert!(migrate_in(&hypr, &root.join("backups"), &mut state).unwrap().is_none());
        assert_eq!(state, after_first);
    }

    #[test]
    fn lua_leftovers_are_imported_and_disabled() {
        let root = scratch("lua");
        let hypr = root.join("hypr");
        fs::write(
            hypr.join("hyprland.lua"),
            r#"hl.config({ general = { gaps_in = 5 } })
hl.keyword("monitor", "DP-1,2560x1440@144.00Hz,0x0,1.25,transform,0")
hl.keyword("workspace", "1, monitor:DP-1, default:true")
hl.keyword("decoration:active_opacity", "0.93")

-- HyprDesk autostart
hl.keyword("exec-once", "/home/u/.config/hypr/hyprdesk-startup.sh")

-- HyprDesk per-app opacity overrides
dofile("/home/u/.config/hypr/hyprdesk-opacity.lua")
"#,
        )
        .unwrap();
        fs::write(hypr.join("UserConfigs/extra.lua"), "hl.keyword(\"decoration:inactive_opacity\", \"0.80\")\n").unwrap();
        fs::write(hypr.join("hyprdesk-opacity.lua"), "hl.keyword(\"windowrule\", \"match:class ^(kitty)$, opacity 0.95\")\n").unwrap();
        let mut state = HyprState::default();

        migrate_in(&hypr, &root.join("backups"), &mut state).unwrap().unwrap();

        assert_eq!(state.monitors[0].scale, 1.25);
        assert_eq!(state.primary.as_deref(), Some("DP-1"));
        assert_eq!((state.opacity_active, state.opacity_inactive), (Some(0.93), Some(0.80)));
        assert_eq!(state.app_opacity[0].app, "kitty");
        for file in ["hyprland.lua", "UserConfigs/extra.lua"] {
            let text = fs::read_to_string(hypr.join(file)).unwrap();
            assert!(!text.lines().any(|l| l.trim_start().starts_with("hl.keyword(")), "{file}:\n{text}");
            assert!(!text.contains("dofile") && !text.contains("hyprdesk-startup"), "{file}:\n{text}");
        }
        assert!(fs::read_to_string(hypr.join("hyprland.lua")).unwrap().starts_with("hl.config({ general = { gaps_in = 5 } })\n"));
    }

    #[test]
    fn only_the_appended_decoration_block_is_moved() {
        let root = scratch("block");
        let hypr = root.join("hypr");
        let conf = "decoration {\n  rounding = 10\n}\n\ndecoration {\n  active_opacity = 0.85\n}\n";
        fs::write(hypr.join("hyprland.conf"), conf).unwrap();
        let mut state = HyprState::default();

        migrate_in(&hypr, &root.join("backups"), &mut state).unwrap().unwrap();

        assert_eq!(state.opacity_active, Some(0.85));
        let text = fs::read_to_string(hypr.join("hyprland.conf")).unwrap();
        assert!(text.starts_with("decoration {\n  rounding = 10\n}\n"), "{text}");
        assert!(text.contains("# [HyprDesk → hyprdesk.conf]   active_opacity = 0.85\n"), "{text}");
    }

    #[test]
    fn lines_a_person_wrote_are_left_alone() {
        let root = scratch("user");
        let hypr = root.join("hypr");
        let conf = "monitor = DP-1, 1920x1080@60, 0x0, 1\nworkspace = 1, monitor:DP-1\nmonitor=DP-2,preferred,auto,1\n";
        fs::write(hypr.join("monitors.conf"), conf).unwrap();
        let mut state = HyprState::default();

        assert!(migrate_in(&hypr, &root.join("backups"), &mut state).unwrap().is_none());
        assert_eq!(fs::read_to_string(hypr.join("monitors.conf")).unwrap(), conf);
        assert_eq!(state, HyprState::default());
    }
}
