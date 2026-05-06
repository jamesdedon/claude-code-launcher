use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, AlertDialog, Align, Application, ApplicationWindow, Box as GtkBox,
    CssProvider, EventControllerKey, Label, Orientation, Overlay, PolicyType, PropagationPhase,
    ScrolledWindow, TextView, Window, WindowHandle, WrapMode,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::{env, fs};

const APP_ID: &str = "dev.dedon.ClaudeCodeLauncher";
const MAX_HEIGHT: i32 = 320;
const MIN_HEIGHT: i32 = 48;

const STYLES: &str = "
window.launcher {
    background: transparent;
}

box.popup {
    background: rgba(30, 30, 40, 0.92);
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.06);
    padding: 10px;
}

scrolledwindow {
    background: transparent;
    border: none;
}

textview, textview text {
    background: transparent;
    color: #f0f0f0;
    font-size: 14pt;
}

label.placeholder {
    color: rgba(240, 240, 240, 0.4);
    font-size: 14pt;
}

label.project {
    color: rgba(240, 240, 240, 0.55);
    font-size: 10pt;
    margin-bottom: 4px;
    margin-start: 2px;
}

box.completions {
    margin-top: 6px;
}

label.completion {
    color: rgba(240, 240, 240, 0.85);
    font-size: 11pt;
    padding: 4px 6px;
    border-radius: 6px;
}

label.completion.selected {
    background: rgba(255, 255, 255, 0.10);
}
";

const MAX_COMPLETIONS_SHOWN: usize = 8;

#[derive(Deserialize, Debug)]
struct Config {
    working_directory: PathBuf,
    terminal_command: Vec<String>,
    #[serde(default = "default_title")]
    title: String,
    #[serde(default = "default_history_size")]
    history_size: usize,
    #[serde(default)]
    projects: Vec<RawProject>,
    #[serde(default = "default_resume_args")]
    resume_args: Vec<String>,
}

fn default_resume_args() -> Vec<String> {
    vec!["--resume".to_string()]
}

#[derive(Deserialize, Debug, Clone)]
struct RawProject {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct Project {
    name: String,
    path: PathBuf,
}

fn default_title() -> String {
    "Ask Claude...".to_string()
}

fn default_history_size() -> usize {
    100
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

fn state_dir() -> PathBuf {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = env::var_os("HOME").expect("HOME env var not set");
            PathBuf::from(home).join(".local").join("state")
        });
    base.join("claude-code-launcher")
}

fn history_path() -> PathBuf {
    state_dir().join("history.toml")
}

fn load_config() -> Result<Config, Box<dyn Error>> {
    let path = config_path();
    let contents = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

#[derive(Deserialize, Serialize, Default, Debug)]
struct HistoryFile {
    #[serde(default)]
    entries: Vec<String>,
}

fn load_history() -> Vec<String> {
    let path = history_path();
    let Ok(contents) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    match toml::from_str::<HistoryFile>(&contents) {
        Ok(h) => h.entries,
        Err(_) => Vec::new(),
    }
}

fn load_slash_commands() -> Vec<String> {
    let Some(home) = env::var_os("HOME") else {
        return Vec::new();
    };
    let dir = PathBuf::from(home).join(".claude").join("commands");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            out.push(stem.to_string());
        }
    }
    out.sort();
    out
}

fn save_history(entries: &[String]) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = HistoryFile {
        entries: entries.to_vec(),
    };
    if let Ok(s) = toml::to_string(&file) {
        let _ = fs::write(&path, s);
    }
}

struct State {
    history: Vec<String>,
    history_size: usize,
    history_index: Option<usize>,
    draft: String,
    suppress_change: bool,
    projects: Vec<Project>,
    current_project: usize,
    commands: Vec<String>,
    visible_completions: Vec<usize>,
    selected_completion: usize,
}

impl State {
    fn new(
        history: Vec<String>,
        history_size: usize,
        projects: Vec<Project>,
        commands: Vec<String>,
    ) -> Self {
        Self {
            history,
            history_size,
            history_index: None,
            draft: String::new(),
            suppress_change: false,
            projects,
            current_project: 0,
            commands,
            visible_completions: Vec::new(),
            selected_completion: 0,
        }
    }

