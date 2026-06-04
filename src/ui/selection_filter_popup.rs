//! Selection-filter type picker — choose which entity types are selectable.
//! A checked row means that type can be picked; unchecking it excludes the
//! type from interactive selection. Opened from the FILTER status pill.

use std::collections::HashSet;

use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, Theme};

use crate::app::Message;

/// Full-screen overlay: transparent click-catcher + type list pinned
/// bottom-right, above the status bar.
///
/// - `types`: entity-type names present in the current layout.
/// - `excluded`: types currently filtered out (unchecked).
pub fn selection_filter_popup_overlay(
    types: Vec<String>,
    excluded: &HashSet<String>,
) -> Element<'static, Message> {
    let rows: Vec<Element<'static, Message>> = if types.is_empty() {
        vec![empty_row()]
    } else {
        types
            .into_iter()
            .map(|name| {
                let included = !excluded.contains(&name);
                type_row(name, included)
            })
            .collect()
    };

    let panel = container(column(rows))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(PANEL_BG)),
            border: Border {
                color: PANEL_BORDER,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fixed(160.0));

    let positioned = container(panel)
        .align_right(Fill)
        .align_bottom(Fill)
        .padding(Padding {
            bottom: 27.0,
            right: 4.0,
            top: 0.0,
            left: 0.0,
        })
        .width(Fill)
        .height(Fill);

    mouse_area(positioned)
        .on_press(Message::CloseSelectionFilterPopup)
        .into()
}

fn type_row(name: String, included: bool) -> Element<'static, Message> {
    let check = text(if included { "✓" } else { "  " })
        .size(11)
        .color(if included { CHECK_COLOR } else { Color::TRANSPARENT })
        .width(Length::Fixed(14.0));

    let lbl = text(name.clone())
        .size(11)
        .color(if included { LABEL_ON } else { LABEL_OFF });

    let content = row![check, lbl].spacing(6).align_y(iced::Center);

    button(content)
        .on_press(Message::ToggleSelectionFilterType(name))
        .style(|_: &Theme, status| button::Style {
            background: Some(Background::Color(match status {
                button::Status::Hovered => ROW_HOVER,
                _ => Color::TRANSPARENT,
            })),
            ..Default::default()
        })
        .width(Fill)
        .padding([4, 10])
        .into()
}

fn empty_row() -> Element<'static, Message> {
    container(text("No objects").size(11).color(LABEL_OFF))
        .padding([4, 10])
        .into()
}

// ── Colours ───────────────────────────────────────────────────────────────

const PANEL_BG: Color = Color {
    r: 0.15,
    g: 0.15,
    b: 0.15,
    a: 1.0,
};
const PANEL_BORDER: Color = Color {
    r: 0.32,
    g: 0.32,
    b: 0.32,
    a: 1.0,
};
const ROW_HOVER: Color = Color {
    r: 0.22,
    g: 0.22,
    b: 0.22,
    a: 1.0,
};
const CHECK_COLOR: Color = Color {
    r: 0.35,
    g: 0.75,
    b: 1.00,
    a: 1.0,
};
const LABEL_ON: Color = Color {
    r: 0.92,
    g: 0.92,
    b: 0.92,
    a: 1.0,
};
const LABEL_OFF: Color = Color {
    r: 0.6,
    g: 0.6,
    b: 0.6,
    a: 1.0,
};
