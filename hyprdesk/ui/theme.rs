// Theme — color palette, fonts and reusable widget styles for the whole app.
// Tema — paleta de colores, fuentes y estilos reutilizables de widgets para toda la app.

use iced::{Border, Color, widget::container};

// ── Font / Fuente ─────────────────────────────────────────────

pub const NUNITO: iced::Font = iced::Font::with_name("Nunito");

// ── Palette / Paleta ─────────────────────────────────────────

pub const BG: Color = Color { r: 0.941, g: 0.941, b: 0.941, a: 1.0 };
pub const CARD: Color = Color::WHITE;
pub const SIDEBAR: Color = Color { r: 0.969, g: 0.969, b: 0.969, a: 1.0 };
pub const ACCENT: Color = Color { r: 0.239, g: 0.435, b: 0.714, a: 1.0 };
pub const AMBER: Color = Color { r: 0.910, g: 0.639, b: 0.200, a: 1.0 }; // #E8A333
pub const SUCCESS: Color = Color { r: 0.180, g: 0.800, b: 0.443, a: 1.0 };
pub const TEXT: Color = Color { r: 0.102, g: 0.102, b: 0.102, a: 1.0 };
pub const SUBTEXT: Color = Color { r: 0.450, g: 0.450, b: 0.450, a: 1.0 };
pub const BORDER: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.07 };
pub const SEP: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.06 };

// ── Helpers / Utilidades ─────────────────────────────────────

pub fn color_with_alpha(c: Color, a: f32) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a }
}

/// Build padding from [top, right, bottom, left] values. / Construye padding desde valores [arriba, derecha, abajo, izquierda].
pub fn pad(top: f32, right: f32, bottom: f32, left: f32) -> iced::Padding {
    iced::Padding { top, right, bottom, left }
}

// ── Container styles / Estilos de contenedor ─────────────────

pub fn card_style(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(CARD)),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.031),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 4.0,
        },
        ..Default::default()
    }
}

pub fn chip_style(color: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(iced::Background::Color(color_with_alpha(color, 0.12))),
        border: Border {
            color: color_with_alpha(color, 0.25),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

// ── Button styles / Estilos de botón ─────────────────────────

pub fn primary_btn_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let bg = match status {
        iced::widget::button::Status::Hovered => Color { r: 0.220, g: 0.220, b: 0.220, a: 1.0 },
        iced::widget::button::Status::Pressed => Color { r: 0.040, g: 0.040, b: 0.040, a: 1.0 },
        _ => Color { r: 0.102, g: 0.102, b: 0.102, a: 1.0 },
    };
    iced::widget::button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border { radius: 20.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub fn secondary_btn_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(match status {
            iced::widget::button::Status::Hovered => Color::from_rgba8(0, 0, 0, 0.047),
            iced::widget::button::Status::Pressed => Color::from_rgba8(0, 0, 0, 0.071),
            _ => Color::from_rgba8(0, 0, 0, 0.031),
        })),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn ghost_btn_style(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(match status {
            iced::widget::button::Status::Hovered => Color::from_rgba8(255, 255, 255, 0.12),
            iced::widget::button::Status::Pressed => Color::from_rgba8(255, 255, 255, 0.18),
            _ => Color::from_rgba8(255, 255, 255, 0.07),
        })),
        text_color: Color::from_rgba8(210, 210, 210, 1.0),
        border: iced::Border { radius: 20.0.into(), ..Default::default() },
        ..Default::default()
    }
}

// ── Toggler styles / Estilos de interruptor ──────────────────

pub fn toggler_style(
    color: Color,
) -> impl Fn(&iced::Theme, iced::widget::toggler::Status) -> iced::widget::toggler::Style {
    move |_, status| {
        let is_toggled = match status {
            iced::widget::toggler::Status::Active { is_toggled } => is_toggled,
            iced::widget::toggler::Status::Hovered { is_toggled } => is_toggled,
            iced::widget::toggler::Status::Disabled { .. } => false,
        };
        iced::widget::toggler::Style {
            background: if is_toggled {
                iced::Background::Color(color)
            } else {
                iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.20))
            },
            background_border_width: 0.0,
            background_border_color: Color::TRANSPARENT,
            foreground: iced::Background::Color(Color::WHITE),
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
            text_color: None,
            border_radius: None,
            padding_ratio: 0.15,
        }
    }
}

// ── Slider styles / Estilos de deslizador ────────────────────

pub fn slider_style(
    color: Color,
) -> impl Fn(&iced::Theme, iced::widget::slider::Status) -> iced::widget::slider::Style {
    move |_, _| iced::widget::slider::Style {
        rail: iced::widget::slider::Rail {
            backgrounds: (
                iced::Background::Color(color),
                iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.078)),
            ),
            width: 4.0,
            border: Border { radius: 2.0.into(), ..Default::default() },
        },
        handle: iced::widget::slider::Handle {
            shape: iced::widget::slider::HandleShape::Circle { radius: 8.0 },
            background: iced::Background::Color(color),
            border_width: 2.0,
            border_color: Color::WHITE,
        },
    }
}

// ── Pick list styles / Estilos de lista desplegable ──────────

pub fn menu_style(_theme: &iced::Theme) -> iced::overlay::menu::Style {
    iced::overlay::menu::Style {
        background: iced::Background::Color(CARD),
        border: iced::Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        text_color: TEXT,
        selected_text_color: Color::WHITE,
        selected_background: iced::Background::Color(ACCENT),
        shadow: iced::Shadow::default(),
    }
}

pub fn pick_list_style(
    _theme: &iced::Theme,
    _status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    iced::widget::pick_list::Style {
        text_color: TEXT,
        background: iced::Background::Color(Color::WHITE),
        placeholder_color: SUBTEXT,
        handle_color: SUBTEXT,
        border: iced::Border {
            color: BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
    }
}

// ── Text input styles / Estilos de entrada de texto ──────────

pub fn input_style(
    _theme: &iced::Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: iced::Background::Color(Color::WHITE),
        border: Border {
            color: match status {
                iced::widget::text_input::Status::Focused { .. } => ACCENT,
                _ => BORDER,
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        placeholder: SUBTEXT,
        value: TEXT,
        selection: color_with_alpha(ACCENT, 0.3),
        icon: TEXT,
    }
}
