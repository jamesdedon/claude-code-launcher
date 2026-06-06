// Visual harness for the pluggable animation system.
//
//   cargo run --example little_guy            # the little guy (default)
//   ANIM=spinner cargo run --example little_guy   # a sprite, same pipeline
//
// The ANIM env var names a built-in pack; the harness seeds the packs to a temp
// dir, loads the chosen one, and previews its spawn slot. It proves
// interchangeability: change the string, change the animation.
//
// This harness only owns the prompt-card chrome (rounded, masked via
// overflow:hidden, bottom padding dropped so animations seat flush to the
// edge) and a demo show/hide loop. All motion + appearance live in the lib.

use claude_code_launcher::anim::{self, Animation, Stage};
use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, Align, Application, ApplicationWindow, Box as GtkBox, CssProvider, DrawingArea,
    EventControllerKey, Label, Orientation, Overflow, Overlay, WindowHandle,
};
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "dev.dedon.ClaudeCodeLauncher.LittleGuy";
const CARD_H: i32 = 96;

struct Harness {
    anim: Box<dyn Animation>,
    last_us: Option<i64>,
}

fn main() -> glib::ExitCode {
    // NON_UNIQUE so each `cargo run` is its own instance instead of pinging an
    // existing window and exiting (which makes you stare at a stale build).
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build);
    app.run()
}

fn build(app: &Application) {
    install_css();

    // Seed the built-in packs to a temp dir and load the chosen one (ANIM=name,
    // default little_guy), previewing its spawn slot.
    let dir = std::env::temp_dir().join("ccl-anim-preview");
    anim::seed_builtin_packs(&dir);
    let name = std::env::var("ANIM").unwrap_or_else(|_| "little_guy".to_string());
    let (open, _submit) = anim::load_pack(&dir.join(format!("{name}.toml")));
    let harness = Rc::new(RefCell::new(Harness {
        anim: open,
        last_us: None,
    }));

    let area = DrawingArea::builder()
        .halign(Align::Fill)
        .valign(Align::Fill)
        .hexpand(true)
        .vexpand(true)
        .can_target(false)
        .build();

    let draw_h = harness.clone();
    area.set_draw_func(move |_, cr, w, h| {
        let stage = Stage {
            w: w as f64,
            h: h as f64,
        };
        draw_h.borrow().anim.draw(cr, &stage);
    });

    let tick_h = harness.clone();
    area.add_tick_callback(move |area, clock| {
        let now = clock.frame_time();
        let mut h = tick_h.borrow_mut();

        let dt = match h.last_us {
            Some(prev) => ((now - prev) as f64 / 1_000_000.0).min(1.0 / 30.0),
            None => 0.0,
        };
        h.last_us = Some(now);

        // The animation drives its own lifecycle (Once / Loop / Hold).
        if dt > 0.0 {
            h.anim.tick(dt);
        }
        let finished = h.anim.is_finished();

        area.queue_draw();
        if finished {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    // Prompt-card chrome.
    let placeholder = Label::builder()
        .label("Ask Claude...")
        .halign(Align::Start)
        .valign(Align::Start)
        .can_target(false)
        .build();
    placeholder.add_css_class("placeholder");

    // The overlay must expand to fill the card, else the animation gets pinned
    // to a thin strip at the top.
    let inner = Overlay::new();
    inner.set_hexpand(true);
    inner.set_vexpand(true);
    inner.set_child(Some(&placeholder));
    inner.add_overlay(&area);

    let card = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .height_request(CARD_H)
        .build();
    card.add_css_class("popup");
    card.set_overflow(Overflow::Hidden); // masks animations to the rounded border
    card.set_margin_start(8);
    card.set_margin_end(8);
    card.set_margin_top(8);
    card.set_margin_bottom(8);
    card.append(&inner);

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
    harness.borrow_mut().anim.show(); // kick the lifecycle off
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
    /* No bottom padding so animations reach the rounded bottom edge. */
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
