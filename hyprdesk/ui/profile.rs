// Profile page — UI for editing display name, profile photo (with crop) and system locale.
// Página de perfil — interfaz para editar el nombre, foto de perfil (con recorte) e idioma del sistema.

use iced::{
    Alignment, Element, Length,
    widget::{column, container, image, pick_list, row, slider, text, text_input},
};

use crate::i18n::t;
use super::{App, Message};
use super::theme::*;
use super::widgets::*;

pub const AVATAR_SIZE: u32 = 72;
pub const PREVIEW_SIZE: u32 = 340;

// ── Image helpers / Utilidades de imagen ─────────────────────

/// Loads a file (detecting format by magic bytes) and returns a circular RGBA Handle.
/// Carga un archivo (detectando el formato por magic bytes) y devuelve un Handle RGBA circular.
pub fn make_circular_handle(path: &std::path::Path) -> Option<image::Handle> {
    let data = std::fs::read(path).ok()?;
    let img = ::image::load_from_memory(&data).ok()?;
    let img = img.resize_to_fill(AVATAR_SIZE, AVATAR_SIZE, ::image::imageops::FilterType::Lanczos3);
    let mut rgba = img.to_rgba8();
    let cx = AVATAR_SIZE as f32 / 2.0;
    let r2 = cx * cx;
    for y in 0..AVATAR_SIZE {
        for x in 0..AVATAR_SIZE {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cx;
            if dx * dx + dy * dy > r2 {
                rgba.get_pixel_mut(x, y)[3] = 0;
            }
        }
    }
    Some(image::Handle::from_rgba(AVATAR_SIZE, AVATAR_SIZE, rgba.into_raw()))
}

/// Generates a circular preview handle from an in-memory RGBA image (no disk I/O).
/// Genera un Handle de vista previa circular desde una imagen RGBA en memoria (sin I/O en disco).
pub fn make_preview_handle(
    rgba: &::image::RgbaImage,
    img_w: u32,
    img_h: u32,
    zoom: f32,
    offset_x: f32,
    offset_y: f32,
) -> Option<image::Handle> {
    let size = PREVIEW_SIZE;
    let base_dim = img_w.min(img_h) as f32;
    let crop_f = (base_dim / zoom.max(0.1)).max(1.0).min(base_dim);
    let crop_dim = crop_f as u32;

    let max_ox = ((img_w as f32 - crop_f) / 2.0).max(0.0);
    let max_oy = ((img_h as f32 - crop_f) / 2.0).max(0.0);
    let cx = img_w as f32 / 2.0 + offset_x * max_ox;
    let cy = img_h as f32 / 2.0 + offset_y * max_oy;
    let x = (cx - crop_f / 2.0).max(0.0).min((img_w.saturating_sub(crop_dim)) as f32) as u32;
    let y = (cy - crop_f / 2.0).max(0.0).min((img_h.saturating_sub(crop_dim)) as f32) as u32;
    let w = crop_dim.min(img_w - x);
    let h = crop_dim.min(img_h - y);

    let sub = ::image::imageops::crop_imm(rgba, x, y, w, h).to_image();
    let dyn_img: ::image::DynamicImage = sub.into();
    let resized = dyn_img.resize_to_fill(size, size, ::image::imageops::FilterType::Lanczos3);

    let mut out = resized.to_rgba8();
    let cf = size as f32 / 2.0;
    let r2 = cf * cf;
    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 + 0.5 - cf;
            let dy = py as f32 + 0.5 - cf;
            if dx * dx + dy * dy > r2 {
                out.get_pixel_mut(px, py)[3] = 55;
            }
        }
    }
    Some(image::Handle::from_rgba(size, size, out.into_raw()))
}

// ── Profile view / Vista de perfil ───────────────────────────

