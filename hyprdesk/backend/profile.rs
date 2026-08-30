// Profile backend — manages display name, avatar (with crop) and system locale via AccountsService and localectl.
// Backend de perfil — gestiona nombre de usuario, avatar (con recorte) e idioma del sistema mediante AccountsService y localectl.

use crate::config::config_dir;
use std::fs;
use std::process::Command;
use std::path::Path;

pub fn get_username() -> String {
    std::env::var("USER").unwrap_or_else(|_| "user".into())
}

pub fn get_display_name() -> String {
    // Parse GECOS from /etc/passwd / Lee GECOS de /etc/passwd
    let username = get_username();
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 5 && parts[0] == username {
                let gecos = parts[4].split(',').next().unwrap_or("").trim();
                if !gecos.is_empty() { return gecos.to_string(); }
            }
        }
    }
    username
}

pub fn get_avatar_path() -> Option<String> {
    // Try AccountsService file first / Intenta primero el archivo AccountsService
    let username = get_username();
    let accounts_file = format!("/var/lib/AccountsService/users/{username}");
    if let Ok(content) = fs::read_to_string(&accounts_file) {
        for line in content.lines() {
            if let Some(path) = line.strip_prefix("Icon=") {
                let p = path.trim();
                if !p.is_empty() && std::path::Path::new(p).exists() {
                    return Some(p.to_string());
                }
            }
        }
    }
    // Fall back to local avatar / Recurre al avatar local
    let local = config_dir().join("avatars").join("avatar.png");
    if local.exists() { Some(local.to_string_lossy().into_owned()) } else { None }
}

pub fn set_display_name(name: &str) -> bool {
    let uid = libc_getuid();
    let obj_path = format!("/org/freedesktop/Accounts/User{uid}");
    let out = Command::new("gdbus")
        .args(["call", "--system",
               "--dest", "org.freedesktop.Accounts",
               "--object-path", &obj_path,
               "--method", "org.freedesktop.Accounts.User.SetRealName",
               &format!("\"{name}\"")])
        .output();
    if out.map(|o| o.status.success()).unwrap_or(false) {
        return true;
    }
    // Fallback: pkexec chfn / Alternativa: pkexec chfn
    Command::new("pkexec")
        .args(["chfn", "-f", name, &get_username()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn save_avatar_with_fractions(src: &Path, cx_frac: f32, cy_frac: f32, r_frac: f32) -> Option<String> {
    let data = fs::read(src).ok()?;
    let img = ::image::load_from_memory(&data).ok()?;
    let (orig_w, orig_h) = (img.width(), img.height());

    let cx = (cx_frac * orig_w as f32).clamp(0.0, orig_w as f32);
    let cy = (cy_frac * orig_h as f32).clamp(0.0, orig_h as f32);
    let r = (r_frac * orig_w.min(orig_h) as f32).max(1.0);

    let crop_dim = (2.0 * r) as u32;
    let x = (cx - r).max(0.0).min((orig_w.saturating_sub(crop_dim)) as f32) as u32;
    let y = (cy - r).max(0.0).min((orig_h.saturating_sub(crop_dim)) as f32) as u32;
    let w = crop_dim.min(orig_w - x);
    let h = crop_dim.min(orig_h - y);

    let rgba = img.to_rgba8();
    let sub = ::image::imageops::crop_imm(&rgba, x, y, w, h).to_image();
    let dyn_img: ::image::DynamicImage = sub.into();
    let resized = dyn_img.resize_to_fill(256, 256, ::image::imageops::FilterType::Lanczos3);

    let dest_dir = config_dir().join("avatars");
    let _ = fs::create_dir_all(&dest_dir);
    let dest = dest_dir.join("avatar.png");
    resized.save_with_format(&dest, ::image::ImageFormat::Png).ok()?;

    let dest_str = dest.to_string_lossy().into_owned();
    let uid = libc_getuid();
    let obj_path = format!("/org/freedesktop/Accounts/User{uid}");
    let _ = Command::new("gdbus")
        .args(["call", "--system",
               "--dest", "org.freedesktop.Accounts",
               "--object-path", &obj_path,
               "--method", "org.freedesktop.Accounts.User.SetIconFile",
               &format!("\"{dest_str}\"")])
        .output();
    Some(dest_str)
}

pub fn get_system_locales() -> Vec<String> {
    let Ok(out) = Command::new("localectl").arg("list-locales").output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn get_current_system_locale() -> String {
    if let Ok(out) = Command::new("localectl").arg("status").output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some(lang) = line.split("LANG=").nth(1) {
                return lang.trim().to_string();
            }
        }
    }
    std::env::var("LANG").unwrap_or_default()
}

pub fn set_system_locale(locale: &str) -> bool {
    Command::new("pkexec")
        .args(["localectl", "set-locale", &format!("LANG={locale}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn libc_getuid() -> u32 {
    std::process::Command::new("id").arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1000)
}
