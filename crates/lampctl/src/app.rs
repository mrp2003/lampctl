//! The interactive ratatui application.
use std::time::Duration;

use lamparray::{LampArray, Rgb};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{DefaultTerminal, Frame};

use crate::color::hsv_to_rgb;

const ACCENT: Color = Color::Rgb(0xF5, 0xF5, 0xF5); // bright white accent
const DIM: Color = Color::Rgb(0xA6, 0xAE, 0xB0); // brighter, more legible gray
const TICK: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Static,
    Rainbow,
    Breathe,
    Off,
}

impl Mode {
    const ALL: [Mode; 4] = [Mode::Static, Mode::Rainbow, Mode::Breathe, Mode::Off];
    fn label(self) -> &'static str {
        match self {
            Mode::Static => "Static",
            Mode::Rainbow => "Rainbow",
            Mode::Breathe => "Breathe",
            Mode::Off => "Off",
        }
    }
    fn desc(self) -> &'static str {
        match self {
            Mode::Static => "Solid colour",
            Mode::Rainbow => "Flowing hue cycle",
            Mode::Breathe => "Pulsing glow",
            Mode::Off => "Lights out",
        }
    }
    fn animated(self) -> bool {
        matches!(self, Mode::Rainbow | Mode::Breathe)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Hue,
    Sat,
    Bri,
}

struct App {
    selected: usize,
    hue: f64, // 0..360
    sat: u8,  // 0..100 (%)
    bri: u8,  // 0..100 (%)
    focus: Focus,
    phase: f64, // animation phase
    quit: bool,
}

impl App {
    fn new() -> Self {
        App {
            selected: 0,
            hue: 188.0,
            sat: 100,
            bri: 85,
            focus: Focus::Hue,
            phase: 0.0,
            quit: false,
        }
    }

    fn mode(&self) -> Mode {
        Mode::ALL[self.selected]
    }

    /// Colour to display/apply *right now*, accounting for animation.
    fn color(&self) -> Rgb {
        let v = self.bri as f64 / 100.0;
        let s = self.sat as f64 / 100.0;
        match self.mode() {
            Mode::Off => Rgb::BLACK,
            Mode::Static => hsv_to_rgb(self.hue, s, v),
            Mode::Rainbow => hsv_to_rgb(self.phase * 90.0 % 360.0, 1.0, v),
            Mode::Breathe => {
                let m = (self.phase.sin() * 0.5 + 0.5) * v;
                hsv_to_rgb(self.hue, s, m)
            }
        }
    }

    fn apply(&self, lamp: &mut LampArray) {
        let _ = lamp.set_all(self.color());
    }

