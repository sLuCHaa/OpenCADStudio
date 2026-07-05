pub mod cycle_popup;
pub mod isolate_popup;
pub mod scale_popup;
pub mod selection_filter_popup;
pub mod snap_popup;
pub mod units_popup;

use iced::widget::container;
use iced::{Element, Fill, Padding, Rectangle};

use crate::app::Message;

/// Decide a status-bar popup's horizontal anchor and paddings from its pill's
/// screen bounds. Returns `(align_right, horizontal_pad, bottom_pad)`.
///
/// The popup opens just above the pill. Horizontally it prefers `prefer_right`
/// (right edge aligned to the pill's right, growing left) but flips to the other
/// side when that direction would cross the window edge.
pub fn popup_anchor(
    pill: Option<Rectangle>,
    win: (f32, f32),
    popup_w: f32,
    prefer_right: bool,
) -> (bool, f32, f32) {
    let Some(b) = pill else {
        // No recorded position yet — fall back to the bottom-right corner.
        return (prefer_right, 4.0, 27.0);
    };
    let bottom = (win.1 - b.y).max(0.0);
    let right_pad = (win.0 - (b.x + b.width)).max(4.0);
    let left_pad = b.x.max(4.0);

    if prefer_right {
        // Right-aligned popup grows left; flip left-aligned if it would pass the
        // left window edge.
        if (b.x + b.width) - popup_w < 2.0 {
            (false, left_pad, bottom)
        } else {
            (true, right_pad, bottom)
        }
    } else {
        // Left-aligned popup grows right; flip right-aligned if it would pass the
        // right window edge.
        if b.x + popup_w > win.0 - 2.0 {
            (true, right_pad, bottom)
        } else {
            (false, left_pad, bottom)
        }
    }
}

/// Wrap a status-bar popup `panel` in a full-window container positioned just
/// above its pill, flipping horizontally to stay on screen.
pub fn position_statusbar_popup<'a>(
    panel: Element<'a, Message>,
    pill: Option<Rectangle>,
    win: (f32, f32),
    popup_w: f32,
    prefer_right: bool,
) -> Element<'a, Message> {
    let (align_right, h_pad, bottom) = popup_anchor(pill, win, popup_w, prefer_right);
    let pad = Padding {
        top: 0.0,
        bottom,
        left: if align_right { 0.0 } else { h_pad },
        right: if align_right { h_pad } else { 0.0 },
    };
    let c = container(panel)
        .align_bottom(Fill)
        .width(Fill)
        .height(Fill)
        .padding(pad);
    let c = if align_right {
        c.align_right(Fill)
    } else {
        c.align_left(Fill)
    };
    c.into()
}
