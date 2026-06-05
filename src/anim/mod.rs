//! Pluggable, interchangeable animations for the prompt card.
//!
//! Design in three layers, so neither *direction* nor *render-style* is baked
//! into the interface:
//!
//! - [`Content`] — appearance only. Sprites ([`SpriteContent`]) and vectors
//!   (e.g. [`VectorGuy`]) implement the *same* trait, so neither is privileged.
//! - [`Approach`] + [`Spring`] — motion. Direction is a vector ([`Dir`]),
//!   expressible as degrees *or* raw scalars, with independent enter/exit, so
//!   every direction works for every content.
//! - [`Staged<C>`] — composes a `Content` with motion and implements
//!   [`Animation`], the lifecycle the launcher drives.
//!
//! Selection is by a config string via [`build`]: change the string, change
//! the guy.

pub mod content;
pub mod little_guy;
pub mod motion;
pub mod stage;

pub use content::{Content, Play, SpriteContent};
pub use little_guy::VectorGuy;
pub use motion::{Approach, Lifecycle, Spring, Staged};
pub use stage::{Anchor, Dir, Stage};

use serde::Deserialize;

/// Where an animation is in its lifecycle. The `Idle` phase is open-ended — it
/// ticks forever until `hide()` — so step count is unbounded by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Entering,
    Idle,
    Exiting,
    Done,
}

/// The interface the launcher drives. Object-safe so animations are swappable
/// behind a `Box<dyn Animation>`.
pub trait Animation {
    /// Begin the entrance (pop-in).
    fn show(&mut self);
    /// Begin the exit (slink-out).
    fn hide(&mut self);
    /// Advance internal state by `dt` seconds; report the resulting phase.
    fn tick(&mut self, dt: f64) -> Phase;
    /// Render into the card's interior (`stage`).
    fn draw(&self, cr: &gtk4::cairo::Context, stage: &Stage);
    /// True once the exit has fully played out — for deferring window close.
    fn is_exit_done(&self) -> bool;

    /// True once the animation is done and will NOT restart on its own, so a
    /// host can stop ticking. Defaults to [`Animation::is_exit_done`]; looping
    /// animations override this to stay live.
    fn is_finished(&self) -> bool {
        self.is_exit_done()
    }

    /// The minimum card height (px) this animation wants to fit; the host can
    /// raise the prompt to at least this. Defaults to 0 (no requirement).
    fn min_card_height(&self) -> i32 {
        0
    }
}

/// Config-facing selection. `name` maps to a registered animation; the optional
/// direction overrides accept **degrees**, a **named edge**, or raw **scalars**.
///
/// ```toml
/// [animation]
/// name = "little_guy"
/// enter_from = 270            # degrees (up from below)
/// exit_to    = { dx = -180, dy = 0 }   # scalars (slink left)
/// # enter_from = "bottom"     # named sugar -> 270
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct AnimSpec {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub enter_from: Option<DirSpec>,
    #[serde(default)]
    pub exit_to: Option<DirSpec>,
    /// "once" | "loop" | "hold". Overrides the animation's default lifecycle.
    #[serde(default)]
    pub cycle: Option<String>,
}

impl Default for AnimSpec {
    fn default() -> Self {
        Self {
            name: default_name(),
            enter_from: None,
            exit_to: None,
            cycle: None,
        }
    }
}

/// Parse a config `cycle` string into a [`Lifecycle`] with default timings.
fn parse_cycle(s: &str) -> Option<Lifecycle> {
    match s.to_ascii_lowercase().as_str() {
        "once" => Some(Lifecycle::Once { hold: 1.3 }),
        "loop" => Some(Lifecycle::Loop { hold: 2.5, gap: 1.0 }),
        "hold" => Some(Lifecycle::Hold),
        _ => None,
    }
}

fn default_name() -> String {
    "little_guy".to_string()
}

/// A direction as it appears in config: a bare number is **degrees**, a string
/// is a **named edge**, and a `{ dx, dy }` table is a raw **scalar** offset.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DirSpec {
    Degrees(f64),
    Named(String),
    Offset { dx: f64, dy: f64 },
}

