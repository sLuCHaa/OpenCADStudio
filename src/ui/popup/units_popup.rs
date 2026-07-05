//! Drawing-units picker — sets the INSUNITS header value. Rendered as a
//! floating overlay above the status bar, same pattern as the scale picker.

use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Background, Border, Color, Element, Fill, Length, Rectangle, Theme};

use crate::app::Message;

/// Units offered in the picker: (INSUNITS code, menu label).
const UNITS: &[(i16, &str)] = &[
    (0, "Unitless"),
    (4, "Millimeters"),
    (5, "Centimeters"),
    (6, "Meters"),
    (7, "Kilometers"),
    (1, "Inches"),
    (2, "Feet"),
    (3, "Miles"),
    (10, "Yards"),
];

/// Short label shown on the status-bar pill for an INSUNITS code.
pub fn unit_short(code: i16) -> &'static str {
    match code {
        1 => "in",
        2 => "ft",
        3 => "mi",
        4 => "mm",
        5 => "cm",
        6 => "m",
        7 => "km",
        10 => "yd",
        0 => "Unitless",
        _ => "Unit",
    }
}

/// Full-screen overlay: transparent click-catcher + units list pinned
/// bottom-right, above the status bar.
pub fn units_popup_overlay(
    current: i16,
    pill: Option<Rectangle>,
    win: (f32, f32),
) -> Element<'static, Message> {
    let rows: Vec<Element<'static, Message>> = UNITS
        .iter()
        .map(|&(code, label)| unit_row(label, code == current, Message::SetDrawingUnits(code)))
        .collect();

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
        .width(Length::Fixed(140.0));

    let positioned = super::position_statusbar_popup(panel.into(), pill, win, 140.0, true);

    mouse_area(positioned)
        .on_press(Message::CloseUnitsPopup)
        .into()
}

fn unit_row(label: &'static str, active: bool, msg: Message) -> Element<'static, Message> {
    let check = crate::ui::icons::check_cell(active, CHECK_COLOR);

    let lbl = text(label)
        .size(11)
        .color(if active { LABEL_ON } else { LABEL_OFF });

    let content = row![check, lbl].spacing(6).align_y(iced::Center);

    button(content)
        .on_press(msg)
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
    r: 0.65,
    g: 0.65,
    b: 0.65,
    a: 1.0,
};
