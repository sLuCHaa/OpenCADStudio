//! Isolate action menu — Isolate / Hide / End Isolation. Opened from the
//! ISO status pill, rendered as a floating overlay above the status bar
//! (same pattern as the scale picker). The same actions are also offered
//! in the viewport right-click menu.

use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Background, Border, Color, Element, Fill, Length, Rectangle, Theme};

use crate::app::Message;

/// Full-screen overlay: transparent click-catcher + action list pinned
/// bottom-right, above the status bar.
///
/// - `has_selection`: enables Isolate / Hide (they act on the selection).
/// - `isolation_active`: enables End Isolation (something is hidden).
pub fn isolate_popup_overlay(
    has_selection: bool,
    isolation_active: bool,
    pill: Option<Rectangle>,
    win: (f32, f32),
) -> Element<'static, Message> {
    let rows = column![
        action_row(
            "Isolate Objects",
            has_selection,
            Message::Command("ISOLATEOBJECTS".to_string()),
        ),
        action_row(
            "Hide Objects",
            has_selection,
            Message::Command("HIDEOBJECTS".to_string()),
        ),
        action_row(
            "End Isolation",
            isolation_active,
            Message::Command("UNISOLATEOBJECTS".to_string()),
        ),
    ];

    let panel = container(rows)
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

    let positioned = super::position_statusbar_popup(panel.into(), pill, win, 160.0, true);

    mouse_area(positioned)
        .on_press(Message::CloseIsolatePopup)
        .into()
}

fn action_row(label: &'static str, enabled: bool, msg: Message) -> Element<'static, Message> {
    let lbl = text(label)
        .size(11)
        .color(if enabled { LABEL_ON } else { LABEL_OFF });
    let content = row![lbl].align_y(iced::Center);

    let mut btn = button(content)
        .style(move |_: &Theme, status| button::Style {
            background: Some(Background::Color(match (enabled, status) {
                (true, button::Status::Hovered) => ROW_HOVER,
                _ => Color::TRANSPARENT,
            })),
            ..Default::default()
        })
        .width(Fill)
        .padding([4, 12]);
    if enabled {
        btn = btn.on_press(msg);
    }
    btn.into()
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
const LABEL_ON: Color = Color {
    r: 0.92,
    g: 0.92,
    b: 0.92,
    a: 1.0,
};
const LABEL_OFF: Color = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};
