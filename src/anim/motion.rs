//! Motion: a spring + a directional approach, composed with any [`Content`]
//! into a drivable [`Animation`].

use super::content::Content;
use super::stage::{Anchor, Dir, Stage};
use super::{Animation, Phase};

/// A 1-D spring integrated against real time. `pos` runs 0 (hidden) → 1
/// (resting) and may overshoot past 1 — that overshoot *is* the pop.
pub struct Spring {
    pub pos: f64,
    pub vel: f64,
    pub stiffness: f64,
    pub damping: f64,
}

impl Spring {
    pub fn new(stiffness: f64, damping: f64) -> Self {
        Self {
            pos: 0.0,
            vel: 0.0,
            stiffness,
            damping,
        }
    }

    pub fn step(&mut self, target: f64, dt: f64) {
        let force = self.stiffness * (target - self.pos) - self.damping * self.vel;
        self.vel += force * dt;
        self.pos += self.vel * dt;
    }

    pub fn settled(&self, target: f64) -> bool {
        (self.pos - target).abs() < 0.001 && self.vel.abs() < 0.001
    }
}

/// Where the animation rests, and which way it enters / exits. Enter and exit
/// are independent, so it can pop up from below and slink off to the side.
#[derive(Debug, Clone, Copy)]
pub struct Approach {
    pub rest: Anchor,
    pub enter_from: Dir,
    pub exit_to: Dir,
}

/// Composes a [`Content`] (appearance) with spring-driven directional motion.
/// This is where every content — sprite or vector — gets identical placement,
/// directional travel, and squash-and-stretch.
pub struct Staged<C: Content> {
    content: C,
    approach: Approach,
    spring: Spring,
    squash_k: f64,
    target: f64,
    t: f64,
    phase: Phase,
}

impl<C: Content> Staged<C> {
    pub fn new(content: C, approach: Approach, spring: Spring, squash_k: f64) -> Self {
        Self {
            content,
            approach,
            spring,
            squash_k,
            target: 0.0,
            t: 0.0,
            phase: Phase::Done,
        }
    }
}

impl<C: Content> Animation for Staged<C> {
    fn show(&mut self) {
        self.target = 1.0;
        if matches!(self.phase, Phase::Done | Phase::Exiting) {
            self.phase = Phase::Entering;
        }
    }

    fn hide(&mut self) {
        self.target = 0.0;
        if matches!(self.phase, Phase::Idle | Phase::Entering) {
            self.phase = Phase::Exiting;
        }
    }

    fn tick(&mut self, dt: f64) -> Phase {
        self.t += dt; // unbounded: drives frames / idle forever
        self.spring.step(self.target, dt);
        if self.spring.settled(self.target) {
            self.phase = if self.target >= 0.5 {
                Phase::Idle
            } else {
                Phase::Done
            };
        }
        self.phase
    }

    fn draw(&self, cr: &gtk4::cairo::Context, stage: &Stage) {
        // Fully gone: nothing to paint.
        if matches!(self.phase, Phase::Done) && self.spring.pos < 0.002 {
            return;
        }

        let pos = self.spring.pos;
        let (rx, ry) = self.approach.rest.resolve(stage);
        let dir = if self.target >= 0.5 {
            self.approach.enter_from
        } else {
            self.approach.exit_to
        };
        let (ox, oy) = dir.offset(stage);

        // Interpolate from off-stage (pos 0) to rest (pos 1) along the dir.
        let x = rx + (1.0 - pos) * ox;
        let y = ry + (1.0 - pos) * oy;

        // Squash-and-stretch along the *travel axis* — so it reads right no
        // matter which direction the motion is. Reduces to vertical stretch for
        // a bottom entrance, horizontal for a side entrance, etc.
        let stretch = (self.spring.vel * self.squash_k).clamp(-0.30, 0.30);
        let axis = oy.atan2(ox);

        cr.save().unwrap();
        cr.translate(x, y);
        cr.rotate(axis);
        cr.scale(1.0 + stretch, 1.0 - 0.5 * stretch);
        cr.rotate(-axis);
        self.content.draw(cr, self.t);
        cr.restore().unwrap();
    }

    fn is_exit_done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }
}
