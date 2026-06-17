//! Plugin Manager window — lists the add-ons compiled into this build and lets
//! the user enable/disable each one. A disabled plugin keeps its manifest
//! listed but drops its ribbon tab and command dispatch (persisted across
//! launches). Dynamic loading still comes with the phase-2 loader; see
//! `docs/plugin-architecture.md`.

use crate::app::Message;
use crate::plugin::manifest::PluginManifest;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Background, Border, Color, Element, Fill, Theme};
use rustc_hash::FxHashSet;

// Register the command names for autocomplete.
inventory::submit!(crate::command::CommandRegistration {
    names: &["PLUGINS", "PLUGINMANAGER"]
});

const BG: Color = Color {
    r: 0.15,
    g: 0.15,
    b: 0.15,
    a: 1.0,
};
const CARD: Color = Color {
    r: 0.12,
    g: 0.12,
    b: 0.12,
    a: 1.0,
};
const BORDER: Color = Color {
    r: 0.30,
    g: 0.30,
    b: 0.30,
    a: 1.0,
};
const DIM: Color = Color {
    r: 0.55,
    g: 0.55,
    b: 0.55,
    a: 1.0,
};
const ACCENT: Color = Color {
    r: 0.30,
    g: 0.62,
    b: 0.95,
    a: 1.0,
};
const WHITE: Color = Color {
    r: 0.92,
    g: 0.92,
    b: 0.92,
    a: 1.0,
};

fn badge<'a>(label: String) -> Element<'a, Message> {
    container(text(label).size(11).color(WHITE))
        .padding([2, 8])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(Color {
                r: 0.20,
                g: 0.34,
                b: 0.52,
                a: 1.0,
            })),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn toggle_button<'a>(id: &str, disabled: bool) -> Element<'a, Message> {
    // Label shows the action the click performs.
    let (label, on, off) = if disabled {
        ("Enable", Color { r: 0.18, g: 0.5, b: 0.25, a: 1.0 }, Color { r: 0.22, g: 0.6, b: 0.3, a: 1.0 })
    } else {
        ("Disable", Color { r: 0.4, g: 0.22, b: 0.22, a: 1.0 }, Color { r: 0.55, g: 0.28, b: 0.28, a: 1.0 })
    };
    let want_enabled = disabled; // clicking flips the state
    let id_owned = id.to_string();
    button(text(label).size(12).color(WHITE))
        .padding([3, 12])
        .on_press(Message::SetPluginEnabled(id_owned, want_enabled))
        .style(move |_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => off,
                _ => on,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: WHITE,
                border: Border { radius: 4.0.into(), ..Default::default() },
                ..Default::default()
            }
        })
        .into()
}

fn plugin_card<'a>(m: &PluginManifest, disabled: bool) -> Element<'a, Message> {
    let mut header = row![text(m.name.to_string()).size(15).color(WHITE)];
    if disabled {
        header = header.push(Space::new().width(8));
        header = header.push(badge("Disabled".to_string()));
    }
    let header = header
        .push(Space::new().width(Fill))
        .push(badge(format!("v{}", m.version)))
        .push(Space::new().width(8))
        .push(badge(format!("API {}", m.api_version.major)))
        .push(Space::new().width(10))
        .push(toggle_button(m.id, disabled))
        .align_y(iced::Center);

    let id_line = text(m.id.to_string()).size(11).color(ACCENT);
    let desc = text(m.description.to_string()).size(12).color(DIM);

    let mut body = column![header, id_line, desc].spacing(5);

    if !m.command_prefixes.is_empty() {
        body = body.push(
            text(format!("Commands: {}", m.command_prefixes.join(", ")))
                .size(11)
                .color(DIM),
        );
    }

    container(body.padding([12, 14]))
        .width(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(CARD)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}

pub fn view_window<'a>(
    plugins: &[&'static PluginManifest],
    disabled: &FxHashSet<String>,
) -> Element<'a, Message> {
    let title = text("Installed Plugins").size(20).color(WHITE);
    let subtitle = text(format!(
        "{} add-on{} compiled into this build",
        plugins.len(),
        if plugins.len() == 1 { "" } else { "s" }
    ))
    .size(12)
    .color(DIM);

    let body: Element<'_, Message> = if plugins.is_empty() {
        container(text("No plugins installed.").size(13).color(DIM))
            .padding(20)
            .into()
    } else {
        let mut list = column![].spacing(10);
        for m in plugins {
            list = list.push(plugin_card(m, disabled.contains(m.id)));
        }
        scrollable(list.width(Fill)).height(Fill).into()
    };

    container(
        column![title, subtitle, Space::new().height(12), body]
            .spacing(4)
            .padding(20)
            .width(Fill)
            .height(Fill),
    )
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(BG)),
        ..Default::default()
    })
    .width(Fill)
    .height(Fill)
    .into()
}