impl DirSpec {
    pub fn to_dir(&self) -> Dir {
        match self {
            DirSpec::Degrees(d) => Dir::Deg(*d),
            DirSpec::Offset { dx, dy } => Dir::Xy { dx: *dx, dy: *dy },
            DirSpec::Named(s) => Dir::Deg(named_degrees(s)),
        }
    }
}

/// Convention: 0 = right, 90 = up, 180 = left, 270 = down (CCW; screen y down).
fn named_degrees(s: &str) -> f64 {
    match s.to_ascii_lowercase().as_str() {
        "right" => 0.0,
        "top" | "up" => 90.0,
        "left" => 180.0,
        "bottom" | "down" => 270.0,
        "top-right" | "upper-right" => 45.0,
        "top-left" | "upper-left" => 135.0,
        "bottom-left" | "lower-left" => 225.0,
        "bottom-right" | "lower-right" => 315.0,
        _ => 270.0,
    }
}

/// A no-op animation: draws nothing and is always "done", so hosting code can
/// treat "no animation" uniformly (close paths never wait on it).
pub struct NoAnim;

impl Animation for NoAnim {
    fn show(&mut self) {}
    fn hide(&mut self) {}
    fn tick(&mut self, _dt: f64) -> Phase {
        Phase::Done
    }
    fn draw(&self, _cr: &gtk4::cairo::Context, _stage: &Stage) {}
    fn is_exit_done(&self) -> bool {
        true
    }
}

/// The registry: resolve a [`AnimSpec`] into a ready-to-drive animation.
/// Unknown names fall back to the little guy.
pub fn build(spec: &AnimSpec) -> Box<dyn Animation> {
    let enter = |fallback: Dir| spec.enter_from.as_ref().map(DirSpec::to_dir).unwrap_or(fallback);
    let exit = |fallback: Dir| spec.exit_to.as_ref().map(DirSpec::to_dir).unwrap_or(fallback);
    // Config `cycle` overrides the per-animation default lifecycle.
    let lifecycle = |default: Lifecycle| {
        spec.cycle.as_deref().and_then(parse_cycle).unwrap_or(default)
    };

    match spec.name.as_str() {
        "none" | "off" => Box::new(NoAnim),
        // A procedurally-generated sprite, proving sprites run through the exact
        // same pipeline as the vector guy. Drifts in from the right by default.
        "spinner" => {
            let approach = Approach {
                rest: Anchor::new(0.5, 0.5, 0.0, 0.0),
                enter_from: enter(Dir::Deg(0.0)),
                exit_to: exit(Dir::Deg(0.0)),
            };
            Box::new(
                Staged::new(SpriteContent::spinner(), approach, Spring::new(180.0, 16.0), 0.02)
                    .with_lifecycle(lifecycle(Lifecycle::Loop { hold: 2.5, gap: 1.0 }))
                    .with_fit(0.8)
                    .with_min_card(96),
            )
        }
        // An 8-bit F1 car: pops in from the left fighting for grip (under-damped
        // spring), then takes off to the right.
        "f1" | "f1_car" => {
            let approach = Approach {
                rest: Anchor::new(0.25, 0.62, 0.0, 0.0), // rest in the left quarter
                enter_from: enter(Dir::Deg(180.0)),      // from the left
                exit_to: exit(Dir::Deg(0.0)),            // off to the right
            };
            Box::new(
                Staged::new(
                    SpriteContent::f1_car(),
                    approach,
                    Spring::new(240.0, 11.0), // under-damped: it fights for speed
                    0.02,
                )
                .with_lifecycle(lifecycle(Lifecycle::Once { hold: 1.0 })) // a single run
                .with_fit(0.85)
                .with_min_card(96),
            )
        }
        // Default: the little guy peeking up over the bottom edge.
        _ => {
            let approach = Approach {
                rest: Anchor::new(1.0, 1.0, -58.0, 37.0),
                enter_from: enter(Dir::Deg(270.0)),
                exit_to: exit(Dir::Deg(270.0)),
            };
            Box::new(
                Staged::new(VectorGuy::new(), approach, Spring::new(220.0, 18.0), 0.028)
                    .with_lifecycle(lifecycle(Lifecycle::Once { hold: 1.4 })),
            )
        }
    }
}
