// Standalone playground for the "little guy" animation.
//
//   cargo run --example little_guy
//
// Draws a procedural character (no sprite asset yet) that lives *inside* the
// launcher's rounded prompt card. He pops up with a spring overshoot, idles
// with a soft bob and the odd blink, then slinks back down — squashing and
// stretching off his vertical velocity. The card sets `overflow: hidden`, so
// GTK masks him to its rounded border: he rises from within the card and the
// edges clip him, rather than floating in free space.
//
// The show/hide target loops here so you can watch and tune the feel; in the
// real launcher the trigger would be window present/close instead of a timer.
//
// Everything is pure gtk4 + cairo + a hand-rolled spring, so this adds no new
// dependencies over the launcher's existing Cargo.toml.

use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, Align, Application, ApplicationWindow, Box as GtkBox, CssProvider, DrawingArea,
    EventControllerKey, Label, Orientation, Overflow, Overlay, WindowHandle,
};
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "dev.dedon.ClaudeCodeLauncher.LittleGuy";

// Demo card height. A bit taller than the real 48px prompt so there's a clear
// gap between the prompt text and the peeking guy.
const CARD_H: i32 = 96;

// How far below the card's bottom edge his feet rest, so only the top of his
// head + eyes crest the edge — the rest is masked by overflow:hidden.
const REST_FEET_BELOW: f64 = 37.0;

// Extra distance he sinks below the resting peek when fully hidden.
const HIDE_DISTANCE: f64 = 120.0;

// Spring constants. Stiffness/damping picked for a lively pop with a little
// overshoot; bump `damping` up to settle faster, down for more bounce.
const STIFFNESS: f64 = 220.0;
const DAMPING: f64 = 18.0;

// One full show/hide cycle, in seconds (visible for SHOW_FOR, then hidden).
const CYCLE: f64 = 5.0;
const SHOW_FOR: f64 = 3.0;

struct Anim {
    // Spring state. `pos` is 0 = fully hidden, 1 = resting; it can overshoot
    // past 1 on the way in, which is exactly the pop we want.
    pos: f64,
    vel: f64,
    last_frame_us: Option<i64>,
    start_us: Option<i64>,
    elapsed: f64,
}

impl Anim {
    fn new() -> Self {
        Self {
            pos: 0.0,
            vel: 0.0,
            last_frame_us: None,
            start_us: None,
            elapsed: 0.0,
        }
    }

    // Advance the spring one frame toward `target` (0 or 1). `dt` in seconds.
    fn step(&mut self, target: f64, dt: f64) {
        let force = STIFFNESS * (target - self.pos) - DAMPING * self.vel;
        self.vel += force * dt;
        self.pos += self.vel * dt;
    }
}

fn main() -> glib::ExitCode {
    // NON_UNIQUE so each `cargo run` is its own instance — otherwise GTK's
    // single-instance behaviour makes a relaunch just ping the existing window
    // and exit, and you keep seeing the old build.
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run()
}

