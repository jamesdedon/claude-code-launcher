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
        let _ = cr.paint();
        cr.restore().unwrap();
    }
}
