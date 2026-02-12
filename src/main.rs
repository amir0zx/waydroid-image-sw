use anyhow::{bail, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs::home_dir;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    collections::BTreeMap,
    fs,
    io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const WAYDROID_CFG: &str = "/var/lib/waydroid/waydroid.cfg";

#[derive(Clone, Debug)]
struct ImageProfile {
    name: String,
    path: PathBuf,
}

#[derive(Clone, Debug)]
struct Field {
    label: &'static str,
    value: String,
    cursor: usize,
}

impl Field {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            value: String::new(),
            cursor: 0,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        self.value.remove(self.cursor);
    }

    fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }
}

#[derive(Debug)]
struct ManualAddState {
    fields: Vec<Field>,
    selected: usize,
}

impl ManualAddState {
    fn new() -> Self {
        Self {
            fields: vec![
                Field::new("Profile name"),
                Field::new("System image path"),
                Field::new("Vendor image path"),
            ],
            selected: 0,
        }
    }

    fn next(&mut self) {
        self.selected = (self.selected + 1) % 5;
    }

    fn prev(&mut self) {
        self.selected = (self.selected + 4) % 5;
    }

    fn selected_field_mut(&mut self) -> Option<&mut Field> {
        if self.selected < self.fields.len() {
            self.fields.get_mut(self.selected)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Profiles,
    ManualAdd,
}

#[derive(Debug)]
struct App {
    screen: Screen,
    profiles: Vec<ImageProfile>,
    selected: usize,
    current_images_path: Option<String>,
    status: String,
    manual: ManualAddState,
}

fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("waydroid-switch {}", APP_VERSION);
        return Ok(());
    }

    let profiles = discover_profiles()?;
    if profiles.is_empty() {
        bail!(
            "No image profiles found in ~/waydroid-images (need folders with system.img and vendor.img)"
        );
    }

    let current_images_path = current_images_path().ok();
    let selected = current_images_path
        .as_ref()
        .and_then(|cur| {
            profiles
                .iter()
                .position(|p| p.path.to_string_lossy().as_ref() == cur)
        })
        .unwrap_or(0);

    let mut app = App {
        screen: Screen::Profiles,
        profiles,
        selected,
        current_images_path,
        status: "Auto-scan loaded. Enter=switch, a=manual add, r=refresh, q=quit".to_string(),
        manual: ManualAddState::new(),
    };

    let mut terminal = init_terminal()?;
    let ui_result = run_ui(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    ui_result
}

fn run_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if !event::poll(Duration::from_millis(150))? {
            continue;
        }

        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.screen {
                Screen::Profiles => handle_profiles_key(app, key, terminal)?,
                Screen::ManualAdd => handle_manual_key(app, key)?,
            }
        }
    }
}

