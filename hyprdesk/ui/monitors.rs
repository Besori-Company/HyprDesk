// Monitors page — UI for selecting, repositioning and configuring monitors with a drag-and-drop canvas.
// Página de monitores — interfaz para seleccionar, reposicionar y configurar monitores con un canvas de arrastrar y soltar.

use std::collections::HashMap;

use iced::{
    Alignment, Color, Element, Length, Point, Rectangle, Size,
    alignment,
    mouse,
    widget::{canvas, container, pick_list, row, text, text_input},
};

use crate::backend::monitors::{self, Monitor, SCALE_PRESETS, TRANSFORMS_EN, TRANSFORMS_ES};
use crate::i18n::{get_lang, t};
use super::{App, Message};
use super::theme::*;
use super::widgets::*;

// ── Canvas ──────────────────────────────────────────

pub struct MonitorCanvas {
    pub monitors: Vec<Monitor>,
    pub positions: HashMap<String, (i32, i32)>,
    pub selected: String,
}

#[derive(Default, Clone, Debug)]
pub struct CanvasState {
    drag: Option<DragState>,
    live_positions: HashMap<String, (i32, i32)>,
    scale: f64,
    offset: (f64, f64),
}

#[derive(Clone, Debug)]
struct DragState {
    name: String,
    cursor_start: Point,
    position_start: (i32, i32),
    last_pos: (i32, i32),
}

impl canvas::Program<Message> for MonitorCanvas {
    type State = CanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let positions: HashMap<_, _> = self
            .monitors
            .iter()
            .map(|m| {
                let pos = state
                    .live_positions
                    .get(&m.name)
                    .or_else(|| self.positions.get(&m.name))
                    .copied()
                    .unwrap_or((m.x, m.y));
                (m.name.clone(), pos)
            })
            .collect();

