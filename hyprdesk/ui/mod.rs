// Main UI module — defines app state, messages, update loop, view and sidebar.
// Módulo UI principal — define el estado de la app, mensajes, bucle de actualización, vista y barra lateral.

mod brightness;
mod monitors;
mod night;
mod opacity;
mod profile;
pub mod theme;
pub mod widgets;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{
    button, column, container, image, row, rule, scrollable, space, svg, text,
};
use iced::{
    self, Alignment, Color, Element, Length, Size, Subscription, Task, Theme,
};

use crate::backend::monitors::Monitor;
use crate::backend::opacity::AppOpacity;
use crate::backend::{display, monitors as mon_backend, opacity as op_backend, profile as prof_backend};
use crate::config::{self, Config};
use crate::i18n::t;
use theme::*;

// ── Page / Página ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Brightness,
    Night,
    Monitors,
    Opacity,
    Profile,
}

// ── Value confirm state (brightness / night temp / opacity) / Estado de confirmación de valor ──

#[derive(Debug, Clone)]
pub struct ValueConfirm {
    pub old_value: u32,
}

#[derive(Debug, Clone)]
pub struct AppOpacityConfirm {
    pub app: String,
    pub old_value: f64,
}

// ── Photo crop state / Estado de recorte de foto ─────────────

pub struct PhotoCropState {
    pub path: PathBuf,
    pub source_rgba: ::image::RgbaImage,
    pub img_w: u32,
    pub img_h: u32,
    pub zoom: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub drag_active: bool,
    pub drag_last: Option<(f32, f32)>,
    pub preview: Option<iced::widget::image::Handle>,
}

// ── Monitor confirm state / Estado de confirmación de monitor ─

#[derive(Debug, Clone)]
pub struct MonitorConfirm {
    pub monitor_name: String,
    pub old_mode: String,
    pub old_x: i32,
    pub old_y: i32,
    pub old_scale: f64,
    pub old_transform: u32,
}

// ── App / Aplicación ─────────────────────────────────────────

pub struct App {
    pub page: Page,
    pub config: Config,
    pub toast: Option<String>,
    pub brand_handle: image::Handle,

    // Brightness / Brillo
    pub brightness: u32,
    pub brightness_gen: u64,
    pub brightness_available: bool,
    pub brightness_method: String,
    pub brightness_confirm: Option<ValueConfirm>,

    // Night / Modo noche
    pub night_mode: bool,
    pub night_temp: u32,
    pub night_temp_committed: u32,
    pub night_gen: u64,
    pub night_available: bool,
    pub night_temp_confirm: Option<ValueConfirm>,

    // Monitors / Monitores
    pub monitors: Vec<Monitor>,
    pub selected_monitor: usize,
    pub monitor_positions: HashMap<String, (i32, i32)>,
    pub monitor_res_mode: String,
    pub monitor_scale_idx: usize,
    pub monitor_transform_idx: usize,
    pub monitor_pos_x: String,
    pub monitor_pos_y: String,
    pub monitor_confirm: Option<MonitorConfirm>,

    // Opacity / Opacidad
    pub opacity_available: bool,
    pub opacity_active: f64,
    pub opacity_inactive: f64,
    pub opacity_active_gen: u64,
    pub opacity_inactive_gen: u64,
    pub opacity_active_confirm: Option<ValueConfirm>,
    pub opacity_inactive_confirm: Option<ValueConfirm>,
    pub app_opacities: Vec<AppOpacity>,
    pub app_opacity_committed: HashMap<String, f64>,
    pub app_opacity_confirm: Option<AppOpacityConfirm>,
    pub confirm_seconds: Option<u32>,
    pub open_window_classes: Vec<String>,
    pub open_window_total: usize,
    pub opacity_custom_input: String,
    pub opacity_selected_class: Option<String>,

    // Update / Actualización
    pub update_available: Option<String>,

    // Profile / Perfil
    pub display_name: String,
    pub display_name_input: String,
    pub username: String,
    pub avatar_path: Option<PathBuf>,
    pub avatar_handle: Option<iced::widget::image::Handle>,
    pub system_locales: Vec<String>,
    pub locale_idx: usize,
    pub lang_idx: usize,
    pub photo_crop: Option<PhotoCropState>,
}

// ── Message / Mensaje ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
    ClearToast,

    // Brightness / Brillo
    BrightnessChanged(f64),
    BrightnessApply(u64, u32),

    // Night / Modo noche
    NightModeToggled(bool),
    NightTempChanged(f64),
    NightTempApply(u64, u32),

    // Monitors / Monitores
    MonitorCanvasSelected(String),
    MonitorMoved(String, i32, i32),
    MonitorSelected(usize),
    MonitorResChanged(String),
    MonitorScaleChanged(usize),
    MonitorTransformChanged(usize),
    MonitorPosXChanged(String),
    MonitorPosYChanged(String),
    MonitorApply,
    MonitorSetPrimary,

    // Shared confirm / Confirmación compartida
    ConfirmTick,
    ConfirmKeep,
    ConfirmRevert,

    // Opacity / Opacidad
    OpacityActiveChanged(f64),
    OpacityActiveApply(u64, f64),
    OpacityInactiveChanged(f64),
    OpacityInactiveApply(u64, f64),
    AppOpacityChanged(String, f64),
    AppOpacityApply(String, u64, f64),
    AddAppOverride,
    RemoveAppOverride(String),
    OpacityCustomChanged(String),
    OpacityClassSelected(String),
    RefreshOpenWindows,

    // Update / Actualización
    UpdateChecked(Option<String>),
    OpenUpdateLink,
    DismissUpdate,

    // Profile / Perfil
    DisplayNameInputChanged(String),
    ApplyDisplayName,
    ChangePhoto,
    PhotoPicked(Option<PathBuf>),
    PhotoCropZoomChanged(f64),
    PhotoCropDragStart,
    PhotoCropDragMove(f32, f32),
    PhotoCropDragEnd,
    PhotoCropConfirm,
    PhotoCropCancel,
    LanguageChanged(usize),
    LocaleChanged(usize),
    ApplyLocale,
}