pub fn view(app: &App) -> Element<'_, Message> {
    let avatar: Element<_> = if let Some(handle) = &app.avatar_handle {
        image::Image::new(handle.clone())
            .width(AVATAR_SIZE as f32)
            .height(AVATAR_SIZE as f32)
            .into()
    } else {
        let initials: String = app
            .display_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        let initials_display = if initials.is_empty() { "?".to_string() } else { initials };
        container(
            text(initials_display)
                .size(24)
                .color(iced::Color::WHITE)
                .font(iced::Font { weight: iced::font::Weight::ExtraBold, ..NUNITO }),
        )
        .style(|_| container::Style {
            background: Some(iced::Background::Color(ACCENT)),
            border: iced::Border { radius: 36.0.into(), ..Default::default() },
            ..Default::default()
        })
        .width(AVATAR_SIZE as f32)
        .height(AVATAR_SIZE as f32)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
    };

    let name_col = column![
        text(&app.display_name)
            .size(18)
            .font(iced::Font { weight: iced::font::Weight::Semibold, ..NUNITO })
            .color(TEXT),
        text(format!("@{}", app.username)).size(13).color(SUBTEXT),
        secondary_button(t("btn_change_photo"), Message::ChangePhoto),
    ]
    .spacing(6);

    let avatar_row = container(
        row![avatar, name_col]
            .spacing(20)
            .align_y(Alignment::Center)
            .padding(pad(20.0, 20.0, 20.0, 20.0)),
    )
    .width(Length::Fill);

    let photo_card = card(Some(t("section_photo")), vec![avatar_row.into()]);

    let name_input = row![
        text_input(&app.display_name, &app.display_name_input)
            .on_input(Message::DisplayNameInputChanged)
            .on_submit(Message::ApplyDisplayName)
            .style(input_style)
            .padding(pad(7.0, 10.0, 7.0, 10.0))
            .width(220),
        primary_button(t("btn_apply"), Message::ApplyDisplayName),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let name_card = card(
        Some(t("section_user_info")),
        vec![info_row(
            Icon::Person,
            t("row_display_name"),
            Some(t("row_display_name_sub")),
            ACCENT,
            Some(name_input.into()),
        )],
    );

    let lang_options = vec!["English".to_string(), "Español".to_string()];
    let lang_selected = lang_options.get(app.lang_idx).cloned();
    let lang_dd = pick_list(lang_options, lang_selected, |chosen: String| {
        let idx = if chosen == "Español" { 1 } else { 0 };
        Message::LanguageChanged(idx)
    })
    .width(160)
    .style(pick_list_style)
    .menu_style(menu_style);

    let mut lang_children = vec![info_row(
        Icon::Globe,
        t("row_app_language"),
        Some(t("row_app_language_sub")),
        ACCENT,
        Some(lang_dd.into()),
    )];

    if app.system_locales.is_empty() {
        lang_children.push(card_sep());
        lang_children.push(info_row(
            Icon::Warning,
            t("row_system_language"),
            Some(t("localectl_unavailable")),
            AMBER,
            None,
        ));
    } else {
        let locale_selected = app.system_locales.get(app.locale_idx).cloned();
        let locales_c = app.system_locales.clone();
        let locale_dd = pick_list(
            app.system_locales.clone(),
            locale_selected,
            move |chosen: String| {
                let idx = locales_c.iter().position(|l| *l == chosen).unwrap_or(0);
                Message::LocaleChanged(idx)
            },
        )
        .width(220)
        .style(pick_list_style)
        .menu_style(menu_style);

        let locale_row = row![locale_dd, primary_button(t("btn_apply"), Message::ApplyLocale)]
            .spacing(8)
            .align_y(Alignment::Center);

        lang_children.push(card_sep());
        lang_children.push(info_row(
            Icon::Globe,
            t("row_system_language"),
            Some(t("row_system_language_sub")),
            ACCENT,
            Some(locale_row.into()),
        ));
    }

    let lang_card = card(Some(t("section_language")), lang_children);

    page_content(vec![
        page_title(t("title_profile")),
        photo_card,
        name_card,
        lang_card,
    ])
}

// ── Crop modal / Modal de recorte ────────────────────────────