        let (sc, (ox, oy)) =
            compute_transform(&self.monitors, &positions, bounds.width as f64, bounds.height as f64);

        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // Background / Fondo
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgba8(10, 10, 12, 0.071),
        );

        for mon in &self.monitors {
            let (px, py) = positions[&mon.name];
            let (mw, mh) = mon.eff_size();
            let rx = (ox + px as f64 * sc) as f32;
            let ry = (oy + py as f64 * sc) as f32;
            let rw = ((mw as f64 * sc) as f32).max(4.0);
            let rh = ((mh as f64 * sc) as f32).max(4.0);
            let sel = mon.name == self.selected;

            let bg_color = if sel {
                Color::from_rgba8(56, 107, 184, 0.878)
            } else {
                Color::from_rgba8(92, 92, 102, 0.800)
            };
            let border_color = if sel {
                Color::from_rgba8(115, 166, 242, 1.0)
            } else {
                Color::from_rgba8(140, 140, 153, 1.0)
            };
            let border_width = if sel { 2.5 } else { 1.5 };

            let path = canvas::Path::rounded_rectangle(
                Point::new(rx, ry),
                Size::new(rw, rh),
                8.0f32.into(),
            );
            frame.fill(&path, bg_color);
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(border_color)
                    .with_width(border_width),
            );

            let short = mon.name.split('-').last().unwrap_or(&mon.name);
            let fs = (8.0f32).max((13.0f32).min(rw / 9.0));

            frame.fill_text(canvas::Text {
                content: short.to_string(),
                position: Point::new(rx + rw / 2.0, ry + rh / 2.0 - fs * 0.8),
                color: Color::WHITE,
                size: iced::Pixels(fs),
                font: iced::Font {
                    weight: iced::font::Weight::ExtraBold,
                    ..NUNITO
                },
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center.into(),
                max_width: f32::INFINITY,
                line_height: iced::widget::text::LineHeight::default(),
                shaping: iced::widget::text::Shaping::default(),
            });

            let res = format!("{}×{}", mon.width, mon.height);
            frame.fill_text(canvas::Text {
                content: res,
                position: Point::new(rx + rw / 2.0, ry + rh / 2.0 + fs * 0.8),
                color: Color::from_rgba8(255, 255, 255, 0.600),
                size: iced::Pixels((6.5f32).max(fs - 3.0)),
                font: NUNITO,
                align_x: alignment::Horizontal::Center.into(),
                align_y: alignment::Vertical::Center.into(),
                max_width: f32::INFINITY,
                line_height: iced::widget::text::LineHeight::default(),
                shaping: iced::widget::text::Shaping::default(),
            });
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(cursor_pos) = cursor.position_in(bounds) else {
                    return None;
                };

                let positions: HashMap<_, _> = self
                    .monitors
                    .iter()
                    .map(|m| {
                        let pos = state
                            .live_positions
                            .get(&m.name)
                            .or_else(|| self.positions.get(&m.name))
                            .copied()
                            .unwrap_or((m.x, m.y));
                        (m.name.clone(), pos)
                    })
                    .collect();

                let (sc, (ox, oy)) = compute_transform(
                    &self.monitors,
                    &positions,
                    bounds.width as f64,
                    bounds.height as f64,
                );
                state.scale = sc;
                state.offset = (ox, oy);

                if let Some(name) = hit_test(&self.monitors, &positions, cursor_pos, sc, (ox, oy)) {
                    let pos = positions[&name];
                    state.drag = Some(DragState {
                        name: name.clone(),
                        cursor_start: cursor_pos,
                        position_start: pos,
                        last_pos: pos,
                    });
                    state.live_positions = positions;

                    if name != self.selected {
                        return Some(canvas::Action::publish(Message::MonitorCanvasSelected(name)));
                    }
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                None
            }

            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if state.drag.is_none() {
                    return None;
                }

                let cursor_pos = cursor
                    .position_in(bounds)
                    .unwrap_or_else(|| state.drag.as_ref().map(|d| d.cursor_start).unwrap_or(Point::ORIGIN));

                let sc = state.scale;
                let drag = state.drag.as_ref().unwrap();
                let dx = (cursor_pos.x - drag.cursor_start.x) as f64 / sc;
                let dy = (cursor_pos.y - drag.cursor_start.y) as f64 / sc;
                let raw_x = (drag.position_start.0 as f64 + dx).round() as i32;
                let raw_y = (drag.position_start.1 as f64 + dy).round() as i32;
                let drag_name = drag.name.clone();
                let position_start = drag.position_start;

                let live = state.live_positions.clone();
                let (sx, sy) = snap_position(&self.monitors, &live, &drag_name, raw_x, raw_y, sc);
                let (rx, ry) = resolve_overlap(&self.monitors, &live, &drag_name, sx, sy, position_start);

                if let Some(d) = &mut state.drag {
                    d.last_pos = (rx, ry);
                }
                state.live_positions.insert(drag_name, (rx, ry));
                Some(canvas::Action::request_redraw().and_capture())
            }

            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(drag) = state.drag.take() {
                    let (fx, fy) = drag.last_pos;
                    state.live_positions.clear();
                    return Some(canvas::Action::publish(Message::MonitorMoved(drag.name, fx, fy)));
                }
                None
            }

            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::default()
        }
    }
}

// ── Canvas helpers / Utilidades del canvas ───────────────────

fn compute_transform(
    monitors: &[Monitor],
    positions: &HashMap<String, (i32, i32)>,
    cw: f64,
    ch: f64,
) -> (f64, (f64, f64)) {
    let pad = 22.0;
    if monitors.is_empty() {
        return (1.0, (pad, pad));
    }
    let min_x = positions.values().map(|(x, _)| *x).min().unwrap_or(0);
    let min_y = positions.values().map(|(_, y)| *y).min().unwrap_or(0);
    let max_x = monitors
        .iter()
        .map(|m| positions[&m.name].0 + m.eff_size().0)
        .max()
        .unwrap_or(1920);
    let max_y = monitors
        .iter()
        .map(|m| positions[&m.name].1 + m.eff_size().1)
        .max()
        .unwrap_or(1080);
    let tw = (max_x - min_x).max(1) as f64;
    let th = (max_y - min_y).max(1) as f64;
    let sc = ((cw - 2.0 * pad) / tw)
        .min((ch - 2.0 * pad) / th)
        .min(1.0);
    let ox = (cw - tw * sc) / 2.0 - min_x as f64 * sc;
    let oy = (ch - th * sc) / 2.0 - min_y as f64 * sc;
    (sc, (ox, oy))
}

