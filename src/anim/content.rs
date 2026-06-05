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
        let (art_w, art_h) = (38.0, 18.0);
        let fw = (art_w * p) as i32; // 152
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
/// device pixels per art pixel; `frame` drives the wheels, the dirt spray, and
/// the revving shake. Original blue-livery art (inspired-by, not copied).
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

    // Palette.
    let blue = (0.13, 0.33, 0.78);
    let blue_dk = (0.08, 0.20, 0.52);
    let red = (0.85, 0.15, 0.15);
    let white = (0.95, 0.95, 0.97);
    let tyre = (0.10, 0.10, 0.12);
    let brake = (0.80, 0.12, 0.12);
    let hub = (0.65, 0.65, 0.68);
    let wing = (0.13, 0.13, 0.16);
    let set = |c: (f64, f64, f64)| cr.set_source_rgba(c.0, c.1, c.2, 1.0);

    // Ground contact shadow.
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.18);
    px(7.0, 14.6, 24.0, 1.2);

    // Dirt kicking up behind the rear wheel — specks streaming back-left + up.
    for k in 0..5usize {
        let phase = ((frame + k * 2) % 8) as f64;
        let dx = 8.0 - phase * 0.8 - k as f64 * 0.3;
        let dy = 13.5 - phase * 0.6 - (k as f64 * 0.2);
        let sz = (1.5 - phase * 0.13).max(0.4);
        if k % 2 == 0 {
            set((0.42, 0.30, 0.17));
        } else {
            set((0.58, 0.45, 0.28));
        }
        px(dx, dy, sz, sz);
    }

    // Wheels (steady; the body shakes around them) — tyre, red disc, hub.
    for &cx in &[11.0, 28.0] {
        set(tyre);
        px(cx - 2.5, 8.0, 5.0, 7.0);
        px(cx - 3.5, 9.0, 7.0, 5.0);
        set(brake);
        px(cx - 1.5, 10.0, 3.0, 3.0);
        set(hub);
        px(cx - 0.5, 11.0, 1.0, 1.0);
        // Spinning highlight on the tyre.
        set((0.40, 0.40, 0.44));
        match frame % 4 {
            0 => px(cx - 0.5, 8.3, 1.0, 1.4),
            1 => px(cx + 1.6, 11.0, 1.4, 1.0),
            2 => px(cx - 0.5, 13.3, 1.0, 1.4),
            _ => px(cx - 3.0, 11.0, 1.4, 1.0),
        }
    }

    // Rear wing.
    set(wing);
    px(2.0, 3.5 + vy, 4.0, 1.2); // top plane
    px(2.5, 3.5 + vy, 1.5, 5.0); // endplate
    set(red);
    px(2.0, 3.5 + vy, 4.0, 0.4); // red tip

    // Body: rear structure, main tub, side stripe, floor, nose.
    set(blue_dk);
    px(5.0, 7.0 + vy, 4.0, 4.0);
    set(blue);
    px(7.0, 8.0 + vy, 22.0, 3.0);
    set(red);
    px(9.0, 9.0 + vy, 18.0, 1.0); // side stripe
    set(white);
    px(7.0, 8.0 + vy, 3.0, 1.0); // front-of-tub flash
    set(blue_dk);
    px(9.0, 10.5 + vy, 18.0, 1.2); // floor
    set(blue);
    px(29.0, 8.5 + vy, 5.0, 2.0); // nose
    px(33.5, 9.0 + vy, 2.5, 1.0); // nose tip
    set(red);
    px(35.5, 9.2 + vy, 1.0, 0.8); // nose tip accent

    // Cockpit + driver + helmet.
    set(blue);
    px(15.0, 6.5 + vy, 6.0, 2.0); // cockpit base
    px(17.0, 6.0 + vy, 3.0, 1.5); // torso
    set(wing);
    px(15.5, 5.0 + vy, 1.0, 2.5); // halo / roll hoop
    set((0.10, 0.25, 0.70));
    px(18.5, 5.0 + vy, 3.0, 2.2); // helmet
    set(red);
    px(18.5, 5.0 + vy, 3.0, 0.6); // helmet top
    set(white);
    px(18.5, 5.7 + vy, 3.0, 0.5); // helmet stripe
    set((0.05, 0.05, 0.08));
    px(20.0, 5.8 + vy, 1.3, 0.9); // visor

    // Front wing.
    set(wing);
    px(30.0, 11.0 + vy, 5.0, 1.0);
    px(34.0, 10.0 + vy, 1.2, 2.0); // endplate
    set(red);
    px(30.0, 11.0 + vy, 5.0, 0.4);
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
