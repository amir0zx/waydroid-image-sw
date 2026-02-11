use anyhow::{bail, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Header,
    Field,
    Button,
}

struct Field {
    value: String,
    cursor: usize,
}

struct Item {
    kind: ItemKind,
    label: &'static str,
    field_idx: Option<usize>,
}

struct App {
    fields: Vec<Field>,
    items: Vec<Item>,
    selected: usize,
}

#[derive(Default)]
struct Config {
    tv_system_url: String,
    tv_vendor_url: String,
    a13_system_url: String,
    a13_vendor_url: String,
    tv_system_path: String,
    tv_vendor_path: String,
    a13_system_path: String,
    a13_vendor_path: String,
    tv_system_sha: String,
    tv_vendor_sha: String,
    a13_system_sha: String,
    a13_vendor_sha: String,
}

fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new();

    let result = run_ui(&mut terminal, &mut app);

    restore_terminal(&mut terminal)?;

    let cfg = result?;

    let mut terminal = init_terminal()?;
    let result = run_tasks(&mut terminal, cfg);
    restore_terminal(&mut terminal)?;

    result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

impl App {
    fn new() -> Self {
        let fields = vec![
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
            Field::new(),
        ];

        let mut items = Vec::new();
        items.push(Item::header("Download URLs (optional)"));
        items.push(Item::field("TV system.img URL", 0));
        items.push(Item::field("TV vendor.img URL", 1));
        items.push(Item::field("A13 system.img URL", 2));
        items.push(Item::field("A13 vendor.img URL", 3));
        items.push(Item::header("Local paths (required if no URL)"));
        items.push(Item::field("TV system.img path", 4));
        items.push(Item::field("TV vendor.img path", 5));
        items.push(Item::field("A13 system.img path", 6));
        items.push(Item::field("A13 vendor.img path", 7));
        items.push(Item::header("Checksums (optional)"));
        items.push(Item::field("TV system.img SHA256", 8));
        items.push(Item::field("TV vendor.img SHA256", 9));
        items.push(Item::field("A13 system.img SHA256", 10));
        items.push(Item::field("A13 vendor.img SHA256", 11));
        items.push(Item::button("[ Start ]"));
        items.push(Item::button("[ Cancel ]"));

        let selected = items
            .iter()
            .position(|i| i.kind == ItemKind::Field)
            .unwrap_or(0);

        Self {
            fields,
            items,
            selected,
        }
    }

    fn next_selectable(&mut self) {
        let mut idx = self.selected;
        for _ in 0..self.items.len() {
            idx = (idx + 1) % self.items.len();
            if self.items[idx].kind != ItemKind::Header {
                self.selected = idx;
                return;
            }
        }
    }

    fn prev_selectable(&mut self) {
        let mut idx = self.selected;
        for _ in 0..self.items.len() {
            idx = (idx + self.items.len() - 1) % self.items.len();
            if self.items[idx].kind != ItemKind::Header {
                self.selected = idx;
                return;
            }
        }
    }

    fn selected_field_mut(&mut self) -> Option<&mut Field> {
        let item = &self.items[self.selected];
        let idx = item.field_idx?;
        self.fields.get_mut(idx)
    }

    fn to_config(&self) -> Config {
        let mut cfg = Config::default();
        cfg.tv_system_url = self.fields[0].value.clone();
        cfg.tv_vendor_url = self.fields[1].value.clone();
        cfg.a13_system_url = self.fields[2].value.clone();
        cfg.a13_vendor_url = self.fields[3].value.clone();
        cfg.tv_system_path = self.fields[4].value.clone();
        cfg.tv_vendor_path = self.fields[5].value.clone();
        cfg.a13_system_path = self.fields[6].value.clone();
        cfg.a13_vendor_path = self.fields[7].value.clone();
        cfg.tv_system_sha = self.fields[8].value.clone();
        cfg.tv_vendor_sha = self.fields[9].value.clone();
        cfg.a13_system_sha = self.fields[10].value.clone();
        cfg.a13_vendor_sha = self.fields[11].value.clone();
        cfg
    }
}