fn hit_test(
    monitors: &[Monitor],
    positions: &HashMap<String, (i32, i32)>,
    cursor: Point,
    sc: f64,
    (ox, oy): (f64, f64),
) -> Option<String> {
    for mon in monitors.iter().rev() {
        let (px, py) = positions[&mon.name];
        let (mw, mh) = mon.eff_size();
        let rx = (ox + px as f64 * sc) as f32;
        let ry = (oy + py as f64 * sc) as f32;
        let rw = (mw as f64 * sc) as f32;
        let rh = (mh as f64 * sc) as f32;
        if cursor.x >= rx
            && cursor.x <= rx + rw
            && cursor.y >= ry
            && cursor.y <= ry + rh
        {
            return Some(mon.name.clone());
        }
    }
    None
}

fn snap_position(
    monitors: &[Monitor],
    positions: &HashMap<String, (i32, i32)>,
    name: &str,
    raw_x: i32,
    raw_y: i32,
    sc: f64,
) -> (i32, i32) {
    let threshold = 18.0 / sc.max(0.001);
    let mon = match monitors.iter().find(|m| m.name == name) {
        Some(m) => m,
        None => return (raw_x, raw_y),
    };
    let (mw, mh) = mon.eff_size();
    let mut snap_x = raw_x as f64;
    let mut snap_y = raw_y as f64;
    let mut best_dx = threshold + 1.0;
    let mut best_dy = threshold + 1.0;

    for other in monitors {
        if other.name == name {
            continue;
        }
        let (ox, oy) = positions[&other.name];
        let (ow, oh) = other.eff_size();
        let (ox, oy, ow, oh) = (ox as f64, oy as f64, ow as f64, oh as f64);
        let (rx, ry) = (raw_x as f64, raw_y as f64);

        for (cand, dist) in &[
            (ox + ow, (rx - (ox + ow)).abs()),
            (ox - mw as f64, (rx - (ox - mw as f64)).abs()),
        ] {
            if *dist < best_dx {
                best_dx = *dist;
                snap_x = *cand;
            }
        }
        for (cand, dist) in &[
            (oy + oh, (ry - (oy + oh)).abs()),
            (oy - mh as f64, (ry - (oy - mh as f64)).abs()),
        ] {
            if *dist < best_dy {
                best_dy = *dist;
                snap_y = *cand;
            }
        }
    }

    (
        if best_dx <= threshold { snap_x as i32 } else { raw_x },
        if best_dy <= threshold { snap_y as i32 } else { raw_y },
    )
}

fn resolve_overlap(
    monitors: &[Monitor],
    positions: &HashMap<String, (i32, i32)>,
    name: &str,
    x: i32,
    y: i32,
    start: (i32, i32),
) -> (i32, i32) {
    let mon = match monitors.iter().find(|m| m.name == name) {
        Some(m) => m,
        None => return (x, y),
    };
    let (mw, mh) = mon.eff_size();
    let ddx = x - start.0;
    let ddy = y - start.1;
    let primary_h = ddx.abs() >= ddy.abs();
    let mut rx = x;
    let mut ry = y;

    for _ in 0..monitors.len() {
        let prev = (rx, ry);
        for other in monitors {
            if other.name == name {
                continue;
            }
            let (ox, oy) = positions[&other.name];
            let (ow, oh) = other.eff_size();
            if rx + mw <= ox || rx >= ox + ow || ry + mh <= oy || ry >= oy + oh {
                continue;
            }
            if primary_h {
                rx = if ddx >= 0 { ox - mw } else { ox + ow };
            } else {
                ry = if ddy >= 0 { oy - mh } else { oy + oh };
            }
        }
        if (rx, ry) == prev {
            break;
        }
    }
    (rx, ry)
}

