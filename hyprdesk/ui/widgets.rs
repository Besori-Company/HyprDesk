// Widgets — reusable UI components: icon set, setting rows, buttons and section headers.
// Widgets — componentes de UI reutilizables: iconos, filas de ajuste, botones y cabeceras de sección.

use iced::{
    Alignment, Color, Element, Length,
    widget::{button, column, container, row, svg, text},
};

use super::theme::*;
use super::Message;

// ── Icon / Icono ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub enum Icon {
    Sun,
    Moon,
    Monitor,
    Eye,
    EyeOff,
    Person,
    Globe,
    Star,
    Warning,
    Move,
    Rotate,
    Expand,
    Grid,
    Thermometer,
}

impl Icon {
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Icon::Sun     => include_bytes!("../assets/icons/sun.svg"),
            Icon::Moon    => include_bytes!("../assets/icons/moon.svg"),
            Icon::Monitor => include_bytes!("../assets/icons/monitor.svg"),
            Icon::Eye     => include_bytes!("../assets/icons/eye.svg"),
            Icon::EyeOff  => include_bytes!("../assets/icons/eye-off.svg"),
            Icon::Person  => include_bytes!("../assets/icons/person.svg"),
            Icon::Globe   => include_bytes!("../assets/icons/globe.svg"),
            Icon::Star    => include_bytes!("../assets/icons/star.svg"),
            Icon::Warning => include_bytes!("../assets/icons/warning.svg"),
            Icon::Move    => include_bytes!("../assets/icons/move.svg"),
            Icon::Rotate  => include_bytes!("../assets/icons/rotate.svg"),
            Icon::Expand  => include_bytes!("../assets/icons/expand.svg"),
            Icon::Grid    => include_bytes!("../assets/icons/grid.svg"),
            Icon::Thermometer => include_bytes!("../assets/icons/thermometer.svg"),
        }
    }
}

// ── Page layout / Estructura de página ───────────────────────

pub fn page_content<'a>(children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    column(children)
        .spacing(0)
        .padding(pad(30.0, 34.0, 30.0, 34.0))
        .align_x(Alignment::Start)
        .into()
}

pub fn page_title(label: impl Into<String>) -> Element<'static, Message> {
    container(
        text(label.into())
            .size(22)
            .font(iced::Font {
                weight: iced::font::Weight::ExtraBold,
                ..NUNITO
            })
            .color(TEXT),
    )
    .padding(pad(0.0, 0.0, 24.0, 0.0))
    .into()
}

// ── Card / Tarjeta ───────────────────────────────────────────

pub fn card<'a>(
    section_title: Option<impl Into<String>>,
    children: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut outer: Vec<Element<_>> = Vec::new();

    if let Some(title) = section_title {
        let title_str = title.into().to_uppercase();
        outer.push(
            text(title_str)
                .size(11)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..NUNITO
                })
                .color(SUBTEXT)
                .into(),
        );
    }

    outer.push(
        container(column(children).spacing(0))
            .style(card_style)
            .padding(0)
            .width(Length::Fill)
            .into(),
    );

    column(outer)
        .spacing(6)
        .padding(pad(0.0, 0.0, 18.0, 0.0))
        .into()
}

pub fn card_sep() -> Element<'static, Message> {
    container(iced::widget::rule::horizontal(1).style(|_| iced::widget::rule::Style {
        color: SEP,
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
        snap: false,
    }))
    .padding(pad(0.0, 16.0, 0.0, 16.0))
    .into()
}

// ── Row / Fila ───────────────────────────────────────────────

pub fn info_row<'a>(
    icon: Icon,
    title: impl Into<String>,
    subtitle: Option<impl Into<String>>,
    icon_color: Color,
    trailing: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let handle = svg::Handle::from_memory(icon.bytes().to_vec());
    let icon_widget = svg(handle)
        .width(16)
        .height(16)
        .style(move |_, _| svg::Style { color: Some(icon_color) });

    let icon_box = container(icon_widget)
        .width(32)
        .height(32)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color_with_alpha(icon_color, 0.12))),
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let title_str = title.into();
    let mut text_col: Vec<Element<_>> = vec![
        text(title_str).size(14).color(TEXT).font(iced::Font {
            weight: iced::font::Weight::Semibold,
            ..NUNITO
        }).into(),
    ];
    if let Some(sub) = subtitle {
        text_col.push(text(sub.into()).size(12).color(SUBTEXT).into());
    }

    let mut r: Vec<Element<_>> = vec![
        icon_box.into(),
        column(text_col).spacing(2).width(Length::Fill).into(),
    ];

    if let Some(trail) = trailing {
        r.push(trail);
    }

    container(
        row(r).spacing(12).align_y(Alignment::Center),
    )
    .padding(pad(12.0, 16.0, 12.0, 16.0))
    .width(Length::Fill)
    .into()
}

// ── Badge / Etiqueta ─────────────────────────────────────────

pub fn value_badge<'a>(val: impl Into<String>, color: Color) -> Element<'a, Message> {
    container(
        text(val.into()).size(13).color(color).font(iced::Font {
            weight: iced::font::Weight::ExtraBold,
            ..NUNITO
        }),
    )
    .style(chip_style(color))
    .padding(pad(4.0, 10.0, 4.0, 10.0))
    .into()
}

pub fn chip_label<'a>(label: impl Into<String>, color: Color) -> Element<'a, Message> {
    container(text(label.into()).size(12).color(color).font(iced::Font {
        weight: iced::font::Weight::Semibold,
        ..NUNITO
    }))
        .style(chip_style(color))
        .padding(pad(3.0, 8.0, 3.0, 8.0))
        .into()
}

// ── Buttons / Botones ────────────────────────────────────────

pub fn primary_button<'a>(label: impl Into<String>, msg: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13).font(iced::Font {
        weight: iced::font::Weight::ExtraBold,
        ..NUNITO
    }))
        .style(primary_btn_style)
        .padding(pad(8.0, 14.0, 8.0, 14.0))
        .on_press(msg)
        .into()
}

pub fn secondary_button<'a>(label: impl Into<String>, msg: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13).font(iced::Font {
        weight: iced::font::Weight::Semibold,
        ..NUNITO
    }))
        .style(secondary_btn_style)
        .padding(pad(8.0, 14.0, 8.0, 14.0))
        .on_press(msg)
        .into()
}

pub fn ghost_button<'a>(label: impl Into<String>, msg: Message) -> Element<'a, Message> {
    button(text(label.into()).size(13).font(iced::Font {
        weight: iced::font::Weight::Semibold,
        ..NUNITO
    }))
        .style(ghost_btn_style)
        .padding(pad(8.0, 14.0, 8.0, 14.0))
        .on_press(msg)
        .into()
}