    fn completions_visible(&self) -> bool {
        !self.visible_completions.is_empty()
    }

    fn update_completions(&mut self, buffer_text: &str) {
        self.visible_completions.clear();
        self.selected_completion = 0;
        let Some(slug) = slash_slug(buffer_text) else {
            return;
        };
        for (i, name) in self.commands.iter().enumerate() {
            if name.starts_with(slug) {
                self.visible_completions.push(i);
                if self.visible_completions.len() >= MAX_COMPLETIONS_SHOWN {
                    break;
                }
            }
        }
    }

    fn move_completion(&mut self, forward: bool) {
        if self.visible_completions.is_empty() {
            return;
        }
        let n = self.visible_completions.len();
        self.selected_completion = if forward {
            (self.selected_completion + 1) % n
        } else {
            (self.selected_completion + n - 1) % n
        };
    }

    fn selected_command_name(&self) -> Option<&str> {
        let idx = *self.visible_completions.get(self.selected_completion)?;
        Some(self.commands[idx].as_str())
    }

    fn cycle_project(&mut self, forward: bool) {
        if self.projects.len() <= 1 {
            return;
        }
        let n = self.projects.len();
        self.current_project = if forward {
            (self.current_project + 1) % n
        } else {
            (self.current_project + n - 1) % n
        };
    }

    fn current_project_path(&self) -> PathBuf {
        self.projects[self.current_project].path.clone()
    }

    fn current_project_name(&self) -> &str {
        &self.projects[self.current_project].name
    }

    fn record(&mut self, prompt: &str) {
        if prompt.is_empty() {
            return;
        }
        if self.history.last().map(String::as_str) == Some(prompt) {
            return;
        }
        self.history.push(prompt.to_string());
        let max = self.history_size.max(1);
        if self.history.len() > max {
            let drop = self.history.len() - max;
            self.history.drain(0..drop);
        }
        save_history(&self.history);
    }
}

fn slash_slug(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('/')?;
    let end = rest
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    Some(&rest[..end])
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

    let projects = match build_project_list(&config) {
        Ok(p) => p,
        Err(msg) => {
            show_startup_error(app, "Invalid project configuration", &msg);
            return;
        }
    };

    build_ui(app, &config, projects);
}

