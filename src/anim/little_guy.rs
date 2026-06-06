//! The original "little guy" as a vector [`Content`]. Pure cairo, authored in
//! local coordinates with the origin at his feet; idle bob + blink run off `t`.
//! Entrance/exit motion, placement and squash all live in [`super::Staged`].

use super::content::{Content, Play, SpriteContent};
use gtk4::cairo;

const BODY_W: f64 = 70.0;
const BODY_H: f64 = 90.0;

/// Bake the little guy to a sprite sheet (the shipped default art). The frame
/// places his **feet** at `(36, 90)` and that point is the sheet anchor, so the
/// motion's rest `[1, 1, -58, 37]` seats him exactly as the live vector did —
/// feet just below the card, head peeking over the masked edge. Frames cover a
/// bob (starting just past the t=0 blink); for a one-shot he never wraps.
pub fn little_guy_sheet() -> SpriteContent {
    let frames = 24usize;
    let (fw, fh) = (72i32, 92i32);
    let fps = 12.0;
    let surface =
        cairo::ImageSurface::create(cairo::Format::ARgb32, fw * frames as i32, fh).unwrap();
    {
        let cr = cairo::Context::new(&surface).unwrap();
        let guy = VectorGuy::new();
        for i in 0..frames {
            let t = 0.3 + i as f64 / fps;
            cr.save().unwrap();
            cr.translate(i as f64 * fw as f64 + 36.0, 90.0); // feet
            guy.draw(&cr, t);
            cr.restore().unwrap();
        }
    }
    SpriteContent::new(
        surface,
        fw as f64,
        fh as f64,
        frames,
        frames,
        fps,
        Play::Loop,
        (36.0, 90.0), // anchor = feet
    )
}

pub struct VectorGuy;

impl VectorGuy {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VectorGuy {
    fn default() -> Self {
        Self::new()
    }
}

impl Content for VectorGuy {
    fn natural_size(&self) -> (f64, f64) {
        (BODY_W, BODY_H)
    }

    fn draw(&self, cr: &cairo::Context, t: f64) {
        let bob = (t * 2.4).sin() * 2.0;
        let bt = t % 3.4;
        let blink = if bt < 0.14 {
            (1.0 - (bt / 0.07 - 1.0).abs()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let look = (t * 2.4).cos() * 1.5;

        cr.save().unwrap();
        cr.translate(0.0, bob); // origin = feet; gentle idle bob
        let cx = 0.0;
        let cy = -BODY_H * 0.5;

        // Body: a rounded blob in launcher-pill yellow.
        cr.save().unwrap();
        cr.translate(cx, cy);
        cr.scale(BODY_W * 0.5, BODY_H * 0.5);
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(0.961, 0.773, 0.094, 1.0); // #f5c518
        let _ = cr.fill_preserve();
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.18);
        cr.set_line_width(2.0);
        let _ = cr.stroke();

        // Feet — something for the squash to push against.
        for fx in [-16.0, 16.0] {
            cr.save().unwrap();
            cr.translate(fx, -6.0);
            cr.scale(11.0, 7.0);
            cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
            cr.restore().unwrap();
            cr.set_source_rgba(0.85, 0.66, 0.05, 1.0);
            let _ = cr.fill();
        }

        // Eyes: whites + dark pupils, pupils riding low to peer over the edge.
        let eye_y = cy - 6.0;
        for ex in [-14.0, 14.0] {
            cr.save().unwrap();
            cr.translate(ex, eye_y);
            cr.scale(10.0, 10.0 * (1.0 - blink * 0.92));
            cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
            cr.restore().unwrap();
            cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
            let _ = cr.fill();

            cr.save().unwrap();
            cr.translate(ex + look * 0.6, eye_y + 3.0);
            cr.scale(4.2, 4.2 * (1.0 - blink * 0.92));
            cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
            cr.restore().unwrap();
            cr.set_source_rgba(0.10, 0.10, 0.10, 1.0);
            let _ = cr.fill();
        }

        // Glossy highlight.
        cr.save().unwrap();
        cr.translate(cx - 16.0, cy - 22.0);
        cr.scale(9.0, 6.0);
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.30);
        let _ = cr.fill();

        cr.restore().unwrap();
    }
}
