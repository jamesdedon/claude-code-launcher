//! Appearance: the [`Content`] trait, implemented identically by sprite and
//! vector animations.

use gtk4::cairo;

/// What an animation *looks like*, decoupled from how it moves. Implemented by
/// both [`SpriteContent`] and vector content (e.g. [`super::VectorGuy`]) — the
/// motion layer never knows or cares which.
pub trait Content {
    /// Logical footprint, in local units. Used for layout hints.
    fn natural_size(&self) -> (f64, f64);

    /// Draw in local coordinates with the origin at the content's anchor (the
    /// point that sits at the rest position and around which squash pivots).
    /// `t` is seconds since first shown — **unbounded**, so frames/idle can run
    /// forever.
    fn draw(&self, cr: &cairo::Context, t: f64);
}

/// How a finite sprite sheet maps onto unbounded time.
#[derive(Debug, Clone, Copy)]
pub enum Play {
    /// Loop forever (infinite steps).
    Loop,
    /// Play once and hold the last frame.
    Once,
    /// Bounce back and forth forever.
    PingPong,
}

/// A sprite-sheet animation. Frames are blitted from a single backing surface;
/// nothing here is privileged relative to vector content — it implements the
/// same [`Content`] trait and rides the same motion pipeline.
pub struct SpriteContent {
    sheet: cairo::ImageSurface,
    fw: f64,
    fh: f64,
    cols: usize,
    frames: usize,
    fps: f64,
    play: Play,
    /// Anchor within a frame, in frame pixels (e.g. bottom-center = feet).
    anchor: (f64, f64),
    /// Sample with nearest-neighbour (crisp pixels) instead of smoothing.
    pixelated: bool,
}

impl SpriteContent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sheet: cairo::ImageSurface,
        fw: f64,
        fh: f64,
        cols: usize,
        frames: usize,
        fps: f64,
        play: Play,
        anchor: (f64, f64),
    ) -> Self {
        Self {
            sheet,
            fw,
            fh,
            cols,
            frames,
            fps,
            play,
            anchor,
            pixelated: false,
        }
    }

    fn frame_index(&self, t: f64) -> usize {
        if self.frames <= 1 {
            return 0;
        }
        let raw = (t * self.fps).floor() as i64;
        match self.play {
            Play::Loop => raw.rem_euclid(self.frames as i64) as usize,
            Play::Once => raw.clamp(0, self.frames as i64 - 1) as usize,
            Play::PingPong => {
                let p = self.frames as i64 - 1;
                let m = raw.rem_euclid(2 * p);
                (if m <= p { m } else { 2 * p - m }) as usize
            }
        }
    }

    /// A procedurally-generated demo sheet — a rotating square — so a *sprite*
    /// animation can be exercised through the same pipeline as the vector guy
    /// without shipping an external asset. Real sprites would load a PNG here.
    pub fn spinner() -> Self {
        let frames = 24usize;
        let (fw, fh) = (72i32, 72i32);
        let cols = frames;
        let surface =
            cairo::ImageSurface::create(cairo::Format::ARgb32, fw * cols as i32, fh).unwrap();
        {
            let cr = cairo::Context::new(&surface).unwrap();
            for i in 0..frames {
                let cx = i as f64 * fw as f64 + fw as f64 / 2.0;
                let cy = fh as f64 / 2.0;
                let ang = (i as f64 / frames as f64) * std::f64::consts::TAU;
                cr.save().unwrap();
                cr.translate(cx, cy);
                cr.rotate(ang);
                cr.rectangle(-20.0, -20.0, 40.0, 40.0);
                cr.set_source_rgba(0.961, 0.773, 0.094, 1.0); // pill yellow
                let _ = cr.fill_preserve();
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
                cr.set_line_width(2.0);
                let _ = cr.stroke();
                // a dot so the rotation is legible
                cr.rectangle(-4.0, -16.0, 8.0, 8.0);
                cr.set_source_rgba(0.1, 0.1, 0.1, 1.0);
                let _ = cr.fill();
                cr.restore().unwrap();
            }
        }
        Self::new(
            surface,
            fw as f64,
            fh as f64,
            cols,
            frames,
            24.0,
            Play::Loop,
            (fw as f64 / 2.0, fh as f64 / 2.0),
        )
    }

    /// A procedurally-generated 8-bit F1 car (side view, pointing right) with
    /// spinning wheels, a flickering exhaust, and a revving body shake. Pairs
    /// with an under-damped spring + enter-left/exit-right approach so it pops
    /// in fighting for grip and then takes off.
    pub fn f1_car() -> Self {
        let frames = 8usize;
        let p = 4.0; // device px per art pixel
        let (art_w, art_h) = (35.0, 18.0);
        let fw = (art_w * p) as i32; // 140
        let fh = (art_h * p) as i32; // 72
        let cols = frames;
        let surface =
            cairo::ImageSurface::create(cairo::Format::ARgb32, fw * cols as i32, fh).unwrap();
        {
            let cr = cairo::Context::new(&surface).unwrap();
            cr.set_antialias(cairo::Antialias::None); // hard pixel edges
            for i in 0..frames {
                draw_f1_frame(&cr, i as f64 * fw as f64, p, i);
            }
        }
        let mut s = Self::new(
            surface,
            fw as f64,
            fh as f64,
            cols,
            frames,
            12.0,
            Play::Loop,
            (fw as f64 / 2.0, 46.0), // roughly the car's centre, on its wheels
        );
        s.pixelated = true;
        s
    }
}

