//! A Speed-Racer-style driver as a bespoke [`Content`]: an original 8-bit pixel
//! homage (helmet + emblem, blue shirt, white pants, orange gloves, red shoes),
//! drawn taller than the card and revealed by a slow **feet-to-face vertical
//! pan**. The pan is internal to the content (a moving window over the figure),
//! so it survives `fit`/squash — `natural_size` reports only the visible slice.
//! Entrance/exit and placement live in [`super::Staged`].

use super::content::Content;
use gtk4::cairo;

/// Device pixels per art pixel (matches the F1 car's chunky grid).
const P: f64 = 4.0;
/// Figure art dimensions. Head/helmet near y=0, shoes near y=FIG_H.
const FIG_W: f64 = 22.0;
const FIG_H: f64 = 60.0;
/// Height of the visible window (the slice the card shows at once).
const SLICE_H: f64 = 18.0;
/// Seconds the feet→face pan takes before holding on the face.
const PAN_SECS: f64 = 4.0;

pub struct Racer;

impl Racer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Racer {
    fn default() -> Self {
        Self::new()
    }
}

/// Where the top of the visible window sits in figure space at time `t`:
/// starts low on the body (feet) and pans up to the face, then holds.
fn window_top(t: f64) -> f64 {
    let prog = (t / PAN_SECS).clamp(0.0, 1.0);
    let eased = prog * prog * (3.0 - 2.0 * prog); // smoothstep: slow in/out
    let start = FIG_H - SLICE_H; // feet at the bottom of the figure
    let end = -2.0; // overshoot the helmet so the full top lands with headroom
    start + (end - start) * eased
}

impl Content for Racer {
    fn natural_size(&self) -> (f64, f64) {
        // Only the visible slice — so fit scales the *window*, not the whole
        // (much taller) figure.
        (FIG_W * P, SLICE_H * P)
    }

    fn draw(&self, cr: &cairo::Context, t: f64) {
        let half_w = FIG_W * P / 2.0;
        let half_h = SLICE_H * P / 2.0;
        let wt = window_top(t);

        cr.save().unwrap();
        // Clip to the slice (origin = slice centre), then draw the tall figure
        // shifted so figure-row `wt` lands at the top of the slice.
        cr.rectangle(-half_w, -half_h, FIG_W * P, SLICE_H * P);
        cr.clip();
        cr.set_antialias(cairo::Antialias::None);
        cr.translate(-half_w, -wt * P - half_h);
        draw_figure(cr, P, t);
        cr.restore().unwrap();
    }
}

