// Brightness page — UI for adjusting screen brightness with live preview and auto-revert.
// Página de brillo — interfaz para ajustar el brillo de pantalla con vista previa y reversión automática.

use iced::{Element, Length, widget::{row, slider}};

use super::{App, Message};
use super::theme::*;
use super::widgets::*;

pub fn view(app: &App) -> Element<'_, Message> {
    let method_sub = crate::i18n::t("active_method").replace("{}", &app.brightness_method);

    let badge = value_badge(&format!("{}%", app.brightness), ACCENT);

    let brightness_row = info_row(
        Icon::Sun,
        crate::i18n::t("row_brightness"),
        Some(method_sub),
        ACCENT,
        Some(badge),
    );

    let sl = slider(10.0..=100.0f64, app.brightness as f64, Message::BrightnessChanged)
        .step(1.0)
        .width(Length::Fill)
        .style(slider_style(ACCENT));

    let slider_container = iced::widget::container(
        row![sl].padding(pad(4.0, 16.0, 14.0, 16.0)),
    )
    .width(Length::Fill);

    let main_card = card(
        Some(crate::i18n::t("section_main_display")),
        if app.brightness_available {
            vec![brightness_row, card_sep(), slider_container.into()]
        } else {
            vec![info_row(
                Icon::Warning,
                crate::i18n::t("row_tool_missing"),
                Some(crate::i18n::t("row_brightnessctl_missing_sub")),
                AMBER,
                None,
            )]
        },
    );

    page_content(vec![
        page_title(crate::i18n::t("title_brightness")),
        main_card,
    ])
}