fn handle_profiles_key(
    app: &mut App,
    key: KeyEvent,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => std::process::exit(0),
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected + 1 < app.profiles.len() {
                app.selected += 1;
            }
        }
        KeyCode::Char('r') => {
            app.profiles = discover_profiles()?;
            if app.selected >= app.profiles.len() {
                app.selected = 0;
            }
            app.current_images_path = current_images_path().ok();
            app.status = "Profile list refreshed from ~/waydroid-images".to_string();
        }
        KeyCode::Char('a') => {
            app.manual = ManualAddState::new();
            app.screen = Screen::ManualAdd;
            app.status = "Manual add mode: enter profile name and image paths".to_string();
        }
        KeyCode::Enter => {
            let selected = app.profiles[app.selected].clone();
            app.status = format!("Switching to '{}'...", selected.name);
            terminal.draw(|f| draw(f, app))?;

            match switch_to_profile(&selected.path) {
                Ok(_) => {
                    app.current_images_path = Some(selected.path.to_string_lossy().to_string());
                    app.status = format!("Switched to '{}'.", selected.name);
                }
                Err(e) => {
                    app.status = format!("Switch failed: {}", e);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_manual_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.screen = Screen::Profiles;
            app.status = "Cancelled manual add".to_string();
        }
        KeyCode::Tab | KeyCode::Down => app.manual.next(),
        KeyCode::BackTab | KeyCode::Up => app.manual.prev(),
        KeyCode::Enter => {
            if app.manual.selected < 2 {
                app.manual.next();
            } else if app.manual.selected == 2 || app.manual.selected == 3 {
                match save_manual_profile(app) {
                    Ok(_) => {
                        app.screen = Screen::Profiles;
                    }
                    Err(e) => {
                        app.status = format!("Manual add failed: {}", e);
                    }
                }
            } else {
                app.screen = Screen::Profiles;
                app.status = "Cancelled manual add".to_string();
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
            if let Some(field) = app.manual.selected_field_mut() {
                field.insert_char(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(field) = app.manual.selected_field_mut() {
                field.backspace();
            }
        }
        KeyCode::Left => {
            if let Some(field) = app.manual.selected_field_mut() {
                field.move_left();
            }
        }
        KeyCode::Right => {
            if let Some(field) = app.manual.selected_field_mut() {
                field.move_right();
            }
        }
        _ => {}
    }
    Ok(())
}

fn save_manual_profile(app: &mut App) -> Result<()> {
    let name = app.manual.fields[0].value.trim();
    let system = app.manual.fields[1].value.trim();
    let vendor = app.manual.fields[2].value.trim();

    if name.is_empty() || system.is_empty() || vendor.is_empty() {
        bail!("All fields are required");
    }

    let system_path = PathBuf::from(system);
    let vendor_path = PathBuf::from(vendor);

    if !system_path.is_file() {
        bail!("System image not found: {}", system_path.display());
    }
    if !vendor_path.is_file() {
        bail!("Vendor image not found: {}", vendor_path.display());
    }

    let base = home_dir()
        .context("Failed to resolve HOME")?
        .join("waydroid-images");
    fs::create_dir_all(&base)?;

    let safe_name = name.replace('/', "-").replace('\\', "-");
    let profile_dir = base.join(&safe_name);
    fs::create_dir_all(&profile_dir)?;

    let dst_system = profile_dir.join("system.img");
    let dst_vendor = profile_dir.join("vendor.img");

    if dst_system.exists() {
        fs::remove_file(&dst_system)?;
    }
    if dst_vendor.exists() {
        fs::remove_file(&dst_vendor)?;
    }

    let system_abs = fs::canonicalize(&system_path)?;
    let vendor_abs = fs::canonicalize(&vendor_path)?;

    symlink(system_abs, &dst_system)
        .with_context(|| format!("Failed creating symlink {}", dst_system.display()))?;
    symlink(vendor_abs, &dst_vendor)
        .with_context(|| format!("Failed creating symlink {}", dst_vendor.display()))?;

    app.profiles = discover_profiles()?;
    if let Some(idx) = app.profiles.iter().position(|p| p.path == profile_dir) {
        app.selected = idx;
    }

    app.status = format!("Added profile '{}' and linked images.", safe_name);
    Ok(())
}

fn discover_profiles() -> Result<Vec<ImageProfile>> {
    let base = home_dir()
        .context("Failed to resolve HOME")?
        .join("waydroid-images");

    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut map: BTreeMap<String, PathBuf> = BTreeMap::new();
    scan_dir(&base, &base, &mut map)?;

    let profiles = map
        .into_iter()
        .map(|(name, path)| ImageProfile { name, path })
        .collect::<Vec<_>>();

    Ok(profiles)
}

fn scan_dir(dir: &Path, base: &Path, out: &mut BTreeMap<String, PathBuf>) -> Result<()> {
    let system = dir.join("system.img");
    let vendor = dir.join("vendor.img");

    if system.is_file() && vendor.is_file() {
        let name = if dir == base {
            "default".to_string()
        } else {
            dir.strip_prefix(base)
                .unwrap_or(dir)
                .to_string_lossy()
                .to_string()
        };
        out.insert(name, dir.to_path_buf());
    }

    for entry in fs::read_dir(dir).with_context(|| format!("Failed reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, base, out)?;
        }
    }
    Ok(())
}

fn current_images_path() -> Result<String> {
    let cfg = fs::read_to_string(WAYDROID_CFG)
        .with_context(|| format!("Failed to read {}", WAYDROID_CFG))?;

    for line in cfg.lines() {
        if let Some(v) = line.strip_prefix("images_path =") {
            return Ok(v.trim().to_string());
        }
    }

    bail!("images_path not found in waydroid.cfg")
}

fn switch_to_profile(path: &Path) -> Result<()> {
    if !path.join("system.img").is_file() || !path.join("vendor.img").is_file() {
        bail!("{} missing system.img/vendor.img", path.display());
    }

    let _ = run_cmd("sudo", &["waydroid", "session", "stop"]);
    let _ = run_cmd("sudo", &["waydroid", "container", "stop"]);

    let sed_expr = format!("s#^images_path = .*#images_path = {}#", path.display());
    run_cmd("sudo", &["sed", "-i", &sed_expr, WAYDROID_CFG])?;

    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
            std::env::set_var("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}/bus", xdg));
        }
    }

    run_cmd("waydroid", &["session", "start"])?;
    Ok(())
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to run: {} {}", cmd, args.join(" ")))?;

    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let msg = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "command failed".to_string()
    };

    bail!("{} {} -> {}", cmd, args.join(" "), msg)
}

fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Profiles => draw_profiles(f, app),
        Screen::ManualAdd => draw_manual_add(f, app),
    }
}

fn draw_profiles(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(f.size());

    let title = Paragraph::new("Waydroid Universal Image Switcher")
        .block(Block::default().borders(Borders::ALL).title("waydroid-switch"))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    let mut state = ListState::default();
    state.select(Some(app.selected));

    let current = app.current_images_path.as_deref().unwrap_or("(unknown)");

    let items: Vec<ListItem> = app
        .profiles
        .iter()
        .map(|p| {
            let active = p.path.to_string_lossy() == current;
            let marker = if active { "[active]" } else { "        " };
            ListItem::new(format!("{} {} -> {}", marker, p.name, p.path.display()))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Profiles (auto-scanned from ~/waydroid-images)"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut state);

    let status = Paragraph::new(format!(
        "Current images_path: {}\nStatus: {}",
        current, app.status
    ))
    .block(Block::default().borders(Borders::ALL).title("Status"))
    .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[2]);

    let help = Paragraph::new("Up/Down: move  Enter: switch  a: manual add  r: refresh  q: quit")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(help, chunks[3]);
}

fn draw_manual_add(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(f.size());

    let title = Paragraph::new("Manual Add Profile")
        .block(Block::default().borders(Borders::ALL).title("waydroid-switch"))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    let mut state = ListState::default();
    state.select(Some(app.manual.selected));

    let items = vec![
        ListItem::new(format!("{}: {}", app.manual.fields[0].label, app.manual.fields[0].value)),
        ListItem::new(format!("{}: {}", app.manual.fields[1].label, app.manual.fields[1].value)),
        ListItem::new(format!("{}: {}", app.manual.fields[2].label, app.manual.fields[2].value)),
        ListItem::new("[ Save ]"),
        ListItem::new("[ Cancel ]"),
    ];

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Enter profile details"))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut state);

    let status = Paragraph::new(format!("Status: {}", app.status))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });
    f.render_widget(status, chunks[2]);

    let help = Paragraph::new("Type to edit  Tab/Up/Down to move  Enter to save  Esc to cancel")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(help, chunks[3]);

    if app.manual.selected < 3 {
        let field = &app.manual.fields[app.manual.selected];
        let x = chunks[1].x + 4 + field.label.len() as u16 + 2 + field.cursor as u16;
        let y = chunks[1].y + 1 + app.manual.selected as u16;
        if x < chunks[1].x + chunks[1].width {
            f.set_cursor(x, y);
        }
    }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
