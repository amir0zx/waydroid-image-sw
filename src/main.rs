use anyhow::{bail, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs::home_dir;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::{
    fs,
    io,
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

#[derive(Debug)]
struct App {
    profiles: Vec<ImageProfile>,
    selected: usize,
    current_images_path: Option<String>,
    status: String,
}

fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("waydroid-image-sw {}", APP_VERSION);
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
        .and_then(|cur| profiles.iter().position(|p| p.path.to_string_lossy() == cur.as_str()))
        .unwrap_or(0);

    let mut app = App {
        profiles,
        selected,
        current_images_path,
        status: "Use Up/Down then Enter to switch. Press q to quit.".to_string(),
    };

    let mut terminal = init_terminal()?;
    let ui_result = run_ui(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    ui_result
}

fn discover_profiles() -> Result<Vec<ImageProfile>> {
    let base = home_dir()
        .context("Failed to resolve HOME")?
        .join("waydroid-images");

    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    for entry in fs::read_dir(&base).with_context(|| format!("Failed reading {}", base.display()))? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let system = path.join("system.img");
        let vendor = path.join("vendor.img");
        if system.is_file() && vendor.is_file() {
            profiles.push(ImageProfile {
                name: entry.file_name().to_string_lossy().to_string(),
                path,
            });
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
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

            match key.code {
                KeyCode::Char('q') => return Ok(()),
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
        }
    }
}

fn switch_to_profile(path: &Path) -> Result<()> {
    if !path.join("system.img").is_file() || !path.join("vendor.img").is_file() {
        bail!("{} missing system.img/vendor.img", path.display());
    }

    // Stop container/session first; ignore failures if not running.
    let _ = run_cmd("sudo", &["waydroid", "session", "stop"]);
    let _ = run_cmd("sudo", &["waydroid", "container", "stop"]);

    let sed_expr = format!("s#^images_path = .*#images_path = {}#", path.display());
    run_cmd("sudo", &["sed", "-i", &sed_expr, WAYDROID_CFG])?;

    // Start session as user with valid DBus if needed.
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
        .block(Block::default().borders(Borders::ALL).title("waydroid-image-sw"))
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
        .block(Block::default().borders(Borders::ALL).title("Image Profiles"))
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

    let help = Paragraph::new("Up/Down: move  Enter: switch  q: quit")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(help, chunks[3]);
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
