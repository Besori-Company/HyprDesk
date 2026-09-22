// Live checks against a running Hyprland — run by hand only, pointed at a nested test instance.
// Comprobaciones en vivo contra un Hyprland en marcha — solo se lanzan a mano, apuntando a una instancia anidada de prueba.
//
// HYPRDESK_LIVE_HOME=<home> HOME=<home> XDG_RUNTIME_DIR=<rt> HYPRLAND_INSTANCE_SIGNATURE=<sig> \
//   cargo test live_ -- --ignored --nocapture --test-threads=1

use crate::backend::hyprconf::{self, Compositor, Provider};
use crate::backend::{display, migrate, monitors, opacity};
use crate::config::{hypr_dir, Config};
use std::fs;
use std::process::Command;
use std::time::Duration;

// Refuses to run unless the test home was named on purpose, so a real session is never touched / Se niega a correr si el home de prueba no se indicó a propósito, así nunca se toca una sesión real
fn guard() -> bool {
    std::env::var("HYPRDESK_LIVE_HOME").ok().is_some_and(|h| Some(h) == std::env::var("HOME").ok())
}

fn hyprctl(args: &[&str]) {
    let _ = Command::new("hyprctl").args(args).output();
}

// Lua autoreload happens after the write returns / El autoreload de Lua ocurre después de que vuelva la escritura
fn settle() {
    std::thread::sleep(Duration::from_millis(700));
}

fn errors() -> Vec<String> {
    hyprconf::Hyprctl.errors()
}

// After any change the file is exactly the saved state, never half of one / Tras cualquier cambio el fichero es justo el estado guardado, nunca medio
fn assert_file_matches_state(p: Provider) {
    let file = fs::read_to_string(hypr_dir().join(p.managed_name())).unwrap();
    assert_eq!(file, hyprconf::render(p, &hyprconf::load_state()));
}

#[test]
#[ignore]
fn live_full_round_trip() {
    if !guard() {
        eprintln!("skipped: HYPRDESK_LIVE_HOME must equal HOME");
        return;
    }
    let p = hyprconf::provider();
    println!("provider: {p:?}");

    let report = migrate::on_startup().expect("the test home carries 1.0.x leftovers");
    println!("migrated {} entries, backup in {}", report.moved, report.backup.display());
    assert!(migrate::on_startup().is_none());
    settle();
    let entry_path = hypr_dir().join(p.entry_name());
    assert!(hyprconf::has_hook(p, &fs::read_to_string(&entry_path).unwrap()));
    assert_file_matches_state(p);
    assert!(errors().is_empty(), "{:?}", errors());

    display::setup_autostart(&Config::default());
    assert!(hyprconf::load_state().autostart.is_some());
    assert_file_matches_state(p);

    opacity::set_opacity("active", 0.8).unwrap();
    settle();
    assert_eq!(hyprconf::get_float_option("decoration:active_opacity"), Some(0.8));
    assert_eq!(hyprconf::conflicts(), hyprconf::Conflicts::default());

    opacity::set_app_opacity("kitty", 0.7).unwrap();
    assert!(opacity::get_app_opacities().iter().any(|a| a.app == "kitty" && a.active == 0.7));
    opacity::remove_app_opacity("kitty").unwrap();
    assert!(!opacity::get_app_opacities().iter().any(|a| a.app == "kitty"));
    assert!(opacity::set_app_opacity("bad,class", 0.5).is_err());
    assert_file_matches_state(p);

    let mon = monitors::get_monitors().into_iter().next().expect("a monitor");
    let mode = monitors::current_mode(&mon);
    monitors::set_monitor_configs(vec![monitors::monitor_rule(&mon.name, &mode, 0, 0, 1.0, 0)]).unwrap();
    settle();
    let now = monitors::get_monitors().into_iter().find(|m| m.name == mon.name).unwrap();
    assert_eq!((now.x, now.y, now.scale, now.transform), (0, 0, 1.0, 0));

    monitors::set_primary_monitor(&mon.name).unwrap();
    settle();
    assert_eq!(monitors::get_primary_monitor_name().as_deref(), Some(mon.name.as_str()));

    // A value set after the include shows up as overridden / Un valor puesto tras el include aparece como pisado
    let original = fs::read_to_string(&entry_path).unwrap();
    let late = match p {
        Provider::Hyprlang => "decoration:active_opacity = 0.5",
        Provider::Lua => "hl.config({ decoration = { active_opacity = 0.5 } })",
    };
    fs::write(&entry_path, format!("{original}{late}\n")).unwrap();
    hyprctl(&["reload", "config-only"]);
    settle();
    assert!(hyprconf::conflicts().opacity_active);
    fs::write(&entry_path, &original).unwrap();
    hyprctl(&["reload", "config-only"]);
    settle();
    assert!(!hyprconf::conflicts().opacity_active);

    // Whatever Hyprland makes of a bad mode, file and state stay in step / Haga lo que haga Hyprland con un modo inválido, fichero y estado siguen a la par
    let before = hyprconf::load_state();
    let bad = monitors::set_monitor_configs(vec![monitors::monitor_rule(&mon.name, "notamode", 0, 0, 1.0, 0)]);
    println!("bad mode: {bad:?}");
    if bad.is_err() {
        assert_eq!(hyprconf::load_state(), before);
    }
    assert_file_matches_state(p);

    monitors::set_monitor_configs(vec![monitors::monitor_rule(&mon.name, &mode, 0, 0, 1.0, 0)]).unwrap();
    settle();
    assert!(errors().iter().all(|e| !e.contains(p.managed_name())), "{:?}", errors());
    println!("final file:\n{}", fs::read_to_string(hypr_dir().join(p.managed_name())).unwrap());
}
