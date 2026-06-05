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

/// Whether an animation plays a single cycle, loops, or holds.
#[derive(Debug, Clone, Copy)]
pub enum Lifecycle {
    /// Enter, then idle forever until `hide()` is called externally.
    Hold,
    /// Enter, idle `hold` seconds, exit, then stay done. A single run.
    Once { hold: f64 },
    /// Enter, idle `hold` s, exit, wait `gap` s, then repeat — forever.
    Loop { hold: f64, gap: f64 },
}

/// Composes a [`Content`] (appearance) with spring-driven directional motion.
/// This is where every content — sprite or vector — gets identical placement,
/// directional travel, and squash-and-stretch.
pub struct Staged<C: Content> {
    content: C,
    approach: Approach,
    spring: Spring,
    squash_k: f64,
    lifecycle: Lifecycle,
    /// If set, scale the content so its natural height ≈ this fraction of the
    /// stage height. `None` leaves it unscaled (e.g. the guy, who overflows and
    /// peeks).
    fit: Option<f64>,
    /// The minimum card height (px) this animation wants in order to fit.
    min_card: i32,
    target: f64,
    t: f64,
    phase: Phase,
    since: f64, // seconds spent in the current settled phase
}

impl<C: Content> Staged<C> {
    pub fn new(content: C, approach: Approach, spring: Spring, squash_k: f64) -> Self {
        Self {
            content,
            approach,
            spring,
            squash_k,
            lifecycle: Lifecycle::Hold,
            fit: None,
            min_card: 0,
            target: 0.0,
            t: 0.0,
            phase: Phase::Done,
            since: 0.0,
        }
    }

    pub fn with_lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Scale the content to fit the card: its natural height becomes ~`frac` of
    /// the stage height.
    pub fn with_fit(mut self, frac: f64) -> Self {
        self.fit = Some(frac);
        self
    }

    /// Request a minimum card height (px) so the content has room.
    pub fn with_min_card(mut self, px: i32) -> Self {
        self.min_card = px;
        self
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
        let prev = self.phase;
        self.spring.step(self.target, dt);
        if self.spring.settled(self.target) {
            self.phase = if self.target >= 0.5 {
                Phase::Idle
            } else {
                Phase::Done
            };
        }
        // Time spent in the current settled phase, reset on transitions.
        self.since = if self.phase == prev { self.since + dt } else { 0.0 };

        // Self-driving lifecycle: auto-exit after a hold, and (for Loop)
        // re-enter after a gap.
        match self.lifecycle {
            Lifecycle::Hold => {}
            Lifecycle::Once { hold } => {
                if self.phase == Phase::Idle && self.since >= hold {
                    self.hide();
                }
            }
            Lifecycle::Loop { hold, gap } => {
                if self.phase == Phase::Idle && self.since >= hold {
                    self.hide();
                } else if self.phase == Phase::Done && self.since >= gap {
                    self.show();
                }
            }
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
        // Scale-to-card: shrink the content to fit the prompt height.
        if let Some(frac) = self.fit {
            let (_, nh) = self.content.natural_size();
            if nh > 0.0 {
                let s = (frac * stage.h / nh).clamp(0.05, 4.0);
                cr.scale(s, s);
            }
        }
        self.content.draw(cr, self.t);
        cr.restore().unwrap();
    }

    fn is_exit_done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    fn min_card_height(&self) -> i32 {
        self.min_card
    }

    fn is_finished(&self) -> bool {
        match self.lifecycle {
            // A looping animation is never finished — its Done is transient.
            Lifecycle::Loop { .. } => false,
            _ => matches!(self.phase, Phase::Done),
        }
    }
}