fn build_project_list(config: &Config) -> Result<Vec<Project>, String> {
    if config.projects.is_empty() {
        return Ok(vec![Project {
            name: "default".to_string(),
            path: config.working_directory.clone(),
        }]);
    }
    let mut out = Vec::with_capacity(config.projects.len());
    for raw in &config.projects {
        if raw.name.trim().is_empty() {
            return Err("Each [[projects]] entry needs a non-empty `name`.".to_string());
        }
        if !raw.path.is_dir() {
            return Err(format!(
                "Project `{}` path is not a directory:\n{}",
                raw.name,
                raw.path.display()
            ));
        }
        out.push(Project {
            name: raw.name.clone(),
            path: raw.path.clone(),
        });
    }
    Ok(out)
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

fn build_ui(app: &Application, config: &Config, projects: Vec<Project>) {
    let text_view = TextView::builder()
        .accepts_tab(false)
        .wrap_mode(WrapMode::WordChar)
        .left_margin(2)
        .right_margin(2)
        .top_margin(2)
        .bottom_margin(2)
        .build();

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .max_content_height(MAX_HEIGHT)
        .min_content_height(MIN_HEIGHT)
        .max_content_width(540)
        .min_content_width(540)
        .child(&text_view)
        .build();

    let placeholder = Label::builder()
        .label(&config.title)
        .halign(Align::Start)
        .valign(Align::Start)
        .can_target(false)
        .margin_start(2)
        .margin_top(2)
        .build();
    placeholder.add_css_class("placeholder");

    let overlay = Overlay::new();
    overlay.set_child(Some(&scrolled));
    overlay.add_overlay(&placeholder);

    let buffer = text_view.buffer();
    let multi_project = projects.len() > 1;
    let state = Rc::new(RefCell::new(State::new(
        load_history(),
        config.history_size,
        projects,
        load_slash_commands(),
    )));

    let completions_box = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .visible(false)
        .build();
    completions_box.add_css_class("completions");

    let placeholder_for_buffer = placeholder.clone();
    let state_for_change = state.clone();
    let completions_for_change = completions_box.clone();
    buffer.connect_changed(move |buf| {
        placeholder_for_buffer.set_visible(buf.char_count() == 0);
        let mut st = state_for_change.borrow_mut();
        if !st.suppress_change {
            st.history_index = None;
            let (s, e) = buf.bounds();
            let text = buf.text(&s, &e, false).to_string();
            st.update_completions(&text);
        }
        render_completions(&completions_for_change, &st);
    });
    placeholder.set_visible(buffer.char_count() == 0);

    let container = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .build();
    container.add_css_class("popup");

    let project_label = Label::builder()
        .halign(Align::Start)
        .can_target(false)
        .visible(multi_project)
        .build();
    project_label.add_css_class("project");
    project_label.set_label(state.borrow().current_project_name());
    container.append(&project_label);

    container.append(&overlay);
    container.append(&completions_box);

    let window_handle = WindowHandle::builder().child(&container).build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Claude Code Launcher")
        .default_width(560)
        .decorated(false)
        .resizable(false)
        .child(&window_handle)
        .build();
    window.add_css_class("launcher");

    let terminal_command = config.terminal_command.clone();
    let resume_args = config.resume_args.clone();
    let window_for_key = window.clone();
    let buffer_for_key = text_view.buffer();
    let state_for_key = state.clone();
    let project_label_for_key = project_label.clone();
    let completions_for_key = completions_box.clone();

    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let has_shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        let has_ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);

        if key == gdk::Key::Escape {
            let visible = state_for_key.borrow().completions_visible();
            if visible {
                let mut st = state_for_key.borrow_mut();
                st.visible_completions.clear();
                render_completions(&completions_for_key, &st);
                return glib::Propagation::Stop;
            }
            window_for_key.close();
            return glib::Propagation::Stop;
        }

        let is_tab_key = key == gdk::Key::Tab || key == gdk::Key::ISO_Left_Tab;
        if is_tab_key && !has_ctrl {
            if state_for_key.borrow().completions_visible() {
                accept_completion(&state_for_key, &buffer_for_key, &completions_for_key);
                return glib::Propagation::Stop;
            }
        }

        let is_enter = key == gdk::Key::Return || key == gdk::Key::KP_Enter;

        if is_enter && !has_shift {
            let (start, end) = buffer_for_key.bounds();
            let text = buffer_for_key.text(&start, &end, false);
            let prompt = text.trim();
            if prompt.is_empty() && !has_ctrl {
                return glib::Propagation::Stop;
            }
            let prompt_opt = (!prompt.is_empty()).then_some(prompt);
            let working_dir = state_for_key.borrow().current_project_path();
            let extra: &[String] = if has_ctrl { &resume_args } else { &[] };
            match launch_terminal(prompt_opt, &working_dir, &terminal_command, extra) {
                Ok(_) => {
                    if let Some(p) = prompt_opt {
                        state_for_key.borrow_mut().record(p);
                    }
                    window_for_key.close();
                }
                Err(e) => show_error(&window_for_key, "Failed to launch", &e.to_string()),
            }
            return glib::Propagation::Stop;
        }

        if is_tab_key && has_ctrl {
            let forward = key == gdk::Key::Tab && !has_shift;
            let mut st = state_for_key.borrow_mut();
            st.cycle_project(forward);
            project_label_for_key.set_label(st.current_project_name());
            return glib::Propagation::Stop;
        }

        let is_up = key == gdk::Key::Up || key == gdk::Key::KP_Up;
        let is_down = key == gdk::Key::Down || key == gdk::Key::KP_Down;
        if (is_up || is_down) && !has_ctrl && !has_shift {
            if state_for_key.borrow().completions_visible() {
                let mut st = state_for_key.borrow_mut();
                st.move_completion(is_down);
                render_completions(&completions_for_key, &st);
                return glib::Propagation::Stop;
            }
            if try_history_nav(&state_for_key, &buffer_for_key, is_up) {
                return glib::Propagation::Stop;
            }
        }

        glib::Propagation::Proceed
    });
    text_view.add_controller(key_controller);

    window.present();
    text_view.grab_focus();
}