// ── Init / Inicialización ────────────────────────────────────

impl App {
    // Saves the config and refreshes the startup script, so a reboot restores these values / Guarda la configuración y refresca el script de arranque, para que un reinicio restaure estos valores
    fn persist(&self) {
        config::save_config(&self.config);
        display::setup_autostart(&self.config);
    }

    pub fn new() -> (Self, Task<Message>) {
        let config = config::load_config();
        crate::i18n::set_lang(&config.app_lang);

        // Keep the startup script in step with the saved values / Mantiene el script de arranque al día con los valores guardados
        display::setup_autostart(&config);

        let (method_name, available) = display::brightness_method();
        let brightness = if available {
            display::get_brightness(&config)
        } else {
            config.brightness
        };

        let monitors = mon_backend::get_monitors();
        let positions: HashMap<_, _> = monitors
            .iter()
            .map(|m| (m.name.clone(), (m.x, m.y)))
            .collect();
        let (res_mode, scale_idx, transform_idx, pos_x, pos_y) =
            if let Some(m) = monitors.first() {
                (
                    mon_backend::current_mode(m),
                    mon_backend::closest_scale_idx(m.scale) as usize,
                    m.transform as usize,
                    m.x.to_string(),
                    m.y.to_string(),
                )
            } else {
                (String::new(), 2, 0, "0".into(), "0".into())
            };

        let ops = op_backend::get_opacities();
        let app_opacities = op_backend::get_app_opacities();
        let mut open_window_classes = op_backend::get_open_window_classes();
        let open_window_total = open_window_classes.len();
        open_window_classes.retain(|c| !app_opacities.iter().any(|e| e.app == *c));

        let display_name = prof_backend::get_display_name();
        let username = prof_backend::get_username();
        let avatar_path = prof_backend::get_avatar_path().map(PathBuf::from);
        let system_locales = prof_backend::get_system_locales();
        let current_locale = prof_backend::get_current_system_locale();
        let locale_idx = system_locales
            .iter()
            .position(|l| *l == current_locale)
            .unwrap_or(0);
        let lang_idx = if crate::i18n::get_lang() == "es" { 1 } else { 0 };

        let night_mode = config.night_mode;
        let night_temp = config.night_temp;

        let app = App {
            page: Page::Brightness,
            config,
            toast: None,
            brand_handle: image::Handle::from_bytes(
                include_bytes!("../assets/icons/hyprdesk-ico.png").to_vec()
            ),

            brightness,
            brightness_gen: 0,
            brightness_available: available,
            brightness_method: method_name.to_string(),
            brightness_confirm: None,

            night_mode,
            night_temp,
            night_temp_committed: night_temp,
            night_gen: 0,
            night_available: display::night_tool_available(),
            night_temp_confirm: None,

            monitors,
            selected_monitor: 0,
            monitor_positions: positions,
            monitor_res_mode: res_mode,
            monitor_scale_idx: scale_idx,
            monitor_transform_idx: transform_idx,
            monitor_pos_x: pos_x,
            monitor_pos_y: pos_y,
            monitor_confirm: None,

            opacity_available: op_backend::hyprctl_available(),
            opacity_active: ops.active,
            opacity_inactive: ops.inactive,
            opacity_active_gen: 0,
            opacity_inactive_gen: 0,
            opacity_active_confirm: None,
            opacity_inactive_confirm: None,
            app_opacity_committed: app_opacities.iter().map(|e| (e.app.clone(), e.active)).collect(),
            app_opacity_confirm: None,
            confirm_seconds: None,
            app_opacities,
            open_window_classes,
            open_window_total,
            opacity_custom_input: String::new(),
            opacity_selected_class: None,

            update_available: None,

            display_name: display_name.clone(),
            display_name_input: display_name,
            username,
            avatar_handle: avatar_path.as_deref().and_then(profile::make_circular_handle),
            avatar_path,
            system_locales,
            locale_idx,
            lang_idx,
            photo_crop: None,
        };

        let update_task = Task::perform(
            crate::backend::update::check_latest_version(),
            Message::UpdateChecked,
        );
        (app, update_task)
    }

    // ── Update / Actualización ───────────────────────────────

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Navigate(p) => {
                self.page = p;
                Task::none()
            }
            Message::ClearToast => {
                self.toast = None;
                Task::none()
            }