impl Field {
    fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            let mut idx = self.cursor;
            while !self.value.is_char_boundary(idx) {
                idx -= 1;
            }
            let prev = self.value[..idx].chars().last().map(|c| c.len_utf8()).unwrap_or(1);
            let new_cursor = idx.saturating_sub(prev);
            self.value.remove(new_cursor);
            self.cursor = new_cursor;
        }
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

impl Item {
    fn header(label: &'static str) -> Self {
        Self {
            kind: ItemKind::Header,
            label,
            field_idx: None,
        }
    }
    fn field(label: &'static str, idx: usize) -> Self {
        Self {
            kind: ItemKind::Field,
            label,
            field_idx: Some(idx),
        }
    }
    fn button(label: &'static str) -> Self {
        Self {
            kind: ItemKind::Button,
            label,
            field_idx: None,
        }
    }
}

fn run_ui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<Config> {
    loop {
        terminal.draw(|f| draw_input(f, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key)? {
                    let cfg = app.to_config();
                    validate(&cfg).map_err(|e| anyhow::anyhow!(e))?;
                    return Ok(cfg);
                }
            }
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => bail!("Cancelled"),
        KeyCode::Tab | KeyCode::Down => app.next_selectable(),
        KeyCode::BackTab | KeyCode::Up => app.prev_selectable(),
        KeyCode::Enter => {
            let item = &app.items[app.selected];
            if item.kind == ItemKind::Button && item.label.contains("Start") {
                return Ok(true);
            }
            if item.kind == ItemKind::Button && item.label.contains("Cancel") {
                bail!("Cancelled");
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(false);
            }
            if let Some(field) = app.selected_field_mut() {
                field.insert_char(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(field) = app.selected_field_mut() {
                field.backspace();
            }
        }
        KeyCode::Left => {
            if let Some(field) = app.selected_field_mut() {
                field.move_left();
            }
        }
        KeyCode::Right => {
            if let Some(field) = app.selected_field_mut() {
                field.move_right();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn draw_input(f: &mut Frame, app: &App) {
    let size = f.size();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(size);

    let title = Paragraph::new("Waydroid Image Switcher Installer")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("waydroid-image-sw"));
    f.render_widget(title, layout[0]);

    let label_width = 26u16;
    let rows: Vec<Row> = app
        .items
        .iter()
        .map(|item| match item.kind {
            ItemKind::Header => Row::new(vec![
                Cell::from(item.label).style(Style::default().add_modifier(Modifier::BOLD)),
                Cell::from(""),
            ]),
            ItemKind::Field => {
                let idx = item.field_idx.unwrap();
                Row::new(vec![
                    Cell::from(item.label),
                    Cell::from(app.fields[idx].value.clone()),
                ])
            }
            ItemKind::Button => Row::new(vec![
                Cell::from(""),
                Cell::from(item.label).style(Style::default().fg(Color::Green)),
            ]),
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(label_width), Constraint::Min(10)])
        .block(Block::default().borders(Borders::ALL).title("Inputs"))
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));

    let mut state = TableState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(table, layout[1], &mut state);

    if let Some(item) = app.items.get(app.selected) {
        if item.kind == ItemKind::Field {
            let idx = item.field_idx.unwrap();
            let cursor = app.fields[idx].cursor as u16;
            let row = app.selected as u16;
            let x = layout[1].x + 1 + label_width + 1 + cursor;
            let y = layout[1].y + 1 + row;
            if x < layout[1].x + layout[1].width {
                f.set_cursor(x, y);
            }
        }
    }

    let help = Paragraph::new("Tab/Shift-Tab or ↑/↓ to move, Enter to start, Esc to cancel")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(help, layout[2]);
}

fn validate(cfg: &Config) -> Result<()> {
    let has_tv_url = !cfg.tv_system_url.trim().is_empty() && !cfg.tv_vendor_url.trim().is_empty();
    let has_tv_path = !cfg.tv_system_path.trim().is_empty() && !cfg.tv_vendor_path.trim().is_empty();
    let has_a13_url = !cfg.a13_system_url.trim().is_empty() && !cfg.a13_vendor_url.trim().is_empty();
    let has_a13_path = !cfg.a13_system_path.trim().is_empty() && !cfg.a13_vendor_path.trim().is_empty();

    if !has_tv_url && !has_tv_path {
        bail!("TV images: provide URLs or local paths");
    }
    if !has_a13_url && !has_a13_path {
        bail!("A13 images: provide URLs or local paths");
    }
    Ok(())
}

fn run_tasks(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, cfg: Config) -> Result<()> {
    let mut logs: Vec<String> = Vec::new();
    let mut push = |msg: &str| {
        logs.push(msg.to_string());
        let _ = terminal.draw(|f| draw_logs(f, &logs));
    };

    push("Starting...");

    let home = dirs::home_dir().context("Failed to resolve home directory")?;
    let base = home.join("waydroid-images");
    let dl_dir = base.join("downloads");
    fs::create_dir_all(&dl_dir)?;
    fs::create_dir_all(base.join("tv"))?;
    fs::create_dir_all(base.join("a13"))?;

    let tv_system = resolve_path("TV system.img", &cfg.tv_system_url, &cfg.tv_system_path, &dl_dir, &mut push)?;
    let tv_vendor = resolve_path("TV vendor.img", &cfg.tv_vendor_url, &cfg.tv_vendor_path, &dl_dir, &mut push)?;
    let a13_system = resolve_path("A13 system.img", &cfg.a13_system_url, &cfg.a13_system_path, &dl_dir, &mut push)?;
    let a13_vendor = resolve_path("A13 vendor.img", &cfg.a13_vendor_url, &cfg.a13_vendor_path, &dl_dir, &mut push)?;

    verify_sha("TV system.img", &tv_system, &cfg.tv_system_sha, &mut push)?;
    verify_sha("TV vendor.img", &tv_vendor, &cfg.tv_vendor_sha, &mut push)?;
    verify_sha("A13 system.img", &a13_system, &cfg.a13_system_sha, &mut push)?;
    verify_sha("A13 vendor.img", &a13_vendor, &cfg.a13_vendor_sha, &mut push)?;

    push("Moving files...");
    move_to(&tv_system, &base.join("tv/system.img"))?;
    move_to(&tv_vendor, &base.join("tv/vendor.img"))?;
    move_to(&a13_system, &base.join("a13/system.img"))?;
    move_to(&a13_vendor, &base.join("a13/vendor.img"))?;

    push("Done! Images are ready.");
    push("Run: ./waydroid-switch tv  or  ./waydroid-switch a13");

    Ok(())
}

fn draw_logs(f: &mut Frame, logs: &[String]) {
    let size = f.size();
    let block = Block::default().borders(Borders::ALL).title("Installer");
    let text = logs.join("\n");
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(p, size);
}

fn resolve_path(
    label: &str,
    url: &str,
    path: &str,
    dl_dir: &Path,
    push: &mut impl FnMut(&str),
) -> Result<PathBuf> {
    if !url.trim().is_empty() {
        let u = Url::parse(url).context("Invalid URL")?;
        let fname = u
            .path_segments()
            .and_then(|s| s.last())
            .filter(|s| !s.is_empty())
            .unwrap_or("download.img");
        let dest = dl_dir.join(fname);
        push(&format!("Downloading {}...", label));
        download(&u, &dest)?;
        return Ok(dest);
    }

    let p = PathBuf::from(path);
    if !p.is_file() {
        bail!("{} path not found: {}", label, p.display());
    }
    Ok(p)
}

fn download(url: &Url, dest: &Path) -> Result<()> {
    let mut resp = reqwest::blocking::get(url.as_str())
        .with_context(|| format!("Failed to download {}", url))?;
    if !resp.status().is_success() {
        bail!("Download failed: {}", resp.status());
    }
    let mut out = fs::File::create(dest)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
    }
    Ok(())
}

fn verify_sha(label: &str, file: &Path, expected: &str, push: &mut impl FnMut(&str)) -> Result<()> {
    if expected.trim().is_empty() {
        return Ok(());
    }
    push(&format!("Verifying {}...", label));
    let mut f = fs::File::open(file)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != expected.to_lowercase() {
        bail!("Checksum mismatch for {}", label);
    }
    Ok(())
}

fn move_to(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(_) => {
            fs::copy(src, dst)?;
            fs::remove_file(src)?;
            Ok(())
        }
    }
}
