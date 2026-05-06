use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, AlertDialog, Application, ApplicationWindow, Box as GtkBox, CssProvider, Entry,
    EventControllerKey, Orientation, Window,
};
use serde::Deserialize;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

const APP_ID: &str = "dev.dedon.ClaudeCodeLauncher";

const STYLES: &str = "
window.launcher {
    background: transparent;
}

box.popup {
    background: rgba(30, 30, 40, 0.88);
    border-radius: 14px;
    padding: 10px;
}

entry {
    background: transparent;
    border: none;
    color: #f0f0f0;
    font-size: 14pt;
    padding: 8px;
    box-shadow: none;
}

entry:focus {
    box-shadow: none;
    outline: none;
}
";

#[derive(Deserialize, Debug)]
struct Config {
    working_directory: PathBuf,
    terminal_command: Vec<String>,
}

fn config_path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env::var_os("HOME").expect("HOME env var not set");
            PathBuf::from(home).join(".config")
        });
    base.join("claude-code-launcher").join("config.toml")
}

fn load_config() -> Result<Config, Box<dyn Error>> {
    let path = config_path();
    let contents = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(activate);
    app.run()
}

fn activate(app: &Application) {
    install_css();

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            show_startup_error(
                app,
                "Failed to load config",
                &format!("{}\n\n{}", config_path().display(), e),
            );
            return;
        }
    };

    if !config.working_directory.is_dir() {
        show_startup_error(
            app,
            "Working directory does not exist",
            &format!(
                "Configured working_directory is not a directory:\n{}",
                config.working_directory.display()
            ),
        );
        return;
    }

    if config.terminal_command.is_empty() {
        show_startup_error(
            app,
            "Invalid terminal_command",
            "Configured terminal_command must be a non-empty array, e.g.\n\n\
             terminal_command = [\"ptyxis\", \"--new-window\", \"--working-directory\", \
             \"{cwd}\", \"--\", \"claude\", \"{prompt}\"]",
        );
        return;
    }

    build_ui(app, &config);
}

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_string(STYLES);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_ui(app: &Application, config: &Config) {
    let entry = Entry::builder()
        .placeholder_text("Ask Claude...")
        .build();

    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .build();
    container.add_css_class("popup");
    container.append(&entry);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Claude Code Launcher")
        .default_width(560)
        .decorated(false)
        .resizable(false)
        .child(&container)
        .build();
    window.add_css_class("launcher");

    let working_dir = config.working_directory.clone();
    let terminal_command = config.terminal_command.clone();
    let window_for_submit = window.clone();
    entry.connect_activate(move |entry| {
        let text = entry.text();
        let prompt = text.trim();
        if prompt.is_empty() {
            return;
        }
        match launch_terminal(prompt, &working_dir, &terminal_command) {
            Ok(_) => window_for_submit.close(),
            Err(e) => show_error(&window_for_submit, "Failed to launch", &e.to_string()),
        }
    });

    let key_controller = EventControllerKey::new();
    let window_for_esc = window.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            window_for_esc.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    window.present();
    entry.grab_focus();
}

fn launch_terminal(
    prompt: &str,
    working_dir: &Path,
    command_template: &[String],
) -> std::io::Result<()> {
    let cwd_str = working_dir.to_string_lossy();
    let args: Vec<String> = command_template
        .iter()
        .map(|arg| arg.replace("{cwd}", &cwd_str).replace("{prompt}", prompt))
        .collect();

    build_host_command(&args).spawn()?;
    Ok(())
}

fn build_host_command(args: &[String]) -> Command {
    let in_container = Path::new("/run/.containerenv").exists();
    if in_container {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").args(args);
        c
    } else {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c
    }
}

fn show_error(parent: &impl IsA<Window>, message: &str, detail: &str) {
    AlertDialog::builder()
        .message(message)
        .detail(detail)
        .modal(true)
        .build()
        .show(Some(parent));
}

fn show_startup_error(app: &Application, message: &str, detail: &str) {
    let dialog = AlertDialog::builder()
        .message(message)
        .detail(detail)
        .modal(true)
        .build();

    let hold_guard = app.hold();
    let app = app.clone();
    dialog.choose(
        None::<&Window>,
        None::<&gio::Cancellable>,
        move |_response| {
            drop(hold_guard);
            app.quit();
        },
    );
}
