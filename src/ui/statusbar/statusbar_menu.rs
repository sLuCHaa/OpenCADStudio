//! Status-bar customization menu — a checkmark list of every toggle pill,
//! opened from the bar's far-right handle. Rendered as a floating overlay
//! above the status bar, same pattern as the scale picker.

use iced::widget::{button, column, container, mouse_area, row, scrollable, text};
use iced::{Background, Border, Color, Element, Fill, Length, Rectangle, Theme};

use crate::app::Message;
use crate::ui::statusbar::statusbar_config::{StatusBarConfig, StatusPill};

/// Full-screen overlay: transparent click-catcher + the menu panel pinned to
/// the bottom-right, just above the status bar.
pub fn statusbar_menu_overlay(
    config: &StatusBarConfig,
    pill: Option<Rectangle>,
    win: (f32, f32),
) -> Element<'static, Message> {
    let rows: Vec<Element<'static, Message>> = StatusPill::ALL
        .iter()
        .map(|&pill| {
            menu_row(
                pill.label(),
                config.is_visible(pill),
                Message::ToggleStatusPill(pill),
            )
        })
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
        .width(Length::Fixed(200.0));

    let positioned =
        crate::ui::popup::position_statusbar_popup(panel.into(), pill, win, 200.0, true);

    mouse_area(positioned)
        .on_press(Message::CloseStatusBarMenu)
        .into()
}

/// Dropdown listing Model + every paper layout, opened from the leftmost
/// hamburger. Pinned bottom-left just above the status bar; a click selects a
/// layout (and closes), an outside click just closes.
pub fn layout_list_overlay<'a>(
    layouts: &[String],
    current: &str,
    pill: Option<Rectangle>,
    win: (f32, f32),
) -> Element<'a, Message> {
    let rows: Vec<Element<'a, Message>> = layouts
        .iter()
        .map(|name| layout_row(name.clone(), name == current))
        .collect();

    let panel = container(scrollable(column(rows)))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(PANEL_BG)),
            border: Border {
                color: PANEL_BORDER,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .width(Length::Fixed(200.0))
        .max_height(360.0);

    // Hamburger is on the left → prefer left-aligned (grows right).
    let positioned =
        crate::ui::popup::position_statusbar_popup(panel.into(), pill, win, 200.0, false);

    mouse_area(positioned)
        .on_press(Message::CloseLayoutList)
        .into()
}

fn layout_row<'a>(name: String, is_current: bool) -> Element<'a, Message> {
    let lbl = text(name.clone())
        .size(11)
        .color(if is_current { LABEL_ON } else { LABEL_OFF });
    button(row![lbl].align_y(iced::Center))
        .on_press(Message::LayoutSwitch(name))
        .style(move |_: &Theme, status| button::Style {
            background: Some(Background::Color(match (is_current, status) {
                (_, button::Status::Hovered) => ROW_HOVER,
                (true, _) => Color {
                    r: 0.18,
                    g: 0.26,
                    b: 0.36,
                    a: 1.0,
                },
                _ => Color::TRANSPARENT,
            })),
            ..Default::default()
        })
        .width(Fill)
        .padding([4, 12])
        .into()
}

fn menu_row(label: &'static str, checked: bool, msg: Message) -> Element<'static, Message> {
    let check = crate::ui::icons::check_cell(checked, CHECK_COLOR);

    let lbl = text(label)
        .size(11)
        .color(if checked { LABEL_ON } else { LABEL_OFF });

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
