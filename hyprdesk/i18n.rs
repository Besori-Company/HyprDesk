// Internationalization — provides translated strings for English and Spanish.
// Internacionalización — proporciona cadenas traducidas para inglés y español.

use std::collections::HashMap;
use std::sync::OnceLock;

static LANG: std::sync::OnceLock<std::sync::Mutex<String>> = OnceLock::new();

pub fn set_lang(lang: &str) {
    let m = LANG.get_or_init(|| std::sync::Mutex::new("en".into()));
    *m.lock().unwrap() = lang.to_string();
}

pub fn get_lang() -> String {
    LANG.get_or_init(|| std::sync::Mutex::new("en".into()))
        .lock()
        .unwrap()
        .clone()
}

pub fn t(key: &str) -> String {
    let lang = get_lang();
    strings(&lang)
        .get(key)
        .or_else(|| strings("en").get(key))
        .map(|s| s.to_string())
        .unwrap_or_else(|| key.to_string())
}

fn strings(lang: &str) -> &'static HashMap<&'static str, &'static str> {
    static EN: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    static ES: OnceLock<HashMap<&str, &str>> = OnceLock::new();

    match lang {
        "es" => ES.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("nav_brightness", "Brillo");
            m.insert("nav_night", "Modo noche");
            m.insert("nav_monitors", "Monitores");
            m.insert("nav_opacity", "Opacidad");
            m.insert("nav_profile", "Perfil");
            m.insert("title_brightness", "Brillo");
            m.insert("title_night", "Modo noche");
            m.insert("title_monitors", "Monitores");
            m.insert("title_opacity", "Opacidad de ventanas");
            m.insert("title_profile", "Perfil");
            m.insert("section_main_display", "Pantalla principal");
            m.insert("row_brightness", "Brillo de pantalla");
            m.insert("section_autostart", "Inicio automático");
            m.insert("row_restore_startup", "Restaurar al iniciar");
            m.insert("row_restore_startup_sub", "exec-once en hyprland.conf — se aplica en cada arranque");
            m.insert("chip_active", "Activo");
            m.insert("section_night_light", "Luz nocturna");
            m.insert("row_enable_night", "Activar modo noche");
            m.insert("row_enable_night_sub", "Reduce la temperatura de color por las noches");
            m.insert("row_color_temp", "Temperatura de color");
            m.insert("row_color_temp_sub", "Más cálido | Más alto = Más frío");
            m.insert("section_monitor_select", "Seleccionar monitor");
            m.insert("section_resolution", "Resolución y frecuencia");
            m.insert("section_position", "Posición");
            m.insert("row_position_sub", "Coordenadas en píxeles desde la esquina superior izquierda");
            m.insert("row_pos_x", "X");
            m.insert("row_pos_y", "Y");
            m.insert("section_orientation", "Orientación");
            m.insert("section_scale", "Escala");
            m.insert("btn_apply_monitor", "Aplicar");
            m.insert("toast_monitor_applied", "Configuración de monitor aplicada");
            m.insert("toast_monitor_failed", "No se pudo aplicar la configuración");
            m.insert("monitor_no_signal", "No se detectaron monitores");
            m.insert("section_primary", "Monitor principal");
            m.insert("row_primary", "Monitor principal");
            m.insert("row_primary_sub", "El espacio de trabajo 1 se asigna a este monitor");
            m.insert("btn_set_primary", "Establecer como principal");
            m.insert("chip_primary", "Principal");
            m.insert("toast_primary_set", "Monitor establecido como principal");
            m.insert("section_global_opacity", "Opacidad global");
            m.insert("row_active_opacity", "Ventana activa");
            m.insert("row_active_opacity_sub", "Opacidad cuando la ventana tiene el foco");
            m.insert("row_inactive_opacity", "Ventana inactiva");
            m.insert("row_inactive_opacity_sub", "Opacidad cuando la ventana no tiene el foco");
            m.insert("section_app_overrides", "Overrides por app");
            m.insert("add_override_open_apps", "Apps abiertas");
            m.insert("add_override_no_open", "No se detectaron ventanas abiertas");
            m.insert("add_override_all_added", "Todas las apps abiertas ya tienen override");
            m.insert("add_override_custom_placeholder", "o escribe una clase manualmente…");
            m.insert("btn_refresh", "Actualizar");
            m.insert("btn_add", "Añadir");
            m.insert("btn_remove", "Quitar");
            m.insert("toast_opacity_updated", "Opacidad aplicada");
            m.insert("toast_opacity_failed", "No se pudo escribir la configuración");
            m.insert("toast_override_added", "Override añadido para {}");
            m.insert("toast_override_removed", "Override eliminado para {}");
            m.insert("toast_override_failed", "No se pudo escribir el override");
            m.insert("section_photo", "Foto de perfil");
            m.insert("btn_change_photo", "Cambiar foto");
            m.insert("section_user_info", "Información de usuario");
            m.insert("row_display_name", "Nombre mostrado");
            m.insert("row_display_name_sub", "Visible en el entorno de escritorio");
            m.insert("btn_apply", "Aplicar");
            m.insert("section_language", "Idioma");
            m.insert("row_app_language", "Idioma de la app");
            m.insert("row_app_language_sub", "Reinicia para aplicar");
            m.insert("row_system_language", "Idioma del sistema");
            m.insert("row_system_language_sub", "Cambia el locale del sistema");
            m.insert("toast_photo_updated", "Foto de perfil actualizada");
            m.insert("toast_photo_failed", "No se pudo cambiar la foto.");
            m.insert("toast_name_updated", "Nombre actualizado: {}");
            m.insert("toast_name_failed", "No se pudo cambiar el nombre.");
            m.insert("toast_locale_updated", "Idioma actualizado. Cierra sesión para aplicar.");
            m.insert("toast_locale_failed", "No se pudo cambiar el idioma del sistema.");
            m.insert("dialog_crop_title", "Ajustar foto de perfil");
            m.insert("crop_zoom", "Zoom");
            m.insert("crop_pos_x", "Pos X");
            m.insert("crop_pos_y", "Pos Y");
            m.insert("btn_cancel", "Cancelar");
            m.insert("dialog_pick_title", "Seleccionar foto de perfil");
            m.insert("filter_images", "Imágenes");
            m.insert("localectl_unavailable", "localectl no disponible");
            m.insert("row_install_brightness", "Instala hyprsunset para control de brillo");
            m.insert("row_install_brightness_sub", "Arch: sudo pacman -S hyprsunset\nFedora: sudo dnf install wlsunset");
            m.insert("row_tool_missing", "Herramienta no encontrada");
            m.insert("row_tool_missing_sub", "Instala hyprsunset o wlsunset para activar esta función.");
            m.insert("row_brightnessctl_missing_sub", "Instala brightnessctl para controlar el brillo.");
            m.insert("row_hyprctl_missing_sub", "Instala Hyprland para activar esta función.");
            m.insert("active_method", "Método activo: {}");
            m.insert("dialog_keep_heading", "¿Mantener esta configuración?");
            m.insert("dialog_keep_body", "Revirtiendo en {} segundos...");
            m.insert("btn_keep", "Mantener");
            m.insert("btn_revert", "Revertir");
            m.insert("confirm_monitor", "¿Mantener la configuración de pantalla?");
            m.insert("confirm_brightness", "¿Mantener el brillo?");
            m.insert("confirm_night_temp", "¿Mantener la temperatura?");
            m.insert("confirm_opacity_active", "¿Mantener opacidad activa?");
            m.insert("confirm_opacity_inactive", "¿Mantener opacidad inactiva?");
            m.insert("confirm_app_opacity", "¿Mantener opacidad de {}?");
            m.insert("toast_monitor_reverted", "Configuración revertida");
            m.insert("update_available", "Actualizar a v{}");
            m.insert("confirm_pending", "¿Mantener todos los cambios?");
            m
        }),
        _ => EN.get_or_init(|| {
            let mut m = HashMap::new();
            m.insert("nav_brightness", "Brightness");
            m.insert("nav_night", "Night mode");
            m.insert("nav_monitors", "Monitors");
            m.insert("nav_opacity", "Opacity");
            m.insert("nav_profile", "Profile");
            m.insert("title_brightness", "Brightness");
            m.insert("title_night", "Night mode");
            m.insert("title_monitors", "Monitors");
            m.insert("title_opacity", "Window Opacity");
            m.insert("title_profile", "Profile");
            m.insert("section_main_display", "Main display");
            m.insert("row_brightness", "Screen brightness");
            m.insert("section_autostart", "Autostart");
            m.insert("row_restore_startup", "Restore on startup");
            m.insert("row_restore_startup_sub", "exec-once in hyprland.conf — applied on each boot");
            m.insert("chip_active", "Active");
            m.insert("section_night_light", "Night light");
            m.insert("row_enable_night", "Enable night mode");
            m.insert("row_enable_night_sub", "Reduces color temperature at night");
            m.insert("row_color_temp", "Color temperature");
            m.insert("row_color_temp_sub", "Lower = Warmer | Higher = Colder");
            m.insert("section_monitor_select", "Select monitor");
            m.insert("section_resolution", "Resolution & refresh rate");
            m.insert("section_position", "Position");
            m.insert("row_position_sub", "Coordinates in pixels from the top-left corner");
            m.insert("row_pos_x", "X");
            m.insert("row_pos_y", "Y");
            m.insert("section_orientation", "Orientation");
            m.insert("section_scale", "Scale");
            m.insert("btn_apply_monitor", "Apply");
            m.insert("toast_monitor_applied", "Monitor settings applied");
            m.insert("toast_monitor_failed", "Could not apply monitor settings");
            m.insert("monitor_no_signal", "No monitors detected");
            m.insert("section_primary", "Primary monitor");
            m.insert("row_primary", "Primary monitor");
            m.insert("row_primary_sub", "Workspace 1 is assigned to this monitor");
            m.insert("btn_set_primary", "Set as primary");
            m.insert("chip_primary", "Primary");
            m.insert("toast_primary_set", "Set as primary monitor");
            m.insert("section_global_opacity", "Global opacity");
            m.insert("row_active_opacity", "Active window");
            m.insert("row_active_opacity_sub", "Opacity when the window has focus");
            m.insert("row_inactive_opacity", "Inactive window");
            m.insert("row_inactive_opacity_sub", "Opacity when the window is unfocused");
            m.insert("section_app_overrides", "Per-app overrides");
            m.insert("add_override_open_apps", "Open apps");
            m.insert("add_override_no_open", "No open windows detected");
            m.insert("add_override_all_added", "All open apps already have an override");
            m.insert("add_override_custom_placeholder", "or type a class manually…");
            m.insert("btn_refresh", "Refresh");
            m.insert("btn_add", "Add");
            m.insert("btn_remove", "Remove");
            m.insert("toast_opacity_updated", "Opacity applied");
            m.insert("toast_opacity_failed", "Could not write opacity config");
            m.insert("toast_override_added", "Override added for {}");
            m.insert("toast_override_removed", "Override removed for {}");
            m.insert("toast_override_failed", "Could not write override");
            m.insert("section_photo", "Profile photo");
            m.insert("btn_change_photo", "Change photo");
            m.insert("section_user_info", "User information");
            m.insert("row_display_name", "Display name");
            m.insert("row_display_name_sub", "Visible in the desktop environment");
            m.insert("btn_apply", "Apply");
            m.insert("section_language", "Language");
            m.insert("row_app_language", "App language");
            m.insert("row_app_language_sub", "Restart to apply");
            m.insert("row_system_language", "System language");
            m.insert("row_system_language_sub", "Changes the system locale");
            m.insert("toast_photo_updated", "Profile photo updated");
            m.insert("toast_photo_failed", "Could not change the photo.");
            m.insert("toast_name_updated", "Name updated: {}");
            m.insert("toast_name_failed", "Could not change the name.");
            m.insert("toast_locale_updated", "Language updated. Log out to apply.");
            m.insert("toast_locale_failed", "Could not change the system language.");
            m.insert("dialog_crop_title", "Adjust profile photo");
            m.insert("crop_zoom", "Zoom");
            m.insert("crop_pos_x", "Pos X");
            m.insert("crop_pos_y", "Pos Y");
            m.insert("btn_cancel", "Cancel");
            m.insert("dialog_pick_title", "Select profile photo");
            m.insert("filter_images", "Images");
            m.insert("localectl_unavailable", "localectl not available");
            m.insert("row_install_brightness", "Install hyprsunset for brightness control");
            m.insert("row_install_brightness_sub", "Arch: sudo pacman -S hyprsunset\nFedora: sudo dnf install wlsunset");
            m.insert("row_tool_missing", "Tool not found");
            m.insert("row_tool_missing_sub", "Install hyprsunset or wlsunset to enable this feature.");
            m.insert("row_brightnessctl_missing_sub", "Install brightnessctl to control brightness.");
            m.insert("row_hyprctl_missing_sub", "Install Hyprland to enable this feature.");
            m.insert("active_method", "Active method: {}");
            m.insert("dialog_keep_heading", "Keep this configuration?");
            m.insert("dialog_keep_body", "Reverting in {} seconds...");
            m.insert("btn_keep", "Keep");
            m.insert("btn_revert", "Revert");
            m.insert("confirm_monitor", "Keep display configuration?");
            m.insert("confirm_brightness", "Keep brightness?");
            m.insert("confirm_night_temp", "Keep temperature?");
            m.insert("confirm_opacity_active", "Keep active opacity?");
            m.insert("confirm_opacity_inactive", "Keep inactive opacity?");
            m.insert("confirm_app_opacity", "Keep opacity for {}?");
            m.insert("toast_monitor_reverted", "Configuration reverted");
            m.insert("update_available", "Update to v{}");
            m.insert("confirm_pending", "Keep all changes?");
            m
        }),
    }
}