pub fn crop_modal_view<'a>(app: &'a App) -> Element<'a, Message> {
    let state = app.photo_crop.as_ref().unwrap();
    let ps = PREVIEW_SIZE as f32;

    // Circular preview wrapped in mouse_area for drag + scroll / Vista previa circular envuelta en mouse_area para arrastrar y hacer scroll
    let preview_img: Element<_> = if let Some(ref handle) = state.preview {
        image::Image::new(handle.clone())
            .width(ps)
            .height(ps)
            .into()
    } else {
        container(iced::widget::Space::new().width(ps).height(ps))
            .width(ps)
            .height(ps)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(ACCENT)),
                border: iced::Border { radius: (PREVIEW_SIZE as f32 / 2.0).into(), ..Default::default() },
                ..Default::default()
            })
            .into()
    };

    let preview_interactive = iced::widget::mouse_area(
        // Gray background so circle mask transparent corners show correctly / Fondo gris para que las esquinas transparentes de la máscara circular se vean bien
        container(preview_img)
            .width(ps)
            .height(ps)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(BG)),
                ..Default::default()
            })
    )
    .on_press(Message::PhotoCropDragStart)
    .on_release(Message::PhotoCropDragEnd)
    .on_move(|p| Message::PhotoCropDragMove(p.x, p.y))
    .on_scroll(|delta| {
        let dy = match delta {
            iced::mouse::ScrollDelta::Lines { y, .. } => y,
            iced::mouse::ScrollDelta::Pixels { y, .. } => y / 30.0,
        };
        Message::PhotoCropZoomChanged((state.zoom + dy * 0.15).max(1.0).min(5.0) as f64)
    });

    // Zoom slider with – and + buttons / Deslizador de zoom con botones – y +
    let zoom_minus = iced::widget::button(text("−").size(16).color(SUBTEXT))
        .style(secondary_btn_style)
        .padding(pad(4.0, 11.0, 4.0, 11.0))
        .on_press(Message::PhotoCropZoomChanged((state.zoom - 0.25).max(1.0) as f64));

    let zoom_plus = iced::widget::button(text("+").size(16).color(SUBTEXT))
        .style(secondary_btn_style)
        .padding(pad(4.0, 11.0, 4.0, 11.0))
        .on_press(Message::PhotoCropZoomChanged((state.zoom + 0.25).min(5.0) as f64));

    let zoom_sl = slider(1.0f64..=5.0, state.zoom as f64, Message::PhotoCropZoomChanged)
        .step(0.05)
        .width(Length::Fill)
        .style(slider_style(SUBTEXT));

    let zoom_row = row![zoom_minus, zoom_sl, zoom_plus]
        .spacing(8)
        .align_y(Alignment::Center);

    let btn_row = row![
        secondary_button(t("btn_cancel"), Message::PhotoCropCancel),
        iced::widget::Space::new().width(Length::Fill),
        primary_button(t("btn_apply"), Message::PhotoCropConfirm),
    ]
    .align_y(Alignment::Center);

    // Card: preview on top (no padding), controls below with padding / Tarjeta: vista previa arriba (sin padding), controles abajo con padding
    let card_width = ps + 40.0;
    let modal_card = container(
        column![
            // Preview area flush with card edges / Área de vista previa al ras de los bordes de la tarjeta
            container(preview_interactive)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .padding(pad(20.0, 20.0, 16.0, 20.0)),
            // Divider / Separador
            container(
                iced::widget::rule::horizontal(1).style(|_| iced::widget::rule::Style {
                    color: SEP,
                    radius: 0.0.into(),
                    fill_mode: iced::widget::rule::FillMode::Full,
                    snap: false,
                })
            )
            .width(Length::Fill)
            .padding(pad(0.0, 0.0, 0.0, 0.0)),
            // Controls / Controles
            container(
                column![zoom_row, iced::widget::Space::new().height(8), btn_row]
                    .spacing(0)
                    .padding(pad(14.0, 20.0, 18.0, 20.0))
            )
            .width(Length::Fill),
        ]
        .spacing(0)
    )
    .width(card_width)
    .style(card_style);

    // Backdrop + centered card / Fondo semitransparente + tarjeta centrada
    container(modal_card)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba8(0, 0, 0, 0.5))),
            ..Default::default()
        })
        .into()
}
