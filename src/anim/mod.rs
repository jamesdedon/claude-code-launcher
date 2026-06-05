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

pub mod blossoms;
pub mod content;
pub mod little_guy;
pub mod motion;
pub mod racer;
pub mod stage;

pub use content::{Content, PanParams, Play, SpriteContent};
pub use little_guy::VectorGuy;
pub use motion::{Approach, Lifecycle, Spring, Staged};
pub use stage::{Anchor, Dir, Stage};

use serde::Deserialize;
use std::path::{Path, PathBuf};

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

/// Parse a config `cycle` string into a [`Lifecycle`] with default timings.
fn parse_cycle(s: &str) -> Option<Lifecycle> {
    match s.to_ascii_lowercase().as_str() {
        "once" => Some(Lifecycle::Once { hold: 1.3 }),
        "loop" => Some(Lifecycle::Loop { hold: 2.5, gap: 1.0 }),
        "hold" => Some(Lifecycle::Hold),
        _ => None,
    }
}

/// When an animation plays: on launcher open (`Spawn`) or on submit (`Submit`).
/// An animation declares this in its manifest / built-in default; the config can
/// override it. The host routes each animation to its slot by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Spawn,
    Submit,
}

fn parse_trigger(s: &str) -> Option<Trigger> {
    match s.to_ascii_lowercase().as_str() {
        "spawn" | "open" => Some(Trigger::Spawn),
        "submit" | "enter" => Some(Trigger::Submit),
        _ => None,
    }
}

/// Coerce any lifecycle to a single run, preserving its hold. Submit animations
/// are forced through this: the window close waits on the exit, so a `loop` or
/// `hold` would hang it open forever.
fn as_single_run(lc: Lifecycle) -> Lifecycle {
    match lc {
        Lifecycle::Once { hold } | Lifecycle::Loop { hold, .. } => Lifecycle::Once { hold },
        Lifecycle::Hold => Lifecycle::Once { hold: 1.0 },
    }
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

/// One animation *section* — the body of a `[spawn]` or `[submit]` table (or a
/// whole flat manifest). The art is either a built-in `figure` (drawn in code)
/// or a PNG `sheet`; the rest are motion fields. All timings live here, so an
/// animation file is self-contained.
#[derive(Debug, Clone, Deserialize, Default)]
struct SpriteManifest {
    /// A built-in figure by name: "racer", "f1", "spinner", "little_guy".
    #[serde(default)]
    figure: Option<String>,
    /// A PNG sprite sheet (relative to the manifest). Use instead of `figure`.
    #[serde(default)]
    sheet: Option<String>,
    #[serde(default = "default_frames")]
    frames: usize,
    /// A vertical pan over a tall figure (see [`PanParams`]); presence switches
    /// the section to pan motion.
    #[serde(default)]
    pan: Option<PanManifest>,
    #[serde(default)]
    columns: Option<usize>,
    #[serde(default)]
    frame_width: Option<f64>,
    #[serde(default)]
    frame_height: Option<f64>,
    #[serde(default = "default_fps")]
    fps: f64,
    #[serde(default)]
    anchor: Option<FrameAnchor>,
    #[serde(default = "default_play")]
    play: String,
    #[serde(default)]
    pixelated: bool,
    // Motion (all optional; config block overrides these).
    #[serde(default)]
    cycle: Option<String>,
    /// "spawn" | "submit" — when this animation plays. Config `trigger` overrides.
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    enter_from: Option<DirSpec>,
    #[serde(default)]
    exit_to: Option<DirSpec>,
    #[serde(default)]
    rest: Option<[f64; 4]>, // nx, ny, dx, dy
    #[serde(default)]
    fit: Option<f64>,
    #[serde(default)]
    min_card: Option<i32>,
    #[serde(default)]
    stiffness: Option<f64>,
    #[serde(default)]
    damping: Option<f64>,
    #[serde(default)]
    squash: Option<f64>,
}

fn default_fps() -> f64 {
    12.0
}
fn default_play() -> String {
    "loop".to_string()
}
fn default_frames() -> usize {
    1
}

/// The `[pan]` table of a section: a vertical camera pan over a tall figure.
/// Defaults reproduce the racer. Measurements are frame pixels / seconds.
#[derive(Debug, Clone, Copy, Deserialize)]
struct PanManifest {
    #[serde(default = "pan_default_slice")]
    slice: f64,
    #[serde(default = "pan_default_focus")]
    focus: f64,
    #[serde(default = "pan_default_reveal")]
    reveal: f64,
    #[serde(default = "pan_default_hold")]
    hold: f64,
    #[serde(default = "pan_default_exit")]
    exit: f64,
}

fn pan_default_slice() -> f64 {
    72.0
}
fn pan_default_focus() -> f64 {
    -8.0
}
fn pan_default_reveal() -> f64 {
    2.6
}
fn pan_default_hold() -> f64 {
    0.7
}
fn pan_default_exit() -> f64 {
    1.2
}

impl PanManifest {
    fn to_params(self) -> PanParams {
        PanParams {
            slice: self.slice,
            focus: self.focus,
            reveal: self.reveal,
            hold: self.hold,
            exit: self.exit,
        }
    }
}

/// The frame anchor: a named position ("center"/"bottom"/"top") or pixels.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum FrameAnchor {
    Named(String),
    Xy { x: f64, y: f64 },
}