    fn run(mut self, terminal: &mut DefaultTerminal, lamp: &mut LampArray) -> anyhow::Result<()> {
        self.apply(lamp);
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(TICK)? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        self.on_key(k.code, lamp);
                    }
                }
            } else if self.mode().animated() {
                self.phase += 0.12;
                self.apply(lamp);
            }
        }
        Ok(())
    }

    fn on_key(&mut self, code: KeyCode, lamp: &mut LampArray) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(Mode::ALL.len() - 1),
            KeyCode::Char(' ') | KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Hue => Focus::Sat,
                    Focus::Sat => Focus::Bri,
                    Focus::Bri => Focus::Hue,
                }
            }
            KeyCode::Left | KeyCode::Right => {
                let step: i32 = if code == KeyCode::Right { 1 } else { -1 };
                match self.focus {
                    Focus::Hue => self.hue = (self.hue + step as f64 * 8.0).rem_euclid(360.0),
                    Focus::Sat => self.sat = (self.sat as i32 + step * 5).clamp(0, 100) as u8,
                    Focus::Bri => self.bri = (self.bri as i32 + step * 5).clamp(0, 100) as u8,
                }
            }
            _ => {}
        }
        self.apply(lamp);
    }

    fn draw(&self, f: &mut Frame) {
        let [logo, modes, adjust, footer] = Layout::vertical([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .areas(centered(f.area(), 64, 18));

        self.draw_logo(f, logo);
        self.draw_modes(f, modes);
        self.draw_adjust(f, adjust);
        self.draw_footer(f, footer);
    }

    fn draw_logo(&self, f: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(r"  ┃  ┏━┓ ┏┳┓ ┏━┓ ┏━╸ ╺┳╸ ╻  ").fg(ACCENT).bold(),
            Line::from(r"  ┃  ┣━┫ ┃┃┃ ┣━┛ ┃    ┃  ┃  ").fg(ACCENT).bold(),
            Line::from(r"  ┗━╸╹ ╹ ╹ ╹ ╹   ┗━╸  ╹  ┗━╸").fg(ACCENT).bold(),
            Line::from("keyboard light control").fg(DIM).italic(),
        ];
        f.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            area,
        );
    }

    fn draw_modes(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = Mode::ALL
            .iter()
            .map(|m| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<10}", m.label()), Style::default().bold()),
                    Span::styled(m.desc(), Style::default().fg(DIM)),
                ]))
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(self.selected));

        let list = List::new(items)
            .block(titled("Mode"))
            .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::REVERSED))
            .highlight_symbol("› ");
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_adjust(&self, f: &mut Frame, area: Rect) {
        let block = titled("Adjust");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let c = self.color();
        let cur = Color::Rgb(c.r, c.g, c.b);
        let rows = vec![
            slider(
                "Hue",
                self.hue / 360.0,
                format!("{:>3}°", self.hue as u16),
                self.focus == Focus::Hue,
                cur,
            ),
            slider(
                "Saturation",
                self.sat as f64 / 100.0,
                format!("{:>3}%", self.sat),
                self.focus == Focus::Sat,
                ACCENT,
            ),
            slider(
                "Brightness",
                self.bri as f64 / 100.0,
                format!("{:>3}%", self.bri),
                self.focus == Focus::Bri,
                ACCENT,
            ),
        ];
        f.render_widget(Paragraph::new(rows), inner);
    }

    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let hint = |k: &'static str, d: &'static str| {
            vec![
                Span::styled(format!(" {k} "), Style::default().fg(Color::Black).bg(ACCENT)),
                Span::styled(format!(" {d}   "), Style::default().fg(DIM)),
            ]
        };
        let mut spans = Vec::new();
        spans.extend(hint("↑↓", "mode"));
        spans.extend(hint("←→", "adjust"));
        spans.extend(hint("space", "h/s/b"));
        spans.extend(hint("q", "quit"));
        f.render_widget(
            Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
            area,
        );
    }
}

/// Render one labelled slider row: `Name  ██████░░░░  value`.
fn slider(name: &str, ratio: f64, value: String, focus: bool, fill: Color) -> Line<'static> {
    const W: usize = 24;
    let filled = (ratio.clamp(0.0, 1.0) * W as f64).round() as usize;
    let name_style = if focus {
        Style::default().fg(ACCENT).bold()
    } else {
        Style::default().fg(DIM)
    };
    Line::from(vec![
        Span::styled(format!(" {name:<12}"), name_style),
        Span::styled("█".repeat(filled), Style::default().fg(fill)),
        Span::styled("░".repeat(W - filled), Style::default().fg(DIM)),
        Span::styled(format!("  {value}"), Style::default().fg(DIM)),
    ])
}

fn titled(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(format!(" {title} "), Style::default().fg(ACCENT).bold()))
}

/// Centre a `width` x `height` region inside `area`, both horizontally and vertically.
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [_, row, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height.min(area.height)),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, mid, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width.min(area.width)),
        Constraint::Fill(1),
    ])
    .areas(row);
    mid
}

pub fn run() -> anyhow::Result<()> {
    let mut lamp = LampArray::open_first().map_err(|e| {
        anyhow::anyhow!(
            "could not open a LampArray device: {e}\n\
             hint: add your user to the 'input' group and re-login."
        )
    })?;
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal, &mut lamp);
    ratatui::restore();
    result
}
