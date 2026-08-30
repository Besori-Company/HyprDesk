// Opacity page — UI for controlling global window opacity and per-app overrides.
// Página de opacidad — interfaz para controlar la opacidad global de ventanas y overrides por app.

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, slider, text, text_input},
};

use crate::i18n::t;
use super::{App, Message};
use super::theme::*;
use super::widgets::*;

pub fn view(app: &App) -> Element<'_, Message> {
    if !app.opacity_available {
        return page_content(vec![
            page_title(t("title_opacity")),
            card(
                Some(t("section_global_opacity")),
                vec![info_row(
                    Icon::Warning,
                    t("row_tool_missing"),
                    Some(t("row_hyprctl_missing_sub")),
                    AMBER,
                    None,
                )],
            ),
        ]);
    }

    let active_pct = (app.opacity_active * 100.0) as u32;
    let inactive_pct = (app.opacity_inactive * 100.0) as u32;

    let active_badge = value_badge(format!("{active_pct}%"), ACCENT);
    let inactive_badge = value_badge(format!("{inactive_pct}%"), ACCENT);

    let active_sl = slider(
        10.0..=100.0f64,
        app.opacity_active * 100.0,
        Message::OpacityActiveChanged,
    )
    .step(1.0)
    .width(Length::Fill)
    .style(slider_style(ACCENT));

    let inactive_sl = slider(
        10.0..=100.0f64,
        app.opacity_inactive * 100.0,
        Message::OpacityInactiveChanged,
    )
    .step(1.0)
    .width(Length::Fill)
    .style(slider_style(ACCENT));

    let global_card = card(
        Some(t("section_global_opacity")),
        vec![
            info_row(
                Icon::Eye,
                t("row_active_opacity"),
                Some(t("row_active_opacity_sub")),
                ACCENT,
                Some(active_badge),
            ),
            card_sep(),
            container(row![active_sl].padding(pad(4.0, 16.0, 14.0, 16.0)))
                .width(Length::Fill)
                .into(),
            card_sep(),
            info_row(
                Icon::EyeOff,
                t("row_inactive_opacity"),
                Some(t("row_inactive_opacity_sub")),
                ACCENT,
                Some(inactive_badge),
            ),
            card_sep(),
            container(row![inactive_sl].padding(pad(4.0, 16.0, 14.0, 16.0)))
                .width(Length::Fill)
                .into(),
        ],
    );

    let overrides_card = build_overrides_card(app);

    page_content(vec![
        page_title(t("title_opacity")),
        global_card,
        overrides_card,
    ])
}

fn build_overrides_card<'a>(app: &'a App) -> Element<'a, Message> {
    let mut card_children: Vec<Element<_>> = Vec::new();

    for (i, entry) in app.app_opacities.iter().enumerate() {
        let pct = (entry.active * 100.0) as u32;
        let app_name = entry.app.clone();
        let app_name_sl = entry.app.clone();
        let btn_remove = t("btn_remove");

        let sl = slider(
            10.0..=100.0f64,
            entry.active * 100.0,
            move |v| Message::AppOpacityChanged(app_name_sl.clone(), v),
        )
        .step(1.0)
        .width(Length::Fill)
        .style(slider_style(ACCENT));

        let entry_row = container(
            row![
                text(&entry.app)
                    .size(13)
                    .color(TEXT)
                    .font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..NUNITO
                    })
                    .width(120),
                sl,
                text(format!("{pct}%"))
                    .size(12)
                    .color(SUBTEXT)
                    .font(iced::Font {
                        weight: iced::font::Weight::ExtraBold,
                        ..NUNITO
                    })
                    .width(35),
                button(text(btn_remove).size(12).font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..NUNITO
                }))
                    .style(secondary_btn_style)
                    .padding(pad(5.0, 10.0, 5.0, 10.0))
                    .on_press(Message::RemoveAppOverride(app_name)),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding(pad(10.0, 16.0, 10.0, 16.0)),
        )
        .width(Length::Fill);

        card_children.push(entry_row.into());
        if i < app.app_opacities.len() - 1 {
            card_children.push(card_sep());
        }
    }

    if app.app_opacities.is_empty() {
        card_children.push(
            container(text(t("add_override_open_apps")).size(13).color(SUBTEXT))
                .padding(pad(12.0, 16.0, 12.0, 16.0))
                .into(),
        );
    } else {
        card_children.push(card_sep());
    }

    let mut add_children: Vec<Element<_>> = Vec::new();

    let header = row![
        text(t("add_override_open_apps"))
            .size(13)
            .color(TEXT)
            .font(iced::Font {
                weight: iced::font::Weight::Semibold,
                ..NUNITO
            })
            .width(Length::Fill),
        button(text(t("btn_refresh")).size(12).font(iced::Font {
            weight: iced::font::Weight::Semibold,
            ..NUNITO
        }))
            .style(secondary_btn_style)
            .padding(pad(5.0, 10.0, 5.0, 10.0))
            .on_press(Message::RefreshOpenWindows),
    ]
    .align_y(Alignment::Center);

    add_children.push(header.into());

    if app.open_window_classes.is_empty() {
        let msg = if app.open_window_total > 0 {
            t("add_override_all_added")
        } else {
            t("add_override_no_open")
        };
        add_children.push(text(msg).size(12).color(SUBTEXT).into());
    } else {
        let chips: Vec<Element<_>> = app
            .open_window_classes
            .iter()
            .map(|cls| {
                let is_sel = app.opacity_selected_class.as_deref() == Some(cls.as_str());
                let cls_c = cls.clone();
                button(text(cls).size(12).color(if is_sel { ACCENT } else { TEXT }))
                    .style(move |_, status| iced::widget::button::Style {
                        background: Some(iced::Background::Color(if is_sel {
                            color_with_alpha(ACCENT, 0.12)
                        } else if matches!(status, iced::widget::button::Status::Hovered) {
                            color_with_alpha(TEXT, 0.06)
                        } else {
                            color_with_alpha(TEXT, 0.04)
                        })),
                        border: iced::Border {
                            color: if is_sel { color_with_alpha(ACCENT, 0.3) } else { color_with_alpha(TEXT, 0.1) },
                            width: 1.0,
                            radius: 14.0.into(),
                        },
                        text_color: if is_sel { ACCENT } else { TEXT },
                        ..Default::default()
                    })
                    .padding(pad(4.0, 10.0, 4.0, 10.0))
                    .on_press(Message::OpacityClassSelected(cls_c))
                    .into()
            })
            .collect();

        add_children.push(
            iced::widget::scrollable(iced::widget::row(chips).spacing(6))
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::default(),
                ))
                .into(),
        );
    }

    let placeholder = t("add_override_custom_placeholder");
    let add_row = row![
        text_input(&placeholder, &app.opacity_custom_input)
            .on_input(Message::OpacityCustomChanged)
            .on_submit(Message::AddAppOverride)
            .style(input_style)
            .padding(pad(7.0, 10.0, 7.0, 10.0))
            .width(Length::Fill),
        primary_button(t("btn_add"), Message::AddAppOverride),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    add_children.push(add_row.into());

    card_children.push(
        container(
            column(add_children).spacing(8).padding(pad(12.0, 16.0, 12.0, 16.0)),
        )
        .width(Length::Fill)
        .into(),
    );

    card(Some(t("section_app_overrides")), card_children)
}