fn parse_play(s: &str) -> Play {
    match s.to_ascii_lowercase().as_str() {
        "once" => Play::Once,
        "pingpong" | "ping-pong" => Play::PingPong,
        _ => Play::Loop,
    }
}

/// Expand a leading `~/` to `$HOME`.
pub fn expand_tilde(p: &str) -> PathBuf {
    p.strip_prefix("~/")
        .and_then(|rest| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(rest)))
        .unwrap_or_else(|| PathBuf::from(p))
}

/// Default motion for a section before the manifest overrides it.
struct Defaults {
    rest: Anchor,
    enter: Dir,
    exit: Dir,
    cycle: Lifecycle,
    fit: Option<f64>,
    min_card: i32,
    stiffness: f64,
    damping: f64,
    squash: f64,
    pan: Option<PanParams>,
}

impl Defaults {
    fn sheet() -> Self {
        Self {
            rest: Anchor::new(0.5, 0.5, 0.0, 0.0),
            enter: Dir::Deg(180.0),
            exit: Dir::Deg(0.0),
            cycle: Lifecycle::Loop { hold: 2.5, gap: 1.0 },
            fit: None,
            min_card: 0,
            stiffness: 200.0,
            damping: 16.0,
            squash: 0.02,
            pan: None,
        }
    }
    fn racer() -> Self {
        Self {
            rest: Anchor::new(0.75, 0.5, 0.0, 0.0),
            enter: Dir::Xy { dx: 0.0, dy: 0.0 }, // the pan does the motion
            exit: Dir::Xy { dx: 0.0, dy: 0.0 },
            cycle: Lifecycle::Hold, // replaced by a pan-timed Once
            fit: Some(0.92),
            min_card: 120,
            stiffness: 220.0,
            damping: 26.0,
            squash: 0.0,
            pan: Some(PanParams { slice: 72.0, focus: -8.0, reveal: 2.6, hold: 0.7, exit: 1.2 }),
        }
    }
    fn f1() -> Self {
        Self {
            rest: Anchor::new(0.25, 0.62, 0.0, 0.0),
            enter: Dir::Deg(180.0),
            exit: Dir::Deg(0.0),
            cycle: Lifecycle::Once { hold: 1.0 },
            fit: Some(0.85),
            min_card: 96,
            stiffness: 240.0,
            damping: 11.0,
            squash: 0.02,
            pan: None,
        }
    }
    fn spinner() -> Self {
        Self {
            rest: Anchor::new(0.5, 0.5, 0.0, 0.0),
            enter: Dir::Deg(0.0),
            exit: Dir::Deg(0.0),
            cycle: Lifecycle::Loop { hold: 2.5, gap: 1.0 },
            fit: Some(0.8),
            min_card: 96,
            stiffness: 180.0,
            damping: 16.0,
            squash: 0.02,
            pan: None,
        }
    }
    fn little_guy() -> Self {
        Self {
            rest: Anchor::new(1.0, 1.0, -58.0, 37.0),
            enter: Dir::Deg(270.0),
            exit: Dir::Deg(270.0),
            cycle: Lifecycle::Once { hold: 1.4 },
            fit: None,
            min_card: 0,
            stiffness: 220.0,
            damping: 18.0,
            squash: 0.028,
            pan: None,
        }
    }
}