fn build(app: &Application) {
    install_css();

    let guy = DrawingArea::builder()
        .halign(Align::Fill)
        .valign(Align::Fill)
        .hexpand(true)
        .vexpand(true)
        .can_target(false)
        .build();

    let anim = Rc::new(RefCell::new(Anim::new()));

    // Draw func: read current spring/idle state and paint the guy.
    let anim_for_draw = anim.clone();
    guy.set_draw_func(move |_, cr, w, h| {
        let a = anim_for_draw.borrow();
        draw_guy(cr, w, h, a.pos, a.vel, a.elapsed);
    });

    // Tick callback: integrate the spring against the frame clock (vsync-paced).
    let anim_for_tick = anim.clone();
    guy.add_tick_callback(move |area, clock| {
        let now = clock.frame_time();
        let mut a = anim_for_tick.borrow_mut();

        let start = *a.start_us.get_or_insert(now);
        a.elapsed = (now - start) as f64 / 1_000_000.0;

        let dt = match a.last_frame_us {
            Some(prev) => ((now - prev) as f64 / 1_000_000.0).min(1.0 / 30.0),
            None => 0.0,
        };
        a.last_frame_us = Some(now);

        // Loop the show/hide target so the motion is easy to watch and tune.
        let phase = a.elapsed % CYCLE;
        let target = if phase < SHOW_FOR { 1.0 } else { 0.0 };

        if dt > 0.0 {
            a.step(target, dt);
        }

        area.queue_draw();
        glib::ControlFlow::Continue
    });

    // The prompt text, sitting at the top-left like the real launcher.
    let placeholder = Label::builder()
        .label("Ask Claude...")
        .halign(Align::Start)
        .valign(Align::Start)
        .can_target(false)
        .build();
    placeholder.add_css_class("placeholder");

    // Guy overlays the prompt content, both inside the card. The overlay must
    // expand to fill the card's full height, otherwise it shrinks to the
    // label's size and the guy gets pinned to a thin strip at the top.
    let inner = Overlay::new();
    inner.set_hexpand(true);
    inner.set_vexpand(true);
    inner.set_child(Some(&placeholder));
    inner.add_overlay(&guy);

    // The rounded card. `overflow: Hidden` is what masks the guy to the
    // rounded border so he reads as *inside* the window.
    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .height_request(CARD_H)
        .build();
    card.add_css_class("popup");
    card.set_overflow(Overflow::Hidden);
    card.set_margin_start(8);
    card.set_margin_end(8);
    card.set_margin_top(8);
    card.set_margin_bottom(8);
    card.append(&inner);

    // No title bar, so make the surface draggable + Escape-to-close.
    let handle = WindowHandle::builder().child(&card).build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Little Guy")
        .default_width(568)
        .default_height(CARD_H + 16)
        .decorated(false)
        .resizable(false)
        .child(&handle)
        .build();
    window.add_css_class("launcher");

    let window_for_key = window.clone();
    let key = EventControllerKey::new();
    key.connect_key_pressed(move |_, keyval, _, _| {
        if keyval == gdk::Key::Escape {
            window_for_key.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key);

    window.present();
}

// Paint the character into the card's interior (`w`x`h`). `pos` 0..~1.1 drives
// entrance offset + alpha, `vel` drives squash-and-stretch, `t` (seconds)
// drives idle bob + blink. He's anchored to the lower-right; the card's
// overflow:hidden masks whatever pokes past the rounded edge.
fn draw_guy(cr: &gtk4::cairo::Context, w: i32, h: i32, pos: f64, vel: f64, t: f64) {
    let visible = pos.clamp(0.0, 1.0);
    if visible <= 0.001 {
        return;
    }

    // Squash-and-stretch from vertical velocity: rising fast = tall + thin,
    // sinking = squat + wide. Anchored at his feet so he doesn't float.
    let stretch = (vel * 0.028).clamp(-0.30, 0.30);
    let sx = 1.0 - stretch * 0.6;
    let sy = 1.0 + stretch;

    // Entrance offset + a gentle idle bob once he's mostly settled. He rests
    // with his feet below the card's bottom edge so only head + eyes peek over.
    let bob = (t * 2.4).sin() * 2.0 * visible;
    let offset_y = (1.0 - pos) * HIDE_DISTANCE;
    let feet_x = (w as f64) - 58.0;
    let feet_y = (h as f64) + REST_FEET_BELOW + offset_y + bob;

    // Blink: a quick triangular dip every ~3.4s.
    let bt = t % 3.4;
    let blink = if bt < 0.14 {
        1.0 - (bt / 0.07 - 1.0).abs()
    } else {
        0.0
    }
    .clamp(0.0, 1.0);

    let body_h = 90.0;
    let body_w = 70.0;

    cr.save().unwrap();
    cr.translate(feet_x, feet_y);
    cr.scale(sx, sy);

    let cx = 0.0;
    let cy = -body_h * 0.5;

    // Body: a rounded blob (ellipse) in launcher-pill yellow.
    cr.save().unwrap();
    cr.translate(cx, cy);
    cr.scale(body_w * 0.5, body_h * 0.5);
    cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
    cr.restore().unwrap();
    cr.set_source_rgba(0.961, 0.773, 0.094, visible); // #f5c518
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.18 * visible);
    cr.set_line_width(2.0);
    let _ = cr.stroke();

    // A little feet pair so the squash has something to push against.
    for fx in [-16.0, 16.0] {
        cr.save().unwrap();
        cr.translate(fx, -6.0);
        cr.scale(11.0, 7.0);
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(0.85, 0.66, 0.05, visible);
        let _ = cr.fill();
    }

    // Eyes: whites + dark pupils. Pupils ride low so he's peering down over the
    // edge to the outside, with a slight side-to-side drift.
    let look = (t * 2.4).cos() * 1.5;
    let eye_y = cy - 6.0;
    for ex in [-14.0, 14.0] {
        // White
        cr.save().unwrap();
        cr.translate(ex, eye_y);
        cr.scale(10.0, 10.0 * (1.0 - blink * 0.92));
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(1.0, 1.0, 1.0, visible);
        let _ = cr.fill();

        // Pupil — pushed down toward the edge he's peeking over.
        cr.save().unwrap();
        cr.translate(ex + look * 0.6, eye_y + 3.0);
        cr.scale(4.2, 4.2 * (1.0 - blink * 0.92));
        cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
        cr.restore().unwrap();
        cr.set_source_rgba(0.10, 0.10, 0.10, visible);
        let _ = cr.fill();
    }

    // A small highlight so he reads as glossy, not flat.
    cr.save().unwrap();
    cr.translate(cx - 16.0, cy - 22.0);
    cr.scale(9.0, 6.0);
    cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
    cr.restore().unwrap();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.30 * visible);
    let _ = cr.fill();

    cr.restore().unwrap();
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_string(
        "
window.launcher { background: transparent; }

box.popup {
    background: rgba(30, 30, 40, 0.92);
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    /* No bottom padding so the guy's canvas reaches the rounded bottom edge,
       closing the gap below him; rounded corners clip him cleanly. */
    padding: 10px 10px 0 10px;
}

label.placeholder {
    color: rgba(240, 240, 240, 0.4);
    font-size: 14pt;
}
",
    );
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
