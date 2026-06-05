//! The original "little guy" as a vector [`Content`]. Pure cairo, authored in
//! local coordinates with the origin at his feet; idle bob + blink run off `t`.
//! Entrance/exit motion, placement and squash all live in [`super::Staged`].

use super::content::Content;
use gtk4::cairo;

const BODY_W: f64 = 70.0;
const BODY_H: f64 = 90.0;

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
