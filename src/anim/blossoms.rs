//! Generator for the cherry-blossom sprite sheet: a continuous field of petals
//! drifting top → bottom, varying in size and in four pink shade styles, with a
//! gentle sway and slow tumble. This is a **one-time asset generator** — it bakes
//! a *seamless looping* PNG that ships as `anims/blossoms.png`, played by the
//! generic sheet engine via `cherry_blossoms.toml`. Nothing here runs at draw
//! time; the animation itself is fully specified by its `.toml` + the PNG.

use super::content::{Play, SpriteContent};
use gtk4::cairo;
use std::f64::consts::TAU;

/// Frame size (the field, matching the prompt — wide and short).
const W: f64 = 540.0;
const H: f64 = 110.0;
/// Blossoms aloft at once.
const N: usize = 16;
/// Loop: frames, columns in the sheet grid, and playback rate.
const FRAMES: usize = 48;
const COLS: usize = 8;
const FPS: f64 = 12.0;

/// Cheap deterministic [0,1) hash so each blossom keeps a stable lane/size/phase.
fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Petal + centre colour for one of four shade styles.
fn palette(style: usize) -> ((f64, f64, f64), (f64, f64, f64)) {
    match style % 4 {
        0 => ((1.00, 0.80, 0.87), (0.85, 0.30, 0.46)), // light pink
        1 => ((0.98, 0.67, 0.80), (0.78, 0.22, 0.42)), // mid pink
        2 => ((1.00, 0.92, 0.95), (0.90, 0.52, 0.64)), // pale / near-white
        _ => ((0.95, 0.56, 0.75), (0.70, 0.16, 0.40)), // deep pink
    }
}

/// Draw one blossom: five petals around a reddish stamen centre.
fn draw_blossom(cr: &cairo::Context, cx: f64, cy: f64, r: f64, style: usize, rot: f64) {
    let (pet, cen) = palette(style);
    let a = 0.92; // slight translucency so prompt text reads through

    for k in 0..5 {
        let ang = rot + k as f64 * TAU / 5.0;
        let px = cx + ang.cos() * r * 0.66;
        let py = cy + ang.sin() * r * 0.66;
        cr.save().unwrap();
        cr.translate(px, py);
        cr.rotate(ang);
        cr.scale(r * 0.62, r * 0.42);
        cr.arc(0.0, 0.0, 1.0, 0.0, TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(pet.0, pet.1, pet.2, a);
        let _ = cr.fill();
    }

    cr.save().unwrap();
    cr.translate(cx, cy);
    cr.scale(r * 0.34, r * 0.34);
    cr.arc(0.0, 0.0, 1.0, 0.0, TAU);
    cr.restore().unwrap();
    cr.set_source_rgba(cen.0, cen.1, cen.2, a);
    let _ = cr.fill();

    for k in 0..5 {
        let ang = rot * 1.3 + k as f64 * TAU / 5.0 + 0.4;
        let sx = cx + ang.cos() * r * 0.20;
        let sy = cy + ang.sin() * r * 0.20;
        cr.save().unwrap();
        cr.translate(sx, sy);
        cr.scale(r * 0.09, r * 0.09);
        cr.arc(0.0, 0.0, 1.0, 0.0, TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(0.50, 0.08, 0.20, a);
        let _ = cr.fill();
    }
}

/// Draw the whole field at time `t`, in centred coords ([-W/2,W/2]×[-H/2,H/2]).
/// Per-blossom speeds, sway, and tumble are quantised to whole cycles over
/// `period`, so the field at `t == 0` and `t == period` is identical — a seamless
/// loop with no visible seam.
fn draw_field(cr: &cairo::Context, t: f64, period: f64) {
    for i in 0..N {
        let fi = i as f64;
        let style = i % 4;
        let size = 9.0 + (i % 4) as f64 * 2.5 + frac(fi * 0.618) * 3.0;
        let pad = size + 4.0;
        let cycle = H + 2.0 * pad;

        // Fall speed → whole wraps over the period.
        let base_speed = 16.0 + frac(fi * 0.371) * 26.0;
        let wraps = (base_speed * period / cycle).round().max(1.0);
        let speed = wraps * cycle / period;
        let offset = frac(fi * 0.197) * cycle;
        let y = -H / 2.0 - pad + (t * speed + offset).rem_euclid(cycle);

        // Lane + sway (whole sine periods over the loop).
        let lane = -W / 2.0 + (fi + 0.5) / N as f64 * W;
        let x0 = lane + (frac(fi * 0.917) - 0.5) * 36.0;
        let sway_amp = 7.0 + (i % 3) as f64 * 5.0;
        let sway_cycles = ((0.5 + frac(fi * 0.713) * 0.7) * period / TAU).round().max(1.0);
        let sway_freq = TAU * sway_cycles / period;
        let phase = fi * 1.3;
        let x = x0 + (t * sway_freq + phase).sin() * sway_amp;

        // Slow tumble (whole turns over the loop).
        let turns = (0.5 * period / TAU).round().max(1.0);
        let rot = t * (TAU * turns / period) + phase;

        draw_blossom(cr, x, y, size, style, rot);
    }
}

/// Bake the seamless looping field into a sprite sheet (the shipped default art).
pub fn blossoms_sheet() -> SpriteContent {
    let rows = FRAMES.div_ceil(COLS);
    let sw = (W * COLS as f64) as i32;
    let sh = (H * rows as f64) as i32;
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, sw, sh).unwrap();
    let period = FRAMES as f64 / FPS;
    {
        let cr = cairo::Context::new(&surface).unwrap();
        for f in 0..FRAMES {
            let col = (f % COLS) as f64;
            let row = (f / COLS) as f64;
            cr.save().unwrap();
            cr.translate(col * W + W / 2.0, row * H + H / 2.0);
            draw_field(&cr, f as f64 / FPS, period);
            cr.restore().unwrap();
        }
    }
    SpriteContent::new(
        surface,
        W,
        H,
        COLS,
        FRAMES,
        FPS,
        Play::Loop,
        (W / 2.0, H / 2.0),
    )
}

#[cfg(test)]
mod preview {
    use super::*;
    use std::path::Path;

    // Bake the sheet + render one frame to /tmp for eyeballing. Ignored:
    //   cargo test -p claude-code-launcher --lib blossoms -- --ignored --nocapture
    #[test]
    #[ignore]
    fn render_preview() {
        blossoms_sheet()
            .save_sheet_png(Path::new("/tmp/blossoms.png"))
            .unwrap();

        // One frame, scaled up, on a dark card.
        let scale = 1.6;
        let sw = (W * scale) as i32;
        let sh = (H * scale) as i32;
        let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, sw, sh).unwrap();
        {
            let cr = cairo::Context::new(&surf).unwrap();
            cr.set_source_rgba(0.12, 0.10, 0.14, 1.0);
            let _ = cr.paint();
            cr.translate(sw as f64 / 2.0, sh as f64 / 2.0);
            cr.scale(scale, scale);
            draw_field(&cr, 2.4, FRAMES as f64 / FPS);
        }
        surf.write_to_png(&mut std::fs::File::create("/tmp/blossoms_frame.png").unwrap())
            .unwrap();
        println!("wrote /tmp/blossoms.png and /tmp/blossoms_frame.png");
    }
}