fn render_completions(container: &GtkBox, state: &State) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    if state.visible_completions.is_empty() {
        container.set_visible(false);
        return;
    }
    for (row_idx, &cmd_idx) in state.visible_completions.iter().enumerate() {
        let row = Label::builder()
            .label(format!("/{}", state.commands[cmd_idx]))
            .halign(Align::Start)
            .can_target(false)
            .build();
        row.add_css_class("completion");
        if row_idx == state.selected_completion {
            row.add_css_class("selected");
        }
        container.append(&row);
    }
    container.set_visible(true);
}

fn accept_completion(
    state: &Rc<RefCell<State>>,
    buffer: &gtk4::TextBuffer,
    completions_box: &GtkBox,
) {
    let (cmd_name, slug_len) = {
        let st = state.borrow();
        let Some(name) = st.selected_command_name() else {
            return;
        };
        let (s, e) = buffer.bounds();
        let text = buffer.text(&s, &e, false).to_string();
        let Some(slug) = slash_slug(&text) else {
            return;
        };
        (name.to_string(), slug.len())
    };

    let mut st = state.borrow_mut();
    st.suppress_change = true;
    drop(st);

    let mut start = buffer.start_iter();
    let mut end = buffer.start_iter();
    end.forward_chars((slug_len + 1) as i32); // +1 for leading '/'
    buffer.delete(&mut start, &mut end);

    let mut insert_iter = buffer.start_iter();
    let (_, e_iter) = buffer.bounds();
    let after_text = buffer.text(&insert_iter, &e_iter, false);
    let needs_space = !after_text
        .chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false);
    let inserted = if needs_space {
        format!("/{} ", cmd_name)
    } else {
        format!("/{}", cmd_name)
    };
    buffer.insert(&mut insert_iter, &inserted);
    buffer.place_cursor(&insert_iter);

    let mut st = state.borrow_mut();
    st.suppress_change = false;
    st.visible_completions.clear();
    render_completions(completions_box, &st);
}

fn try_history_nav(
    state: &Rc<RefCell<State>>,
    buffer: &gtk4::TextBuffer,
    going_older: bool,
) -> bool {
    let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
    let cursor_line = cursor_iter.line();
    let last_line = buffer.line_count() - 1;
    let at_first = cursor_line == 0;
    let at_last = cursor_line == last_line;

    if going_older && !at_first {
        return false;
    }
    if !going_older && !at_last {
        return false;
    }

    let mut st = state.borrow_mut();
    if st.history.is_empty() {
        return false;
    }

    let new_index: Option<usize> = if going_older {
        match st.history_index {
            None => {
                let (s, e) = buffer.bounds();
                st.draft = buffer.text(&s, &e, false).to_string();
                Some(st.history.len() - 1)
            }
            Some(0) => return true,
            Some(i) => Some(i - 1),
        }
    } else {
        match st.history_index {
            None => return true,
            Some(i) if i + 1 >= st.history.len() => None,
            Some(i) => Some(i + 1),
        }
    };

    let text = match new_index {
        Some(i) => st.history[i].clone(),
        None => std::mem::take(&mut st.draft),
    };
    st.history_index = new_index;
    st.suppress_change = true;
    drop(st);

    buffer.set_text(&text);
    let end = buffer.end_iter();
    buffer.place_cursor(&end);

    state.borrow_mut().suppress_change = false;
    true
}

fn launch_terminal(
    prompt: Option<&str>,
    working_dir: &Path,
    command_template: &[String],
    extra_args_before_prompt: &[String],
) -> std::io::Result<()> {
    let cwd_str = working_dir.to_string_lossy();
    let mut args: Vec<String> = Vec::with_capacity(
        command_template.len() + extra_args_before_prompt.len(),
    );
    let mut spliced = false;
    for arg in command_template {
        if !spliced && arg.contains("{prompt}") && !extra_args_before_prompt.is_empty() {
            args.extend(extra_args_before_prompt.iter().cloned());
            spliced = true;
        }
        match prompt {
            Some(p) => {
                args.push(arg.replace("{cwd}", &cwd_str).replace("{prompt}", p));
            }
            None => {
                if arg == "{prompt}" {
                    continue;
                }
                args.push(arg.replace("{cwd}", &cwd_str).replace("{prompt}", ""));
            }
        }
    }
    if !spliced {
        args.extend(extra_args_before_prompt.iter().cloned());
    }

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