/// The pan params for a section: manifest `[pan]` over the figure's default.
fn resolve_pan(m: &SpriteManifest, d: &Defaults) -> Option<PanParams> {
    m.pan.map(PanManifest::to_params).or(d.pan)
}

/// Wrap any content with manifest-over-default motion into a drivable animation.
/// A `pan` makes the lifecycle a single run timed to the pan; a `submit` slot
/// also forces a single run so the window can close.
fn apply_motion<C: Content + 'static>(
    content: C,
    m: &SpriteManifest,
    d: &Defaults,
    pan: Option<PanParams>,
    slot: Trigger,
) -> Box<dyn Animation> {
    let rest = m
        .rest
        .map(|r| Anchor::new(r[0], r[1], r[2], r[3]))
        .unwrap_or(d.rest);
    let enter = m.enter_from.as_ref().map(DirSpec::to_dir).unwrap_or(d.enter);
    let exit = m.exit_to.as_ref().map(DirSpec::to_dir).unwrap_or(d.exit);
    let cyc = if let Some(p) = pan {
        Lifecycle::Once { hold: p.total() }
    } else {
        let lc = m.cycle.as_deref().and_then(parse_cycle).unwrap_or(d.cycle);
        if slot == Trigger::Submit { as_single_run(lc) } else { lc }
    };
    let spring = Spring::new(m.stiffness.unwrap_or(d.stiffness), m.damping.unwrap_or(d.damping));
    let mut staged = Staged::new(
        content,
        Approach { rest, enter_from: enter, exit_to: exit },
        spring,
        m.squash.unwrap_or(d.squash),
    )
    .with_lifecycle(cyc);
    if let Some(f) = m.fit.or(d.fit) {
        staged = staged.with_fit(f);
    }
    let mc = m.min_card.unwrap_or(d.min_card);
    if mc > 0 {
        staged = staged.with_min_card(mc);
    }
    Box::new(staged)
}

/// Load a PNG sheet into sprite content using the section's frame geometry.
fn load_sheet_content(
    sheet: &str,
    base_dir: &Path,
    m: &SpriteManifest,
) -> Result<SpriteContent, String> {
    let sheet_rel = expand_tilde(sheet);
    let sheet_path = if sheet_rel.is_absolute() {
        sheet_rel
    } else {
        base_dir.join(sheet_rel)
    };
    let surface = SpriteContent::load_sheet(&sheet_path)?;
    let (sw, sh) = (surface.width() as f64, surface.height() as f64);
    let cols = m.columns.unwrap_or(m.frames).max(1);
    let rows = m.frames.div_ceil(cols).max(1);
    let fw = m.frame_width.unwrap_or(sw / cols as f64);
    let fh = m.frame_height.unwrap_or(sh / rows as f64);
    let anchor = match m.anchor.clone().unwrap_or(FrameAnchor::Named("center".to_string())) {
        FrameAnchor::Xy { x, y } => (x, y),
        FrameAnchor::Named(n) => match n.to_ascii_lowercase().as_str() {
            "bottom" | "bottom-center" => (fw / 2.0, fh),
            "top" | "top-center" => (fw / 2.0, 0.0),
            _ => (fw / 2.0, fh / 2.0),
        },
    };
    Ok(
        SpriteContent::new(surface, fw, fh, cols, m.frames, m.fps, parse_play(&m.play), anchor)
            .pixelated(m.pixelated),
    )
}