// ── Page view / Vista de página ──────────────────────────────

pub fn view(app: &App) -> Element<'_, Message> {
    if app.monitors.is_empty() {
        return page_content(vec![
            page_title(t("title_monitors")),
            card(
                None::<&str>,
                vec![info_row(Icon::Warning, t("monitor_no_signal"), None::<&str>, AMBER, None)],
            ),
        ]);
    }

    let positions = app.monitor_positions.clone();
    let selected_name = app
        .monitors
        .get(app.selected_monitor)
        .map(|m| m.name.clone())
        .unwrap_or_default();

    let canvas_widget = canvas(MonitorCanvas {
        monitors: app.monitors.clone(),
        positions,
        selected: selected_name.clone(),
    })
    .width(Length::Fill)
    .height(210);

    let canvas_card = card(
        None::<&str>,
        vec![container(canvas_widget)
            .padding(8)
            .width(Length::Fill)
            .into()],
    );

    let mut elements: Vec<Element<_>> = vec![
        page_title(t("title_monitors")),
        canvas_card,
    ];

    if app.monitors.len() > 1 {
        let mon_labels: Vec<String> = app
            .monitors
            .iter()
            .map(|m| format!("{}  —  {}", m.name, m.model.as_deref().unwrap_or(&m.name)))
            .collect();
        let cur_label = mon_labels.get(app.selected_monitor).cloned();
        let mon_labels_c = mon_labels.clone();

        let sel_dd = pick_list(mon_labels, cur_label, move |chosen: String| {
            let idx = mon_labels_c.iter().position(|l| *l == chosen).unwrap_or(0);
            Message::MonitorSelected(idx)
        })
        .width(280)
        .style(pick_list_style)
        .menu_style(menu_style);

        let sel_row = info_row(
            Icon::Monitor,
            t("section_monitor_select"),
            None::<&str>,
            ACCENT,
            Some(sel_dd.into()),
        );
        elements.push(card(Some(t("section_monitor_select")), vec![sel_row]));
    }

    if let Some(mon) = app.monitors.get(app.selected_monitor) {
        elements.extend(monitor_settings(app, mon, &selected_name));
    }

    page_content(elements)
}

