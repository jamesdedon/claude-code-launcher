//! The standardized frame geometry every animation is authored against.

/// The card's interior, in device pixels. Passed to [`super::Animation::draw`]
/// each frame. Animations author against this so they adapt to any card size.
#[derive(Debug, Clone, Copy)]
pub struct Stage {
    pub w: f64,
    pub h: f64,
}

/// A resting point in the stage: a normalized fraction (`nx`, `ny`) of the
/// stage plus a pixel nudge (`dx`, `dy`). So `(1.0, 1.0, -58.0, 37.0)` reads as
/// "bottom-right corner, 58px in from the right, 37px below the bottom edge".
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub nx: f64,
    pub ny: f64,
    pub dx: f64,
    pub dy: f64,
}

impl Anchor {
    pub fn new(nx: f64, ny: f64, dx: f64, dy: f64) -> Self {
        Self { nx, ny, dx, dy }
    }

    pub fn resolve(&self, s: &Stage) -> (f64, f64) {
        (self.nx * s.w + self.dx, self.ny * s.h + self.dy)
    }
}

/// The direction an animation enters from / exits to — the heart of "move in
/// any direction". Either an **angle in degrees** (host computes a distance
/// that clears the stage) or a raw **scalar offset** for precise / partial
/// motion.
#[derive(Debug, Clone, Copy)]
pub enum Dir {
    /// Degrees, CCW: 0 = right, 90 = up, 180 = left, 270 = down.
    Deg(f64),
    /// Explicit offset vector in pixels.
    Xy { dx: f64, dy: f64 },
}

impl Dir {
    /// The outward offset vector toward the off-stage start (for entry) or end
    /// (for exit). For [`Dir::Deg`], the magnitude is sized so the content
    /// clears the stage along that axis.
    pub fn offset(&self, s: &Stage) -> (f64, f64) {
        match self {
            Dir::Xy { dx, dy } => (*dx, *dy),
            Dir::Deg(deg) => {
                let r = deg.to_radians();
                let (ux, uy) = (r.cos(), -r.sin()); // y is down on screen
                let dist = ux.abs() * s.w + uy.abs() * s.h + 40.0;
                (ux * dist, uy * dist)
            }
        }
    }
}