/// Paint the racer in art space (origin at the figure's top-left), `p` device px
/// per art pixel. An original 8-bit homage in a **three-quarter mid-stride** —
/// turned ~45° to his right, right leg striding forward, left leg trailing back,
/// fist raised. `t` drives a little life (raised-fist bob).
fn draw_figure(cr: &cairo::Context, p: f64, t: f64) {
    let px = |x: f64, y: f64, w: f64, h: f64| {
        cr.rectangle(x * p, y * p, w * p, h * p);
        let _ = cr.fill();
    };
    let set = |c: (f64, f64, f64)| cr.set_source_rgba(c.0, c.1, c.2, 1.0);
    // A thick polyline through art-space points — for limbs that bend.
    let limb = |pts: &[(f64, f64)], th: f64, c: (f64, f64, f64)| {
        cr.set_source_rgba(c.0, c.1, c.2, 1.0);
        for seg in pts.windows(2) {
            let (x0, y0) = seg[0];
            let (x1, y1) = seg[1];
            let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0).ceil().max(1.0) as i32;
            for s in 0..=n {
                let f = s as f64 / n as f64;
                let x = x0 + (x1 - x0) * f;
                let y = y0 + (y1 - y0) * f;
                cr.rectangle((x - th / 2.0) * p, (y - th / 2.0) * p, th * p, th * p);
                let _ = cr.fill();
            }
        }
    };

    // Palette.
    let white = (0.95, 0.95, 0.97);
    let red = (0.84, 0.14, 0.14);
    let skin = (0.99, 0.80, 0.62);
    let skin_sh = (0.86, 0.63, 0.46);
    let hair = (0.14, 0.11, 0.09);
    let blue = (0.13, 0.30, 0.64);
    let blue_dk = (0.08, 0.20, 0.46);
    let glove = (0.93, 0.55, 0.12);
    let pants = (0.92, 0.92, 0.95);
    let pants_sh = (0.78, 0.80, 0.86);
    let shoe = (0.80, 0.12, 0.12);
    let ink = (0.07, 0.07, 0.10);

    // Raised fist bobs a touch.
    let fb = (t * 2.2).sin() * 0.6;

    // ---- Legs (drawn first; mid-stride, right forward / left back) ----
    // Left (trailing) leg: pushes up-and-back, toe down behind.
    limb(&[(12.6, 34.0), (14.6, 43.0), (16.6, 50.5)], 3.6, pants);
    // Right (lead) leg: knee bent forward, planted down-left.
    limb(&[(10.4, 34.0), (8.0, 43.0), (6.0, 51.5)], 4.0, pants);
    set(pants_sh);
    limb(&[(10.2, 35.0), (8.0, 43.0), (6.2, 50.5)], 1.1, pants_sh); // lead-leg seam
    // Shoes (red): lead foot forward & low, trailing foot behind & higher (toe).
    set(shoe);
    px(2.8, 51.4, 5.8, 3.8); // lead shoe
    px(15.0, 48.8, 5.2, 3.4); // trailing shoe
    set(white);
    px(2.8, 54.5, 5.8, 0.9); // sole
    px(15.0, 51.4, 5.2, 0.8);

    // ---- Hips / pelvis ----
    set(pants);
    px(7.2, 30.4, 8.8, 5.0);
    set(pants_sh);
    px(11.2, 31.0, 0.9, 4.0);

    // ---- Torso (blue), turned 3/4: narrower, far edge shaded ----
    set(blue);
    px(6.6, 17.0, 8.4, 13.2);
    set(blue_dk);
    px(12.6, 17.0, 1.4, 13.2); // far (back) side in shadow
    set(ink);
    px(6.6, 29.4, 8.4, 1.1); // belt

    // ---- Near arm (his right): down across the front, gloved ----
    set(blue);
    px(5.0, 17.6, 2.8, 4.6); // near shoulder/sleeve
    limb(&[(6.0, 22.0), (5.4, 25.0), (6.6, 27.6)], 2.3, skin); // forearm
    set(glove);
    px(5.6, 26.8, 3.0, 3.0);

    // ---- Far arm (his left): thrown up, fist high ----
    set(blue);
    px(13.4, 17.2, 2.8, 3.0); // far shoulder/sleeve
    limb(&[(15.0, 18.0), (16.8, 12.5), (17.6, 9.0 + fb)], 2.3, skin); // forearm up
    set(skin_sh);
    px(16.9, 11.0 + fb, 0.7, 6.0); // forearm far-edge shade
    set(glove);
    px(16.2, 5.6 + fb, 3.4, 4.0); // fist
    set(ink);
    px(16.2, 8.0 + fb, 3.4, 0.5); // knuckles

    // ---- Collar + neck (turned) ----
    set(skin);
    px(8.8, 14.8, 3.0, 2.0);
    set(white);
    px(6.6, 16.0, 7.6, 1.8); // collar
    set(blue_dk);
    px(9.4, 16.3, 1.5, 1.2); // collar V

    // ---- Head: 3/4 to his right — face mass shifted left, ear on far side ----
    set(white); // helmet dome
    px(7.0, 0.4, 6.2, 1.0);
    px(6.0, 1.4, 8.2, 1.0);
    px(5.0, 2.4, 9.6, 5.8);
    set(red);
    px(5.0, 7.6, 9.6, 1.5); // brow band
    // emblem "M", nudged off-centre with the turn
    set(red);
    px(7.0, 3.3, 1.2, 3.4);
    px(10.6, 3.3, 1.2, 3.4);
    px(8.2, 4.5, 1.0, 1.5);
    px(9.7, 4.5, 1.0, 1.5);
    px(8.95, 3.7, 1.0, 1.3);
    // hair + face (skin biased to the near/left side of the turn)
    set(hair);
    px(5.4, 8.8, 8.2, 1.5); // fringe
    px(5.1, 9.2, 1.1, 2.8); // near sideburn
    set(skin);
    px(5.9, 10.0, 6.8, 5.0);
    set(skin); // far ear poking past the cheek
    px(13.4, 10.2, 1.4, 2.2);
    set(skin_sh);
    px(12.4, 10.4, 0.9, 4.0); // far cheek in shadow
    set(ink);
    px(7.0, 11.4, 1.5, 1.5); // near eye
    px(10.7, 11.5, 1.2, 1.3); // far eye (smaller with the turn)
    px(8.4, 13.4, 2.0, 0.7); // mouth
    set(skin_sh);
    px(6.4, 12.2, 0.8, 1.2); // nose hint at the near edge
    set(ink);
    px(5.0, 8.2, 1.0, 1.8); // helmet front edge
}

#[cfg(test)]
mod preview {
    use super::*;
    use std::f64::consts::TAU;

    // Render the full figure and a feet→face pan filmstrip to /tmp for eyeballing
    // the pixel art. Ignored by default:
    //   cargo test -p claude-code-launcher --lib racer -- --ignored --nocapture
    #[test]
    #[ignore]
    fn render_preview() {
        let _ = TAU;
        let scale = 8.0; // device px per art px, big for inspection

        // Full figure on a checker-ish dark bg so transparency/edges are visible.
        let fw = (FIG_W * scale) as i32;
        let fh = (FIG_H * scale) as i32;
        let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, fw, fh).unwrap();
        {
            let cr = cairo::Context::new(&surf).unwrap();
            cr.set_source_rgba(0.15, 0.15, 0.18, 1.0);
            let _ = cr.paint();
            cr.set_antialias(cairo::Antialias::None);
            draw_figure(&cr, scale, 0.5);
        }
        let mut f = std::fs::File::create("/tmp/racer_figure.png").unwrap();
        surf.write_to_png(&mut f).unwrap();

        // Pan filmstrip: 6 windows from feet (t=0) to face (t>=PAN_SECS).
        let cols = 6;
        let sw = (FIG_W * scale) as i32;
        let sh = (SLICE_H * scale) as i32;
        let strip =
            cairo::ImageSurface::create(cairo::Format::ARgb32, sw * cols, sh + 8).unwrap();
        {
            let cr = cairo::Context::new(&strip).unwrap();
            cr.set_source_rgba(0.15, 0.15, 0.18, 1.0);
            let _ = cr.paint();
            cr.set_antialias(cairo::Antialias::None);
            for i in 0..cols {
                let t = (i as f64 / (cols - 1) as f64) * PAN_SECS;
                let wt = window_top(t);
                cr.save().unwrap();
                cr.translate(i as f64 * sw as f64, 4.0);
                cr.rectangle(0.0, 0.0, FIG_W * scale, SLICE_H * scale);
                cr.clip();
                cr.translate(0.0, -wt * scale);
                draw_figure(&cr, scale, t);
                cr.restore().unwrap();
            }
        }
        let mut f2 = std::fs::File::create("/tmp/racer_pan.png").unwrap();
        strip.write_to_png(&mut f2).unwrap();

        println!("wrote /tmp/racer_figure.png and /tmp/racer_pan.png");
    }
}