/// Build one section (the body of a `[spawn]`/`[submit]` table) into a
/// ready-to-drive animation. Art is a built-in `figure` or a PNG `sheet`.
fn build_section(
    m: &SpriteManifest,
    base_dir: &Path,
    slot: Trigger,
) -> Result<Box<dyn Animation>, String> {
    // The little guy is the only vector built-in figure; it can't pan.
    if m.figure.as_deref() == Some("little_guy") {
        return Ok(apply_motion(VectorGuy::new(), m, &Defaults::little_guy(), None, slot));
    }
    let (content, d) = if let Some(fig) = &m.figure {
        match fig.as_str() {
            "racer" | "speed_racer" => (racer::racer_sheet(), Defaults::racer()),
            "f1" | "f1_car" => (SpriteContent::f1_car(), Defaults::f1()),
            "spinner" => (SpriteContent::spinner(), Defaults::spinner()),
            "none" | "off" => return Ok(Box::new(NoAnim)),
            other => return Err(format!("unknown figure '{other}'")),
        }
    } else if let Some(sheet) = &m.sheet {
        (load_sheet_content(sheet, base_dir, m)?, Defaults::sheet())
    } else {
        return Err("animation section needs a `figure` or `sheet`".to_string());
    };
    let pan = resolve_pan(m, &d);
    let content = match pan {
        Some(p) => content.with_pan(p),
        None => content,
    };
    Ok(apply_motion(content, m, &d, pan, slot))
}

/// Load an animation **pack** file into the (spawn, submit) slots. A pack has
/// optional `[spawn]` / `[submit]` tables, each a self-contained section; a flat
/// manifest is treated as one section routed by its `trigger` (default spawn).
/// Missing or broken sections fall back to [`NoAnim`].
pub fn load_pack(file: &Path) -> (Box<dyn Animation>, Box<dyn Animation>) {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("animation: read {}: {e}", file.display());
            return (Box::new(NoAnim), Box::new(NoAnim));
        }
    };
    let base = file.parent().unwrap_or_else(|| Path::new("."));
    let section = |m: &SpriteManifest, slot| {
        build_section(m, base, slot).unwrap_or_else(|e| {
            eprintln!("animation: {}: {e}", file.display());
            Box::new(NoAnim) as Box<dyn Animation>
        })
    };

    #[derive(Deserialize)]
    struct Pack {
        #[serde(default)]
        spawn: Option<SpriteManifest>,
        #[serde(default)]
        submit: Option<SpriteManifest>,
    }
    let bundle = toml::from_str::<Pack>(&text)
        .ok()
        .filter(|p| p.spawn.is_some() || p.submit.is_some());
    if let Some(pack) = bundle {
        let open = pack
            .spawn
            .map(|m| section(&m, Trigger::Spawn))
            .unwrap_or_else(|| Box::new(NoAnim));
        let submit = pack
            .submit
            .map(|m| section(&m, Trigger::Submit))
            .unwrap_or_else(|| Box::new(NoAnim));
        return (open, submit);
    }
    // Flat single manifest → route by its trigger.
    match toml::from_str::<SpriteManifest>(&text) {
        Ok(m) => {
            let trig = m.trigger.as_deref().and_then(parse_trigger).unwrap_or(Trigger::Spawn);
            let anim = section(&m, trig);
            match trig {
                Trigger::Submit => (Box::new(NoAnim), anim),
                Trigger::Spawn => (anim, Box::new(NoAnim)),
            }
        }
        Err(e) => {
            eprintln!("animation: parse {}: {e}", file.display());
            (Box::new(NoAnim), Box::new(NoAnim))
        }
    }
}

/// The default animation packs, embedded from `assets/anims/` at compile time.
/// The code never enumerates or names them — adding a built-in means dropping a
/// file into that folder, nothing here changes.
static BUILTIN_ANIMS: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/assets/anims");

/// Seed the bundled default packs into `dir` (creating it), copying only files
/// that are missing — so a user's edits and deletions stick across runs.
pub fn seed_builtin_packs(dir: &Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("animation: create {}: {e}", dir.display());
        return;
    }
    for file in BUILTIN_ANIMS.files() {
        let Some(name) = file.path().file_name() else {
            continue;
        };
        let path = dir.join(name);
        if path.exists() {
            continue;
        }
        if let Err(e) = std::fs::write(&path, file.contents()) {
            eprintln!("animation: seed {}: {e}", path.display());
        }
    }
}
