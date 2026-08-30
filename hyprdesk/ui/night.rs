// Night mode page — UI for enabling night light and adjusting color temperature.
// Página de modo noche — interfaz para activar luz nocturna y ajustar la temperatura de color.

use iced::{Element, Length, widget::{row, slider, toggler}};

use super::{App, Message};
use super::theme::*;
use super::widgets::*;
use crate::i18n::t;

pub fn view(app: &App) -> Element<'_, Message> {
    let toggle = toggler(app.night_mode)
        .on_toggle(Message::NightModeToggled)
        .size(22)
        .style(toggler_style(AMBER));

    let card_children = if app.night_available {
        let night_row = info_row(
            Icon::Moon,
            t("row_enable_night"),
            Some(t("row_enable_night_sub")),
            AMBER,
            Some(toggle.into()),
        );

        let temp_badge = value_badge(format!("{} K", app.night_temp), AMBER);
        let temp_row = info_row(
            Icon::Thermometer,
            t("row_color_temp"),
            Some(t("row_color_temp_sub")),
            AMBER,
            Some(temp_badge),
        );

        let temp_slider = slider(1000.0..=6500.0f64, app.night_temp as f64, Message::NightTempChanged)
            .step(100.0)
            .width(Length::Fill)
            .style(slider_style(AMBER));

        let slider_row = iced::widget::container(
            row![temp_slider].padding(pad(4.0, 16.0, 14.0, 16.0)),
        )
        .width(Length::Fill);

        vec![
            night_row,
            card_sep(),
            temp_row,
            card_sep(),
            slider_row.into(),
        ]
    } else {
        vec![info_row(
            Icon::Warning,
            t("row_tool_missing"),
            Some(t("row_tool_missing_sub")),
            AMBER,
            None,
        )]
    };

    page_content(vec![
        page_title(t("title_night")),
        card(Some(t("section_night_light")), card_children),
    ])
}
