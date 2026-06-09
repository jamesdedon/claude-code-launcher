//! A Speed-Racer-style driver: an original 8-bit pixel homage (open-face helmet
//! with emblem and chin strap, blue shirt, white pants, orange gloves, red
//! socks, brown loafers) in a three-quarter mid-stride, pumping his fist. The
//! figure is rendered to a tall sprite sheet ([`racer_sheet`]); the launcher
//! reveals it with a vertical *pan* (see [`super::content::PanParams`])
//! configured from the animation manifest, so all the timings live in the
//! `.toml`, not here.

use super::content::{Play, SpriteContent};
use gtk4::cairo;

/// Device pixels per art pixel (matches the F1 car's chunky grid).
const P: f64 = 4.0;
/// Figure art dimensions. Head/helmet near y=0, shoes near y=FIG_H.
const FIG_W: f64 = 22.0;
const FIG_H: f64 = 60.0;
/// Frames in the sheet — a short loop for the raised-fist bob.
const FRAMES: usize = 8;

/// Render the standing racer to a tall sprite sheet (one column per bob frame).
/// Taller than the card; the launcher wraps it with a manifest-driven pan.
pub fn racer_sheet() -> SpriteContent {
    let fw = (FIG_W * P) as i32;
    let fh = (FIG_H * P) as i32;
    let surface =
        cairo::ImageSurface::create(cairo::Format::ARgb32, fw * FRAMES as i32, fh).unwrap();
    {
        let cr = cairo::Context::new(&surface).unwrap();
        cr.set_antialias(cairo::Antialias::None);
        for i in 0..FRAMES {
            // One full pump cycle across the sheet so the loop closes cleanly.
            let ph = i as f64 / FRAMES as f64 * std::f64::consts::TAU;
            cr.save().unwrap();
            cr.translate(i as f64 * fw as f64, 0.0);
            draw_figure(&cr, P, ph);
            cr.restore().unwrap();
        }
    }
    SpriteContent::new(
        surface,
        fw as f64,
        fh as f64,
        FRAMES,
        FRAMES,
        12.0,
        Play::Loop,
        (fw as f64 / 2.0, fh as f64 / 2.0), // anchor unused under pan
    )
    .pixelated(true)
}