/// Paint one 8-bit F1 frame into the sheet at horizontal offset `ox`. `p` is
/// device pixels per art pixel; `frame` drives wheels, exhaust, and the shake.
fn draw_f1_frame(cr: &cairo::Context, ox: f64, p: f64, frame: usize) {
    // Fill an art-space rect (chunky pixel) with the current source colour.
    let px = |x: f64, y: f64, w: f64, h: f64| {
        cr.rectangle(ox + x * p, y * p, w * p, h * p);
        let _ = cr.fill();
    };
    // Revving shake: the body bounces ±1 art px on its suspension.
    let vy = match frame % 4 {
        1 => -1.0,
        3 => 1.0,
        _ => 0.0,
    };

    // Ground contact shadow.
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.18);
    px(7.0, 14.6, 21.0, 1.2);

    // Wheels (steady; the body shakes around them).
    for &cx in &[10.0, 26.0] {
        cr.set_source_rgba(0.10, 0.10, 0.12, 1.0); // tyre
        px(cx - 2.5, 8.0, 5.0, 7.0);
        px(cx - 3.5, 9.0, 7.0, 5.0);
        cr.set_source_rgba(0.60, 0.60, 0.62, 1.0); // hub
        px(cx - 1.0, 10.5, 2.0, 2.0);
        cr.set_source_rgba(0.85, 0.85, 0.85, 1.0); // spinning spoke highlight
        match frame % 4 {
            0 => px(cx - 0.5, 8.5, 1.0, 2.0),
            1 => px(cx + 1.0, 10.5, 2.0, 1.0),
            2 => px(cx - 0.5, 12.5, 1.0, 2.0),
            _ => px(cx - 3.0, 10.5, 2.0, 1.0),
        }
    }

    // Exhaust flame trailing off the back (left), flickering.
    let (fc, fx, fw2) = match frame % 3 {
        0 => ((1.0, 0.85, 0.2), 1.0, 1.6),
        1 => ((1.0, 0.55, 0.1), 0.3, 2.4),
        _ => ((1.0, 0.72, 0.15), 0.0, 3.2),
    };
    cr.set_source_rgba(fc.0, fc.1, fc.2, 1.0);
    px(fx, 7.6 + vy, fw2, 1.6);

    // Rear wing.
    cr.set_source_rgba(0.15, 0.15, 0.17, 1.0);
    px(2.0, 3.5 + vy, 4.0, 1.2); // top plane
    px(2.5, 3.5 + vy, 1.5, 5.0); // endplate

    // Body: rear structure, main tub, floor, nose.
    cr.set_source_rgba(0.55, 0.08, 0.10, 1.0);
    px(4.0, 7.0 + vy, 4.0, 4.0);
    cr.set_source_rgba(0.82, 0.12, 0.15, 1.0);
    px(6.0, 8.0 + vy, 22.0, 3.0);
    cr.set_source_rgba(0.55, 0.08, 0.10, 1.0);
    px(8.0, 10.5 + vy, 17.0, 1.5);
    cr.set_source_rgba(0.82, 0.12, 0.15, 1.0);
    px(28.0, 8.5 + vy, 4.0, 2.0); // nose
    px(31.5, 9.0 + vy, 2.5, 1.0); // nose tip

    // Cockpit hump, halo, helmet.
    cr.set_source_rgba(0.82, 0.12, 0.15, 1.0);
    px(14.0, 6.5 + vy, 6.0, 2.0);
    cr.set_source_rgba(0.15, 0.15, 0.17, 1.0);
    px(14.5, 5.0 + vy, 1.0, 2.5); // roll hoop / halo
    cr.set_source_rgba(0.15, 0.35, 0.85, 1.0); // helmet
    px(15.5, 5.5 + vy, 3.0, 2.0);
    cr.set_source_rgba(0.95, 0.95, 0.95, 1.0); // helmet stripe
    px(15.5, 5.5 + vy, 3.0, 0.7);

    // Front wing.
    cr.set_source_rgba(0.15, 0.15, 0.17, 1.0);
    px(29.0, 11.0 + vy, 5.0, 1.0);
    px(33.0, 10.0 + vy, 1.2, 2.0); // endplate
}

impl Content for SpriteContent {
    fn natural_size(&self) -> (f64, f64) {
        (self.fw, self.fh)
    }

    fn draw(&self, cr: &cairo::Context, t: f64) {
        let idx = self.frame_index(t);
        let fx = (idx % self.cols) as f64 * self.fw;
        let fy = (idx / self.cols) as f64 * self.fh;
        let dx = -self.anchor.0;
        let dy = -self.anchor.1;

        cr.save().unwrap();
        // Clip to one frame, then blit the sheet shifted so that frame lands at
        // the local origin.
        cr.rectangle(dx, dy, self.fw, self.fh);
        cr.clip();
        cr.set_source_surface(&self.sheet, dx - fx, dy - fy).unwrap();
        if self.pixelated {
            // Keep pixels crisp under the squash scale.
            cr.source().set_filter(cairo::Filter::Nearest);
        }
        let _ = cr.paint();
        cr.restore().unwrap();
    }
}