            // Brightness / Brillo
            Message::BrightnessChanged(v) => {
                self.brightness = v as u32;
                let stamp = self.brightness_gen.wrapping_add(1);
                self.brightness_gen = stamp;
                let config = self.config.clone();
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(120)).await;
                        (stamp, v as u32, config)
                    },
                    |(stamp, v, _)| Message::BrightnessApply(stamp, v),
                )
            }
            Message::BrightnessApply(stamp, v) => {
                if stamp == self.brightness_gen {
                    let old = self.config.brightness;
                    display::set_brightness(v, &self.config);
                    self.config.brightness = v;
                    self.persist();
                    if self.brightness_confirm.is_none() {
                        self.brightness_confirm = Some(ValueConfirm { old_value: old });
                    }
                    self.confirm_seconds = Some(15);
                }
                Task::none()
            }

            // Night / Modo noche
            Message::NightModeToggled(enabled) => {
                self.night_mode = enabled;
                self.config.night_mode = enabled;
                let temp = self.night_temp;
                let brightness = self.brightness;
                display::apply_night_mode(enabled, temp, brightness);
                self.persist();
                Task::none()
            }
            Message::NightTempChanged(v) => {
                self.night_temp = v as u32;
                let stamp = self.night_gen.wrapping_add(1);
                self.night_gen = stamp;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(400)).await;
                        (stamp, v as u32)
                    },
                    |(stamp, v)| Message::NightTempApply(stamp, v),
                )
            }
            Message::NightTempApply(stamp, v) => {
                if stamp == self.night_gen {
                    let old = self.night_temp_committed;
                    display::apply_night_mode(self.night_mode, v, self.brightness);
                    self.night_temp_committed = v;
                    self.config.night_temp = v;
                    self.persist();
                    if self.night_temp_confirm.is_none() {
                        self.night_temp_confirm = Some(ValueConfirm { old_value: old });
                    }
                    self.confirm_seconds = Some(15);
                }
                Task::none()
            }

            // Monitors / Monitores — canvas interaction / Monitores — interacción con el canvas
            Message::MonitorCanvasSelected(name) => {
                if let Some(idx) = self.monitors.iter().position(|m| m.name == name) {
                    self.update(Message::MonitorSelected(idx))
                } else {
                    Task::none()
                }
            }
            Message::MonitorMoved(name, x, y) => {
                self.monitor_positions.insert(name.clone(), (x, y));
                if let Some(idx) = self.monitors.iter().position(|m| m.name == name) {
                    if idx == self.selected_monitor {
                        self.monitor_pos_x = x.to_string();
                        self.monitor_pos_y = y.to_string();
                    }
                }
                Task::none()
            }
            Message::MonitorSelected(idx) => {
                self.selected_monitor = idx;
                if let Some(m) = self.monitors.get(idx) {
                    self.monitor_res_mode = mon_backend::current_mode(m);
                    self.monitor_scale_idx = mon_backend::closest_scale_idx(m.scale) as usize;
                    self.monitor_transform_idx = m.transform as usize;
                    let (px, py) = self.monitor_positions.get(&m.name).copied().unwrap_or((m.x, m.y));
                    self.monitor_pos_x = px.to_string();
                    self.monitor_pos_y = py.to_string();
                }
                Task::none()
            }
            Message::MonitorResChanged(mode) => {
                self.monitor_res_mode = mode;
                Task::none()
            }
            Message::MonitorScaleChanged(idx) => {
                self.monitor_scale_idx = idx;
                Task::none()
            }
            Message::MonitorTransformChanged(idx) => {
                self.monitor_transform_idx = idx;
                Task::none()
            }
            Message::MonitorPosXChanged(s) => {
                self.monitor_pos_x = numeric_coord(&s);
                Task::none()
            }
            Message::MonitorPosYChanged(s) => {
                self.monitor_pos_y = numeric_coord(&s);
                Task::none()
            }
            Message::MonitorApply => {
                if let Some(mon) = self.monitors.get(self.selected_monitor) {
                    let name = mon.name.clone();
                    let old_mode = mon_backend::current_mode(mon);
                    let old_x = mon.x;
                    let old_y = mon.y;
                    let old_scale = mon.scale;
                    let old_transform = mon.transform as u32;

                    let new_x = self.monitor_pos_x.parse::<i32>().unwrap_or(old_x);
                    let new_y = self.monitor_pos_y.parse::<i32>().unwrap_or(old_y);
                    let new_scale =
                        mon_backend::SCALE_PRESETS[self.monitor_scale_idx.min(
                            mon_backend::SCALE_PRESETS.len().saturating_sub(1),
                        )];
                    let new_transform = self.monitor_transform_idx as u32;

                    // Measure the monitor as it will be once applied, not as it is now / Medir el monitor tal como quedará al aplicar, no como está ahora
                    let mut probe = self.monitors.clone();
                    if let Some(p) = probe.get_mut(self.selected_monitor) {
                        if let Some((w, h)) = mon_backend::mode_size(&self.monitor_res_mode) {
                            p.width = w;
                            p.height = h;
                        }
                        p.scale = new_scale;
                        p.transform = new_transform as i32;
                    }
                    let clashes = mon_backend::overlapping(
                        &probe,
                        &self.monitor_positions,
                        &name,
                        new_x,
                        new_y,
                    );
                    if !clashes.is_empty() {
                        self.toast = Some(t("toast_monitor_overlap").replace("{}", &clashes.join(", ")));
                        return Task::none();
                    }

                    // Save other monitors' state before applying changes / Guardar el estado de los otros monitores antes de aplicar cambios
                    let others: Vec<(String, String, i32, i32, f64, u32)> = self.monitors.iter()
                        .filter(|m| m.name != name)
                        .map(|m| {
                            let (ox, oy) = self.monitor_positions.get(&m.name).copied().unwrap_or((m.x, m.y));
                            (m.name.clone(), mon_backend::current_mode(m), ox, oy, m.scale, m.transform as u32)
                        })
                        .collect();

                    if mon_backend::set_monitor_config(
                        &name,
                        &self.monitor_res_mode,
                        new_x,
                        new_y,
                        new_scale,
                        new_transform,
                    ) {
                        self.monitor_positions.insert(name.clone(), (new_x, new_y));

                        // Keep all monitors in sync in the config file / Mantener todos los monitores sincronizados en el archivo de configuración
                        for (oname, omode, ox, oy, oscale, otransform) in others {
                            let _ = mon_backend::set_monitor_config(&oname, &omode, ox, oy, oscale, otransform);
                        }

                        self.monitor_confirm = Some(MonitorConfirm {
                            monitor_name: name,
                            old_mode,
                            old_x,
                            old_y,
                            old_scale,
                            old_transform,
                        });
                        self.confirm_seconds = Some(15);
                    } else {
                        self.toast = Some(t("toast_monitor_failed"));
                    }
                }
                Task::none()
            }
            Message::MonitorSetPrimary => {
                if let Some(mon) = self.monitors.get(self.selected_monitor) {
                    let name = mon.name.clone();
                    if mon_backend::set_primary_monitor(&name) {
                        self.toast = Some(t("toast_primary_set"));
                    } else {
                        self.toast = Some(t("toast_monitor_failed"));
                    }
                }
                Task::none()
            }

            // Opacity / Opacidad
            Message::OpacityActiveChanged(v) => {
                self.opacity_active = v / 100.0;
                let stamp = self.opacity_active_gen.wrapping_add(1);
                self.opacity_active_gen = stamp;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(120)).await;
                        (stamp, v / 100.0)
                    },
                    |(stamp, v)| Message::OpacityActiveApply(stamp, v),
                )
            }
            Message::OpacityActiveApply(stamp, v) => {
                if stamp == self.opacity_active_gen {
                    let ok = op_backend::set_opacity("active", v);
                    if ok {
                        let old = (self.config.opacity_active * 100.0) as u32;
                        self.config.opacity_active = v;
                        config::save_config(&self.config);
                        if self.opacity_active_confirm.is_none() {
                            self.opacity_active_confirm = Some(ValueConfirm { old_value: old });
                        }
                        self.confirm_seconds = Some(15);
                    } else {
                        self.toast = Some(t("toast_opacity_failed"));
                    }
                }
                Task::none()
            }
            Message::OpacityInactiveChanged(v) => {
                self.opacity_inactive = v / 100.0;
                let stamp = self.opacity_inactive_gen.wrapping_add(1);
                self.opacity_inactive_gen = stamp;
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(120)).await;
                        (stamp, v / 100.0)
                    },
                    |(stamp, v)| Message::OpacityInactiveApply(stamp, v),
                )
            }
            Message::OpacityInactiveApply(stamp, v) => {
                if stamp == self.opacity_inactive_gen {
                    let ok = op_backend::set_opacity("inactive", v);
                    if ok {
                        let old = (self.config.opacity_inactive * 100.0) as u32;
                        self.config.opacity_inactive = v;
                        config::save_config(&self.config);
                        if self.opacity_inactive_confirm.is_none() {
                            self.opacity_inactive_confirm = Some(ValueConfirm { old_value: old });
                        }
                        self.confirm_seconds = Some(15);
                    } else {
                        self.toast = Some(t("toast_opacity_failed"));
                    }
                }
                Task::none()
            }
            Message::AppOpacityChanged(app, v) => {
                if let Some(entry) = self.app_opacities.iter_mut().find(|e| e.app == app) {
                    entry.active = v / 100.0;
                }
                let stamp = self.opacity_active_gen.wrapping_add(1);
                self.opacity_active_gen = stamp;
                let app_c = app.clone();
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_millis(120)).await;
                        (app_c, stamp, v / 100.0)
                    },
                    |(app, stamp, v)| Message::AppOpacityApply(app, stamp, v),
                )
            }
            Message::AppOpacityApply(app, stamp, v) => {
                if stamp == self.opacity_active_gen {
                    op_backend::set_app_opacity(&app, v);
                    let old = *self.app_opacity_committed.get(&app).unwrap_or(&v);
                    self.app_opacity_committed.insert(app.clone(), v);
                    match &self.app_opacity_confirm {
                        Some(c) if c.app == app => {}
                        _ => {
                            self.app_opacity_confirm = Some(AppOpacityConfirm { app: app.clone(), old_value: old });
                        }
                    }
                    self.confirm_seconds = Some(15);
                }
                Task::none()
            }
            Message::ConfirmTick => {
                if let Some(ref mut s) = self.confirm_seconds {
                    if *s > 0 { *s -= 1; }
                    if *s == 0 {
                        return self.update(Message::ConfirmRevert);
                    }
                }
                Task::none()
            }
            Message::ConfirmKeep => {
                self.brightness_confirm = None;
                self.night_temp_confirm = None;
                self.opacity_active_confirm = None;
                self.opacity_inactive_confirm = None;
                self.app_opacity_confirm = None;
                if self.monitor_confirm.take().is_some() {
                    self.toast = Some(t("toast_monitor_applied"));
                }
                self.confirm_seconds = None;
                Task::none()
            }
            Message::ConfirmRevert => {
                if let Some(c) = self.brightness_confirm.take() {
                    self.brightness = c.old_value;
                    display::set_brightness(c.old_value, &self.config);
                    self.config.brightness = c.old_value;
                    self.persist();
                }
                if let Some(c) = self.night_temp_confirm.take() {
                    self.night_temp = c.old_value;
                    self.night_temp_committed = c.old_value;
                    display::apply_night_mode(self.night_mode, c.old_value, self.brightness);
                    self.config.night_temp = c.old_value;
                    self.persist();
                }
                if let Some(c) = self.opacity_active_confirm.take() {
                    let v = c.old_value as f64 / 100.0;
                    op_backend::set_opacity("active", v);
                    self.opacity_active = v;
                    self.config.opacity_active = v;
                    config::save_config(&self.config);
                }
                if let Some(c) = self.opacity_inactive_confirm.take() {
                    let v = c.old_value as f64 / 100.0;
                    op_backend::set_opacity("inactive", v);
                    self.opacity_inactive = v;
                    self.config.opacity_inactive = v;
                    config::save_config(&self.config);
                }
                if let Some(c) = self.app_opacity_confirm.take() {
                    op_backend::set_app_opacity(&c.app, c.old_value);
                    self.app_opacity_committed.insert(c.app.clone(), c.old_value);
                    if let Some(entry) = self.app_opacities.iter_mut().find(|e| e.app == c.app) {
                        entry.active = c.old_value;
                    }
                }
                if let Some(c) = self.monitor_confirm.take() {
                    mon_backend::set_monitor_config(
                        &c.monitor_name,
                        &c.old_mode,
                        c.old_x,
                        c.old_y,
                        c.old_scale,
                        c.old_transform,
                    );
                    self.monitor_positions.insert(c.monitor_name.clone(), (c.old_x, c.old_y));
                    if let Some(idx) = self.monitors.iter().position(|m| m.name == c.monitor_name) {
                        if idx == self.selected_monitor {
                            self.monitor_res_mode = c.old_mode.clone();
                            self.monitor_scale_idx = mon_backend::closest_scale_idx(c.old_scale) as usize;
                            self.monitor_transform_idx = c.old_transform as usize;
                            self.monitor_pos_x = c.old_x.to_string();
                            self.monitor_pos_y = c.old_y.to_string();
                        }
                    }
                    self.toast = Some(t("toast_monitor_reverted"));
                }
                self.confirm_seconds = None;
                Task::none()
            }
            Message::AddAppOverride => {
                let class = self
                    .opacity_selected_class
                    .clone()
                    .unwrap_or_else(|| self.opacity_custom_input.trim().to_string());
                if class.is_empty() {
                    return Task::none();
                }
                if self.app_opacities.iter().any(|e| e.app == class) {
                    return Task::none();
                }
                if op_backend::set_app_opacity(&class, 0.9) {
                    self.app_opacities.push(AppOpacity { app: class.clone(), active: 0.9 });
                    self.opacity_custom_input.clear();
                    self.opacity_selected_class = None;
                    self.open_window_classes.retain(|c| *c != class);
                    self.toast = Some(t("toast_override_added").replace("{}", &class));
                } else {
                    self.toast = Some(t("toast_override_failed"));
                }
                Task::none()
            }
            Message::RemoveAppOverride(app) => {
                if op_backend::remove_app_opacity(&app) {
                    self.app_opacities.retain(|e| e.app != app);
                    self.toast = Some(t("toast_override_removed").replace("{}", &app));
                    self.open_window_classes = op_backend::get_open_window_classes();
                }
                Task::none()
            }
            Message::OpacityCustomChanged(s) => {
                self.opacity_custom_input = s;
                self.opacity_selected_class = None;
                Task::none()
            }
            Message::OpacityClassSelected(cls) => {
                self.opacity_selected_class = Some(cls.clone());
                self.opacity_custom_input = cls;
                Task::none()
            }
            Message::RefreshOpenWindows => {
                self.open_window_classes = op_backend::get_open_window_classes();
                self.open_window_total = self.open_window_classes.len();
                let already: Vec<_> = self.app_opacities.iter().map(|e| e.app.clone()).collect();
                self.open_window_classes.retain(|c| !already.contains(c));
                Task::none()
            }

            // Profile / Perfil
            Message::DisplayNameInputChanged(s) => {
                self.display_name_input = prof_backend::sanitize_display_name(&s);
                Task::none()
            }
            Message::ApplyDisplayName => {
                let name = self.display_name_input.trim().to_string();
                if name.is_empty() {
                    self.toast = Some(t("toast_name_empty"));
                    return Task::none();
                }
                if prof_backend::set_display_name(&name) {
                    self.display_name = name;
                    self.toast = Some(t("toast_name_updated").replace("{}", &self.display_name));
                } else {
                    self.toast = Some(t("toast_name_failed"));
                }
                Task::none()
            }
            Message::ChangePhoto => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                        .set_title("Select profile photo")
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                },
                Message::PhotoPicked,
            ),
            Message::PhotoPicked(Some(path)) => {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(img) = ::image::load_from_memory(&data) {
                        let (ow, oh) = (img.width(), img.height());
                        let max_dim = 1024u32;
                        let (iw, ih) = if ow > max_dim || oh > max_dim {
                            if ow >= oh { (max_dim, oh * max_dim / ow) }
                            else { (ow * max_dim / oh, max_dim) }
                        } else { (ow, oh) };
                        let scaled = if (iw, ih) != (ow, oh) {
                            img.resize(iw, ih, ::image::imageops::FilterType::Lanczos3)
                        } else { img };
                        let source_rgba = scaled.to_rgba8();
                        let preview = profile::make_preview_handle(
                            &source_rgba, iw, ih, 1.0, 0.0, 0.0,
                        );
                        self.photo_crop = Some(PhotoCropState {
                            path,
                            source_rgba,
                            img_w: iw,
                            img_h: ih,
                            zoom: 1.0,
                            offset_x: 0.0,
                            offset_y: 0.0,
                            drag_active: false,
                            drag_last: None,
                            preview,
                        });
                    }
                }
                Task::none()
            }
            Message::PhotoPicked(None) => Task::none(),
            Message::PhotoCropZoomChanged(v) => {
                if let Some(ref mut c) = self.photo_crop {
                    c.zoom = (v as f32).max(1.0).min(5.0);
                    c.offset_x = c.offset_x.clamp(-1.0, 1.0);
                    c.offset_y = c.offset_y.clamp(-1.0, 1.0);
                    c.preview = profile::make_preview_handle(
                        &c.source_rgba, c.img_w, c.img_h, c.zoom, c.offset_x, c.offset_y,
                    );
                }
                Task::none()
            }
            Message::PhotoCropDragStart => {
                if let Some(ref mut c) = self.photo_crop {
                    c.drag_active = true;
                    c.drag_last = None;
                }
                Task::none()
            }
            Message::PhotoCropDragMove(x, y) => {
                if let Some(ref mut c) = self.photo_crop {
                    if c.drag_active {
                        if let Some((lx, ly)) = c.drag_last {
                            let half = profile::PREVIEW_SIZE as f32 / 2.0;
                            c.offset_x = (c.offset_x - (x - lx) / half).clamp(-1.0, 1.0);
                            c.offset_y = (c.offset_y - (y - ly) / half).clamp(-1.0, 1.0);
                            c.preview = profile::make_preview_handle(
                                &c.source_rgba, c.img_w, c.img_h, c.zoom, c.offset_x, c.offset_y,
                            );
                        }
                        c.drag_last = Some((x, y));
                    }
                }
                Task::none()
            }
            Message::PhotoCropDragEnd => {
                if let Some(ref mut c) = self.photo_crop {
                    c.drag_active = false;
                    c.drag_last = None;
                }
                Task::none()
            }
            Message::PhotoCropConfirm => {
                if let Some(c) = self.photo_crop.take() {
                    let base_dim = c.img_w.min(c.img_h) as f32;
                    let crop_f = (base_dim / c.zoom.max(0.1)).max(1.0).min(base_dim);
                    let max_ox = ((c.img_w as f32 - crop_f) / 2.0).max(0.0);
                    let max_oy = ((c.img_h as f32 - crop_f) / 2.0).max(0.0);
                    let cx_frac = (c.img_w as f32 / 2.0 + c.offset_x * max_ox) / c.img_w as f32;
                    let cy_frac = (c.img_h as f32 / 2.0 + c.offset_y * max_oy) / c.img_h as f32;
                    let r_frac = (crop_f / 2.0) / base_dim;
                    if let Some(dest) = prof_backend::save_avatar_with_fractions(
                        &c.path, cx_frac, cy_frac, r_frac,
                    ) {
                        let dest_path = PathBuf::from(dest);
                        self.avatar_handle = profile::make_circular_handle(&dest_path);
                        self.avatar_path = Some(dest_path);
                        self.toast = Some(t("toast_photo_updated"));
                    } else {
                        self.toast = Some(t("toast_photo_failed"));
                    }
                }
                Task::none()
            }
            Message::PhotoCropCancel => {
                self.photo_crop = None;
                Task::none()
            }
            Message::LanguageChanged(idx) => {
                self.lang_idx = idx;
                let lang = if idx == 0 { "en" } else { "es" };
                if lang != self.config.app_lang {
                    self.config.app_lang = lang.to_string();
                    config::save_config(&self.config);
                    Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        },
                        |_| {
                            let exe = std::env::current_exe().unwrap_or_default();
                            let _ = std::process::Command::new(exe).spawn();
                            std::process::exit(0);
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::LocaleChanged(idx) => {
                self.locale_idx = idx;
                Task::none()
            }
            Message::ApplyLocale => {
                if let Some(loc) = self.system_locales.get(self.locale_idx) {
                    if prof_backend::set_system_locale(loc) {
                        self.toast = Some(t("toast_locale_updated"));
                    } else {
                        self.toast = Some(t("toast_locale_failed"));
                    }
                }
                Task::none()
            }

            // Update / Actualización
            Message::UpdateChecked(v) => {
                self.update_available = v;
                Task::none()
            }
            Message::OpenUpdateLink => {
                crate::backend::update::open_releases_page();
                Task::none()
            }
            Message::DismissUpdate => {
                self.update_available = None;
                Task::none()
            }
        }
    }

    // ── Subscription / Suscripción ───────────────────────────

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = Vec::new();

        if self.toast.is_some() {
            subs.push(
                iced::time::every(Duration::from_secs(3)).map(|_| Message::ClearToast),
            );
        }

        if self.confirm_seconds.is_some_and(|s| s > 0) {
            subs.push(
                iced::time::every(Duration::from_secs(1)).map(|_| Message::ConfirmTick),
            );
        }

        Subscription::batch(subs)
    }

    // ── View / Vista ─────────────────────────────────────────

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.sidebar();
        let content: Element<_> = scrollable(match &self.page {
            Page::Brightness => brightness::view(self),
            Page::Night => night::view(self),
            Page::Monitors => monitors::view(self),
            Page::Opacity => opacity::view(self),
            Page::Profile => profile::view(self),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        let main_row = row![
            sidebar,
            rule::vertical(1).style(|_| rule::Style {
                color: Color::from_rgba8(0, 0, 0, 0.078),
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
                snap: false,
            }),
            container(content).width(Length::Fill).height(Length::Fill),
        ]
        .height(Length::Fill);

        // Toast overlay at the bottom / Overlay de toast en la parte inferior
        let base: Element<_> = container(main_row)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(BG)),
                ..Default::default()
            })
            .into();

        // Value confirm overlay — single shared timer for all sections / Aviso de valor confirmado — un solo temporizador compartido para todas las secciones
        let confirm_overlay: Option<Element<_>> = if let Some(seconds) = self.confirm_seconds {
            let pending = [
                self.monitor_confirm.is_some(),
                self.brightness_confirm.is_some(),
                self.night_temp_confirm.is_some(),
                self.opacity_active_confirm.is_some(),
                self.opacity_inactive_confirm.is_some(),
                self.app_opacity_confirm.is_some(),
            ]
            .iter()
            .filter(|&&b| b)
            .count();
            let label = if pending > 1 {
                t("confirm_pending")
            } else if self.monitor_confirm.is_some() {
                t("confirm_monitor")
            } else if self.brightness_confirm.is_some() {
                t("confirm_brightness")
            } else if self.night_temp_confirm.is_some() {
                t("confirm_night_temp")
            } else if self.opacity_active_confirm.is_some() {
                t("confirm_opacity_active")
            } else if self.opacity_inactive_confirm.is_some() {
                t("confirm_opacity_inactive")
            } else if let Some(c) = &self.app_opacity_confirm {
                t("confirm_app_opacity").replace("{}", &c.app)
            } else {
                t("confirm_pending")
            };
            Some(self.value_confirm_overlay(&label, seconds, Message::ConfirmKeep, Message::ConfirmRevert))
        } else {
            None
        };

        let toast_overlay: Option<Element<_>> = self
            .toast
            .as_ref()
            .filter(|_| confirm_overlay.is_none())
            .map(|toast_msg| {
                let toast = container(text(toast_msg).size(13).color(Color::WHITE))
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba8(30, 30, 30, 0.902))),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .padding(pad(8.0, 16.0, 8.0, 16.0));

                container(toast)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .padding(pad(0.0, 0.0, 24.0, 0.0))
                    .into()
            });

        let crop_overlay: Option<Element<_>> = self
            .photo_crop
            .is_some()
            .then(|| profile::crop_modal_view(self));

        let empty = || -> Element<'_, Message> { iced::widget::Space::new().into() };
        iced::widget::stack([
            base,
            confirm_overlay.unwrap_or_else(empty),
            toast_overlay.unwrap_or_else(empty),
            crop_overlay.unwrap_or_else(empty),
        ])
        .into()
    }

    // ── Value confirm overlay / Overlay de confirmación de valor ─

    fn value_confirm_overlay(
        &self,
        label: &str,
        seconds_left: u32,
        keep: Message,
        revert: Message,
    ) -> Element<'_, Message> {
        use widgets::{ghost_button, primary_button};

        let content = container(
            row![
                text(format!("{label} — {}s", seconds_left))
                    .size(13)
                    .color(Color::WHITE)
                    .width(Length::Fill),
                ghost_button(t("btn_revert"), revert),
                primary_button(t("btn_keep"), keep),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba8(30, 30, 30, 0.941))),
            border: iced::Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        })
        .padding(pad(10.0, 16.0, 10.0, 16.0))
        .width(500);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(pad(0.0, 0.0, 24.0, 0.0))
            .into()
    }

    // ── Sidebar / Barra lateral ──────────────────────────────

    fn sidebar(&self) -> Element<'_, Message> {
        use widgets::Icon;

        let items: Vec<(Icon, String, Page)> = vec![
            (Icon::Sun,     t("nav_brightness"), Page::Brightness),
            (Icon::Moon,    t("nav_night"),      Page::Night),
            (Icon::Monitor, t("nav_monitors"),   Page::Monitors),
            (Icon::Eye,     t("nav_opacity"),    Page::Opacity),
            (Icon::Person,  t("nav_profile"),    Page::Profile),
        ];

        // Brand / Marca
        let brand = container(
            row![
                image(self.brand_handle.clone())
                    .width(35)
                    .height(35),
                text("HyprDesk")
                    .size(17)
                    .font(iced::Font {
                        weight: iced::font::Weight::ExtraBold,
                        ..NUNITO
                    })
                    .color(TEXT),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(pad(20.0, 16.0, 16.0, 16.0))
        .width(Length::Fill);

        let nav_items: Vec<Element<_>> = items
            .into_iter()
            .map(|(icon, label, page)| {
                let active = self.page == page;
                let page_c = page.clone();
                let icon_color = if active { AMBER } else { ACCENT };
                let text_color = TEXT;
                let icon_bytes = icon.bytes().to_vec();

                button(
                    row![
                        container(
                            svg(svg::Handle::from_memory(icon_bytes))
                                .width(14)
                                .height(14)
                                .style(move |_, _| svg::Style { color: Some(icon_color) })
                        )
                        .width(26)
                        .height(26)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .style(move |_| container::Style {
                            background: Some(iced::Background::Color(
                                color_with_alpha(icon_color, if active { 0.15 } else { 0.10 }),
                            )),
                            border: iced::Border { radius: 7.0.into(), ..Default::default() },
                            ..Default::default()
                        }),
                        text(label).size(14).color(text_color).font(iced::Font {
                            weight: if active { iced::font::Weight::ExtraBold } else { iced::font::Weight::Semibold },
                            ..NUNITO
                        }),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding(pad(9.0, 12.0, 9.0, 12.0))
                .style(move |_, status| button::Style {
                    background: Some(iced::Background::Color(if active {
                        CARD
                    } else if matches!(status, button::Status::Hovered) {
                        color_with_alpha(TEXT, 0.06)
                    } else {
                        Color::TRANSPARENT
                    })),
                    border: iced::Border {
                        radius: 10.0.into(),
                        color: if active { Color::from_rgba8(0, 0, 0, 0.12) } else { Color::TRANSPARENT },
                        width: if active { 1.0 } else { 0.0 },
                    },
                    shadow: if active {
                        iced::Shadow {
                            color: Color::from_rgba8(0, 0, 0, 0.055),
                            offset: iced::Vector::new(0.0, 1.0),
                            blur_radius: 4.0,
                        }
                    } else {
                        iced::Shadow::default()
                    },
                    text_color,
                    ..Default::default()
                })
                .on_press(Message::Navigate(page_c))
                .into()
            })
            .collect();

        let nav_col = column(nav_items).spacing(2).padding(pad(8.0, 8.0, 8.0, 8.0));

        let mut footer_col: Vec<Element<_>> = vec![
            text(concat!("v", env!("CARGO_PKG_VERSION")))
                .size(10)
                .color(SUBTEXT)
                .font(NUNITO)
                .into(),
            text("by Besori")
                .size(11)
                .color(SUBTEXT)
                .font(NUNITO)
                .into(),
        ];

        if let Some(ref version) = self.update_available {
            let label = t("update_available").replace("{}", version);
            let star_bytes = Icon::Star.bytes().to_vec();
            let open_btn = button(
                row![
                    svg(svg::Handle::from_memory(star_bytes))
                        .width(10)
                        .height(10)
                        .style(|_, _| svg::Style { color: Some(AMBER) }),
                    text(label).size(11).color(AMBER).font(NUNITO),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .style(|_, status| button::Style {
                background: Some(iced::Background::Color(color_with_alpha(
                    AMBER,
                    if matches!(status, button::Status::Hovered) { 0.18 } else { 0.12 },
                ))),
                border: iced::Border {
                    color: color_with_alpha(AMBER, 0.30),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .padding(pad(3.0, 7.0, 3.0, 7.0))
            .on_press(Message::OpenUpdateLink);

            let dismiss_btn = button(text("×").size(12).color(SUBTEXT).font(NUNITO))
                .style(|_, _| button::Style {
                    background: None,
                    ..Default::default()
                })
                .padding(pad(2.0, 4.0, 2.0, 4.0))
                .on_press(Message::DismissUpdate);

            footer_col.push(
                row![open_btn, dismiss_btn]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }

        let footer = container(column(footer_col).spacing(4))
            .padding(pad(0.0, 0.0, 16.0, 20.0))
            .width(Length::Fill);

        container(column![brand, nav_col, space::vertical(), footer].spacing(0))
            .width(200)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(SIDEBAR)),
                ..Default::default()
            })
            .into()
    }

    // ── Theme / Tema ─────────────────────────────────────────

    pub fn theme(&self) -> Theme {
        Theme::custom(
            String::from("HyprDesk"),
            iced::theme::Palette {
                background: BG,
                text: TEXT,
                primary: ACCENT,
                success: SUCCESS,
                warning: iced::Color::from_rgb8(0xF3, 0x9C, 0x12),
                danger: iced::Color::from_rgb8(0xE7, 0x4C, 0x3C),
            },
        )
    }
}

pub const MAX_COORD: i32 = 32767;

fn numeric_coord(s: &str) -> String {
    let negative = s.starts_with('-');
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let significant = digits.trim_start_matches('0');
    let over = significant.len() > 5 || significant.parse::<i32>().is_ok_and(|v| v > MAX_COORD);
    let digits = if over { MAX_COORD.to_string() } else { digits };
    if negative { format!("-{digits}") } else { digits }
}

#[cfg(test)]
mod coord_tests {
    use super::numeric_coord;

    #[test]
    fn strips_letters_and_keeps_the_sign() {
        assert_eq!(numeric_coord("19a20"), "1920");
        assert_eq!(numeric_coord("-1080"), "-1080");
        assert_eq!(numeric_coord("abc"), "");
        assert_eq!(numeric_coord("-"), "-");
    }

    #[test]
    fn caps_at_the_screen_space_limit() {
        assert_eq!(numeric_coord("99999"), "32767");
        assert_eq!(numeric_coord("-99999"), "-32767");
        assert_eq!(numeric_coord("123456789012"), "32767");
        assert_eq!(numeric_coord("32767"), "32767");
    }
}

// ── Entry point / Punto de entrada ───────────────────────────

pub fn run() -> iced::Result {
    let icon = {
        let raw = ::image::load_from_memory(
            include_bytes!("../assets/icons/hyprdesk-ico.png")
        )
        .unwrap()
        .into_rgba8();
        let w = raw.width();
        let h = raw.height();
        iced::window::icon::from_rgba(raw.into_raw(), w, h).ok()
    };

    iced::application(App::new, App::update, App::view)
        .title("HyprDesk")
        .theme(App::theme)
        .subscription(App::subscription)
        .font(include_bytes!("../assets/fonts/Nunito-VariableFont_wght.ttf").as_slice())
        .default_font(NUNITO)
        .window(iced::window::Settings {
            size: Size::new(1100.0, 680.0),
            min_size: Some(Size::new(800.0, 500.0)),
            icon,
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "hyprdesk".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