/// Paint the racer in art space (origin at the figure's top-left), `p` device px
/// per art pixel. An original 8-bit homage in a **three-quarter mid-stride** —
/// turned ~45° to his right, right leg striding forward, left leg trailing back.
/// `ph` (one TAU per loop) drives a full victory fist-pump: the arm extends
/// skyward, the upper body rises with it, the mouth opens into a cheer at the
/// top, and he blinks on the rest frame.
fn draw_figure(cr: &cairo::Context, p: f64, ph: f64) {
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
    let blue = (0.16, 0.34, 0.70);
    let blue_dk = (0.10, 0.23, 0.50);
    let glove = (0.93, 0.55, 0.12);
    let pants = (0.92, 0.92, 0.95);
    let pants_sh = (0.78, 0.80, 0.86);
    let sock = (0.84, 0.14, 0.14);
    let loafer = (0.45, 0.28, 0.14);
    let loafer_dk = (0.29, 0.17, 0.08);
    let ink = (0.07, 0.07, 0.10);

    // The pump: 0 at rest (fist beside the helmet), 1 at full punch skyward.
    let pump = (1.0 - ph.cos()) / 2.0;
    let lift = -1.0 * pump; // the whole upper body rises with the punch

    // ---- Legs (planted; drawn first, the knees absorb the lift) ----
    // Left (trailing) leg: pushes up-and-back, toe down behind.
    limb(&[(12.6, 34.0), (14.6, 43.0), (16.6, 50.5)], 3.6, pants);
    // Right (lead) leg: knee bent forward, planted down-left.
    limb(&[(10.4, 34.0), (8.0, 43.0), (6.0, 51.5)], 4.0, pants);
    set(pants_sh);
    limb(&[(10.2, 35.0), (8.0, 43.0), (6.2, 50.5)], 1.1, pants_sh); // lead-leg seam
    // Red socks peeking between cuff and shoe.
    set(sock);
    px(4.4, 49.8, 3.6, 2.0); // lead ankle
    px(15.2, 47.4, 3.2, 1.8); // trailing ankle
    // Brown loafers: lead foot forward & low, trailing foot behind & higher.
    set(loafer);
    px(2.8, 51.4, 5.8, 3.4); // lead shoe
    px(15.0, 48.8, 5.2, 3.0); // trailing shoe
    set(loafer_dk);
    px(2.8, 54.3, 5.8, 1.1); // sole
    px(15.0, 51.4, 5.2, 0.9);

    // ---- Everything above the hips rides the pump ----
    cr.save().unwrap();
    cr.translate(0.0, lift * p);

    // ---- Hips / pelvis (tall enough to overlap the leg tops at full lift) ----
    set(pants);
    px(7.2, 30.4, 8.8, 5.4);
    set(pants_sh);
    px(11.2, 31.0, 0.9, 4.4);

    // ---- Torso (blue), turned 3/4: narrower, far edge shaded ----
    set(blue);
    px(6.6, 17.0, 8.4, 13.2);
    set(blue_dk);
    px(12.6, 17.0, 1.4, 13.2); // far (back) side in shadow
    set(ink);
    px(6.6, 29.4, 8.4, 1.1); // belt

    // ---- Near arm (his right): down across the front, a slight counter-sway --
    set(blue);
    px(5.0, 17.6, 2.8, 4.6); // near shoulder/sleeve
    limb(
        &[
            (6.0, 22.0),
            (5.4 - 0.3 * pump, 25.0),
            (6.6 - 0.6 * pump, 27.6),
        ],
        2.3,
        skin,
    );
    set(glove);
    px(5.6 - 0.6 * pump, 26.8, 3.0, 3.0);

    // ---- Far arm (his left): the pump — bent at rest, punched out at the top --
    let ex = 16.2 + 0.6 * pump; // elbow
    let ey = 14.5 - 2.5 * pump;
    let fx = 16.6 + 1.2 * pump; // fist centre
    let fy = 11.0 - 6.5 * pump;
    set(blue);
    px(13.4, 17.2, 2.8, 3.0); // far shoulder/sleeve
    limb(&[(14.8, 18.4), (ex, ey + 0.5)], 2.6, blue); // upper arm in the sleeve
    limb(&[(ex, ey), (fx, fy + 1.6)], 2.3, skin); // forearm
    set(skin_sh);
    px(ex + 0.6, ey - 0.4, 0.7, 3.0); // forearm far-edge shade
    set(glove);
    px(fx - 1.7, fy - 2.0, 3.4, 4.0); // fist
    set(ink);
    px(fx - 1.7, fy + 0.4, 3.4, 0.5); // knuckles

    // ---- Collar + neck (turned) ----
    set(skin);
    px(8.8, 14.8, 3.0, 2.0);
    set(white);
    px(6.6, 16.0, 7.6, 1.8); // collar
    set(blue_dk);
    px(9.4, 16.3, 1.5, 1.2); // collar V

    // ---- Head: an open-face helmet, 3/4 to his right ----
    set(white); // helmet dome
    px(7.0, 0.4, 6.2, 1.0);
    px(6.0, 1.4, 8.2, 1.0);
    px(5.0, 2.4, 9.6, 5.8);
    // Red trim framing the face opening: brow + both cheek columns down to jaw.
    set(red);
    px(5.0, 7.6, 9.6, 1.2); // brow trim
    px(5.0, 8.8, 1.2, 5.6); // near cheek column
    px(13.6, 8.8, 1.0, 5.2); // far cheek column
    // emblem "M", nudged off-centre with the turn
    px(7.0, 3.3, 1.2, 3.4);
    px(10.6, 3.3, 1.2, 3.4);
    px(8.2, 4.5, 1.0, 1.5);
    px(9.7, 4.5, 1.0, 1.5);
    px(8.95, 3.7, 1.0, 1.3);
    // hair fringe under the trim, then the face inside the opening
    set(hair);
    px(6.2, 8.8, 7.4, 1.4);
    set(skin);
    px(6.2, 9.8, 7.4, 5.2);
    set(skin_sh);
    px(12.6, 10.2, 1.0, 4.4); // far cheek in shadow
    set(ink);
    let blink = pump < 0.05; // eyes shut on the rest frame
    if blink {
        px(7.2, 12.0, 1.5, 0.6);
        px(10.9, 12.0, 1.2, 0.6);
    } else {
        px(7.2, 11.4, 1.5, 1.5); // near eye
        px(10.9, 11.5, 1.2, 1.3); // far eye (smaller with the turn)
    }
    if pump > 0.8 {
        px(8.5, 12.9, 1.9, 1.1); // mouth open in a cheer at the top
    } else {
        px(8.5, 13.4, 1.9, 0.6); // mouth
    }
    set(skin_sh);
    px(6.6, 12.2, 0.8, 1.2); // nose hint at the near edge
    // white chin strap closing the helmet under the jaw (joins the red columns)
    set(white);
    px(6.2, 14.4, 7.4, 0.6);

    cr.restore().unwrap();
}

#[cfg(test)]
mod preview {
    use super::*;

    // Render the figure (rest + peak poses, scaled up) and the full sheet to
    // /tmp for eyeballing the pixel art. Ignored by default:
    //   cargo test -p claude-code-launcher --lib racer -- --ignored --nocapture
    #[test]
    #[ignore]
    fn render_preview() {
        let scale = 8.0; // device px per art px, big for inspection
        let fw = (FIG_W * scale) as i32;
        let fh = (FIG_H * scale) as i32;
        let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, fw * 2, fh).unwrap();
        {
            let cr = cairo::Context::new(&surf).unwrap();
            cr.set_source_rgba(0.15, 0.15, 0.18, 1.0);
            let _ = cr.paint();
            cr.set_antialias(cairo::Antialias::None);
            draw_figure(&cr, scale, 0.5); // near-rest, eyes open
            cr.translate(fw as f64, 0.0);
            draw_figure(&cr, scale, std::f64::consts::PI); // pump peak
        }
        let mut f = std::fs::File::create("/tmp/racer_figure.png").unwrap();
        surf.write_to_png(&mut f).unwrap();
        racer_sheet()
            .save_sheet_png(std::path::Path::new("/tmp/racer_sheet.png"))
            .unwrap();
        println!("wrote /tmp/racer_figure.png and /tmp/racer_sheet.png");
    }
}