fn monitor_settings<'a>(
    app: &'a App,
    mon: &'a Monitor,
    selected_name: &str,
) -> Vec<Element<'a, Message>> {
    let mut out = Vec::new();

    // Resolution & refresh rate / Resolución y frecuencia de actualización
    let friendly: Vec<String> = mon
        .available_modes
        .iter()
        .map(|m| monitors::mode_label(m))
        .collect();
    let cur_friendly = {
        let idx = mon
            .available_modes
            .iter()
            .position(|m| *m == app.monitor_res_mode)
            .unwrap_or_else(|| {
                mon.available_modes
                    .iter()
                    .position(|m| m.starts_with(&format!("{}x{}@", mon.width, mon.height)))
                    .unwrap_or(0)
            });
        friendly.get(idx).cloned()
    };
    let available_modes_c = mon.available_modes.clone();
    let friendly_c = friendly.clone();
    let res_dd = pick_list(friendly, cur_friendly, move |chosen: String| {
        let idx = friendly_c.iter().position(|l| *l == chosen).unwrap_or(0);
        let mode = available_modes_c.get(idx).cloned().unwrap_or_default();
        Message::MonitorResChanged(mode)
    })
    .width(280)
    .style(pick_list_style)
        .menu_style(menu_style);

    out.push(card(
        Some(t("section_resolution")),
        vec![info_row(Icon::Grid, t("section_resolution"), None::<&str>, ACCENT, Some(res_dd.into()))],
    ));

    // Position / Posición
    let pos_inputs = row![
        text("X:").size(13).color(super::theme::SUBTEXT),
        text_input("0", &app.monitor_pos_x)
            .on_input(Message::MonitorPosXChanged)
            .on_submit(Message::MonitorApply)
            .width(100)
            .style(input_style)
            .padding(pad(6.0, 10.0, 6.0, 10.0)),
        text("Y:").size(13).color(super::theme::SUBTEXT),
        text_input("0", &app.monitor_pos_y)
            .on_input(Message::MonitorPosYChanged)
            .on_submit(Message::MonitorApply)
            .width(100)
            .style(input_style)
            .padding(pad(6.0, 10.0, 6.0, 10.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    out.push(card(
        Some(t("section_position")),
        vec![info_row(
            Icon::Move,
            t("section_position"),
            Some(t("row_position_sub")),
            ACCENT,
            Some(pos_inputs.into()),
        )],
    ));

    // Orientation / Orientación
    let lang = get_lang();
    let transforms: &[&str] = if lang == "es" { TRANSFORMS_ES } else { TRANSFORMS_EN };
    let transform_labels: Vec<String> = transforms.iter().map(|s| s.to_string()).collect();
    let cur_transform = transform_labels.get(app.monitor_transform_idx).cloned();
    let transform_c = transform_labels.clone();
    let ori_dd = pick_list(transform_labels, cur_transform, move |chosen: String| {
        let idx = transform_c.iter().position(|l| *l == chosen).unwrap_or(0);
        Message::MonitorTransformChanged(idx)
    })
    .width(280)
    .style(pick_list_style)
        .menu_style(menu_style);

    out.push(card(
        Some(t("section_orientation")),
        vec![info_row(Icon::Rotate, t("section_orientation"), None::<&str>, ACCENT, Some(ori_dd.into()))],
    ));

    // Scale / Escala
    let scale_labels: Vec<String> = SCALE_PRESETS
        .iter()
        .map(|v| {
            if *v == v.floor() {
                format!("{}×", *v as i64)
            } else {
                format!("{v}×")
            }
        })
        .collect();
    let cur_scale = scale_labels.get(app.monitor_scale_idx).cloned();
    let scale_c = scale_labels.clone();
    let sc_dd = pick_list(scale_labels, cur_scale, move |chosen: String| {
        let idx = scale_c.iter().position(|l| *l == chosen).unwrap_or(2);
        Message::MonitorScaleChanged(idx)
    })
    .width(280)
    .style(pick_list_style)
        .menu_style(menu_style);

    out.push(card(
        Some(t("section_scale")),
        vec![info_row(Icon::Expand, t("section_scale"), None::<&str>, ACCENT, Some(sc_dd.into()))],
    ));

    // Primary / Monitor principal
    let primary_name = monitors::get_primary_monitor_name();
    let is_primary = primary_name.as_deref() == Some(selected_name);
    let primary_trailing = if is_primary {
        chip_label(&t("chip_primary"), SUCCESS)
    } else {
        primary_button(&t("btn_set_primary"), Message::MonitorSetPrimary)
    };
    out.push(card(
        Some(t("section_primary")),
        vec![info_row(
            Icon::Star,
            t("row_primary"),
            Some(t("row_primary_sub")),
            AMBER,
            Some(primary_trailing),
        )],
    ));

    // Apply / Aplicar
    let action_row: Element<_> = {
        container(
            row![
                iced::widget::space::horizontal(),
                primary_button(t("btn_apply_monitor"), Message::MonitorApply),
            ]
            .padding(pad(8.0, 0.0, 8.0, 0.0)),
        )
        .width(Length::Fill)
        .into()
    };

    out.push(container(action_row).width(Length::Fill).padding(pad(0.0, 0.0, 12.0, 0.0)).into());

    out
}
