//! 2-D Iced canvas widget for the paper-space view.
//!
//! Replaces the 3-D shader widget used for the paper sheet so that paper-space
//! entities (title blocks, annotations, viewport borders) can be interacted
//! with directly — no need to enter MSPACE just to click on them.
//!
//! A single unified `ViewportPane` shader widget is stacked on top of this
//! canvas to render every content viewport's 3-D model content into its own
//! scissor rectangle — outside those rectangles the shader does not paint,
//! so the paper sheet drawn here shows through.

use iced::widget::canvas;
use iced::{mouse, Color, Point, Rectangle};

use super::hatch_model::{HatchModel, HatchPattern};
use super::Scene;
use crate::app::Message;

// ── PaperCanvas ───────────────────────────────────────────────────────────────

pub struct PaperCanvas<'a> {
    pub scene: &'a Scene,
}

impl<'a> PaperCanvas<'a> {
    pub fn new(scene: &'a Scene) -> Self {
        Self { scene }
    }
}

// ── canvas::Program impl ──────────────────────────────────────────────────────

impl<'a> canvas::Program<Message> for PaperCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        // Update vp_size so viewport_screen_rect() knows the canvas dimensions
        // and can position the MSPACE shader overlay correctly.
        self.scene.selection.borrow_mut().vp_size = (bounds.width, bounds.height);

        let cam = self.scene.camera.borrow();
        let aspect = if bounds.height > 0.0 {
            bounds.width / bounds.height
        } else {
            1.0
        };
        let half_h = cam.ortho_size();
        let half_w = half_h * aspect;
        let tx = cam.target.x;
        let ty = cam.target.y;
        drop(cam);

        // Closure: paper-space world coords → canvas pixel coords.
        let to_px = move |wx: f32, wy: f32| Point {
            x: (wx - tx + half_w) / (2.0 * half_w) * bounds.width,
            y: (ty + half_h - wy) / (2.0 * half_h) * bounds.height,
        };

        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // ── Desk background (area outside the paper sheet) ────────────────────
        const DESK: Color = Color {
            r: 0.22,
            g: 0.24,
            b: 0.28,
            a: 1.0,
        };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), DESK);

        // ── White paper area — use layout paper limits (actual paper size).
        if let Some(((px0, py0), (px1, py1))) = self
            .scene
            .paper_limits()
            .map(|((x0, y0), (x1, y1))| ((x0 as f32, y0 as f32), (x1 as f32, y1 as f32)))
        {
            let tl = to_px(px0, py1);
            let br = to_px(px1, py0);
            let pw = br.x - tl.x;
            let ph = br.y - tl.y;
            if pw > 0.0 && ph > 0.0 {
                let [r, g, b, a] = self.scene.paper_bg_color;
                frame.fill_rectangle(tl, iced::Size::new(pw, ph), Color { r, g, b, a });
            }
        }

        // ── Wipeout fills (rendered before wires, cover background) ──────────
        for hatch in self.scene.paper_canvas_wipeouts().iter() {
            draw_hatch(&mut frame, hatch, &to_px);
        }

        // ── Hatch fills ───────────────────────────────────────────────────────
        for hatch in self.scene.paper_canvas_hatches().iter() {
            draw_hatch(&mut frame, hatch, &to_px);
        }

        // Entity lines, viewport borders and interim/preview wires are now
        // rendered by the GPU shader's full-canvas paper "sheet" viewport, so
        // the 2-D canvas only paints the desk + sheet + fills beneath it.
        vec![frame.into_geometry()]
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Draw one HatchModel (solid, gradient, or pattern) onto `frame`.
/// Patterns and gradients are approximated as solid fills in 2-D canvas mode.
fn draw_hatch(frame: &mut canvas::Frame, hatch: &HatchModel, to_px: &impl Fn(f32, f32) -> Point) {
    if hatch.boundary.is_empty() {
        return;
    }

    let [r, g, b, a] = hatch.color;
    let color = Color { r, g, b, a };

    // Reconstruct offset-rel WCS from stored small offsets + f64 anchor.
    let ox = hatch.world_origin[0] as f32;
    let oy = hatch.world_origin[1] as f32;
    // `boundary` may carry multiple disconnected sub-paths separated by
    // NaN-NaN sentinels (multi-region hatches with islands / holes).
    // Forwarding NaN to lyon's path builder panics
    // (`IncorrectActiveEdgeOrder`) — start a fresh sub-path on each
    // sentinel so each region is closed and filled independently.
    let path = canvas::Path::new(|builder| {
        let mut active = false;
        for &[x, y] in hatch.boundary.iter() {
            if x.is_nan() || y.is_nan() {
                if active {
                    builder.close();
                    active = false;
                }
                continue;
            }
            let p = to_px(x + ox, y + oy);
            if !active {
                builder.move_to(p);
                active = true;
            } else {
                builder.line_to(p);
            }
        }
        if active {
            builder.close();
        }
    });

    match &hatch.pattern {
        HatchPattern::Solid => {
            frame.fill(&path, color);
        }
        HatchPattern::Pattern(_) => {
            // Rasterise the PAT line families clipped to the boundary so
            // the paper canvas shows the actual hatch lines (ANSI31, etc.)
            // instead of just the outline. Matches the GPU shader output
            // for model-space hatches.
            for [a, b] in hatch.pattern_segments() {
                let seg = canvas::Path::new(|builder| {
                    builder.move_to(to_px(a[0], a[1]));
                    builder.line_to(to_px(b[0], b[1]));
                });
                frame.stroke(
                    &seg,
                    canvas::Stroke {
                        style: canvas::Style::Solid(color),
                        width: 1.0,
                        ..Default::default()
                    },
                );
            }
        }
        HatchPattern::Gradient { color2, .. } => {
            // Gradient: average the two colours as a solid fill.
            let avg = Color {
                r: (color.r + color2[0]) * 0.5,
                g: (color.g + color2[1]) * 0.5,
                b: (color.b + color2[2]) * 0.5,
                a: (color.a + color2[3]) * 0.5,
            };
            frame.fill(&path, avg);
        }
    }
}
