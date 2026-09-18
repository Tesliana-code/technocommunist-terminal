use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
};

const MINERAL: Color = Color::Rgb(108, 148, 132);
const PURPLE: Color = Color::Rgb(174, 155, 208);
const AMBER: Color = Color::Rgb(214, 168, 95);
const MUTED: Color = Color::Rgb(122, 128, 132);
const TEXT: Color = Color::Rgb(214, 216, 210);

#[derive(Clone)]
struct Record {
    occurrence: String,
    state: String,
    trust: String,
    allocation: String,
    note: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    None,
    Inspect,
    Counterexamples,
    Allocation,
}

struct App {
    records: Vec<Record>,
    selected: usize,
    pane: Pane,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let record =
            |occurrence: &str, state: &str, trust: &str, allocation: &str, note: &str| Record {
                occurrence: occurrence.into(),
                state: state.into(),
                trust: trust.into(),
                allocation: allocation.into(),
                note: note.into(),
            };

        Self {
            records: vec![
                record(
                    "N:evt-184",
                    "SETTLED",
                    "AUTHORITATIVE",
                    "12.40%",
                    "Canonical occurrence settled with authoritative identity evidence.",
                ),
                record(
                    "N:evt-185",
                    "RESERVED",
                    "COMPLETE",
                    "...",
                    "Occurrence reserved. Economic effect is not represented as complete.",
                ),
                record(
                    "N:evt-186",
                    "UNRESOLVED",
                    "HISTORY ?",
                    "---",
                    "Negative evidence cannot establish novelty without complete history.",
                ),
                record(
                    "N:evt-187",
                    "CONFLICTED",
                    "PAYLOAD Δ",
                    "!!!",
                    "Same modeled occurrence identity carries conflicting payload evidence.",
                ),
            ],
            selected: 0,
            pane: Pane::None,
            quit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.records.len().saturating_sub(1))
            }
            KeyCode::Esc => self.pane = Pane::None,
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'q' => self.quit = true,
                'i' => {
                    self.pane = if self.pane == Pane::Inspect {
                        Pane::None
                    } else {
                        Pane::Inspect
                    };
                }
                'x' => {
                    self.pane = if self.pane == Pane::Counterexamples {
                        Pane::None
                    } else {
                        Pane::Counterexamples
                    };
                }
                'a' => {
                    self.pane = if self.pane == Pane::Allocation {
                        Pane::None
                    } else {
                        Pane::Allocation
                    };
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(4),
            ])
            .split(frame.area());

        self.header(frame, root[0]);

        if self.pane == Pane::None {
            self.table(frame, root[1]);
        } else {
            let pane_constraints = if self.pane == Pane::Allocation {
                [Constraint::Percentage(65), Constraint::Percentage(35)]
            } else {
                [Constraint::Percentage(68), Constraint::Percentage(32)]
            };

            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(pane_constraints)
                .split(root[1]);

            self.table(frame, body[0]);

            match self.pane {
                Pane::Inspect => self.inspect(frame, body[1]),
                Pane::Counterexamples => self.counterexamples(frame, body[1]),
                Pane::Allocation => self.allocation(frame, body[1]),
                Pane::None => {}
            }
        }

        self.footer(frame, root[2]);
    }

    fn header(&self, frame: &mut Frame, area: Rect) {
        let widget = Paragraph::new(Line::from(vec![
            Span::styled(
                " DISTRIBUTION MACHINE ",
                Style::default().fg(MINERAL).add_modifier(Modifier::BOLD),
            ),
            Span::styled("// PP-0.1", Style::default().fg(PURPLE)),
            Span::styled("  READ-ONLY FIXTURE", Style::default().fg(MUTED)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn table(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new(vec!["OCCURRENCE", "STATE", "TRUST", "ALLOCATION"])
            .style(Style::default().fg(PURPLE).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows = self.records.iter().enumerate().map(|(index, record)| {
            let marker = if index == self.selected { "▶" } else { " " };

            let style = if index == self.selected {
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };

            Row::new(vec![
                format!("{marker} {}", record.occurrence),
                record.state.clone(),
                record.trust.clone(),
                record.allocation.clone(),
            ])
            .style(style)
        });

        let widget = Table::new(
            rows,
            [
                Constraint::Length(20),
                Constraint::Length(16),
                Constraint::Length(18),
                Constraint::Min(12),
            ],
        )
        .header(header)
        .column_spacing(1)
        .block(
            Block::default()
                .title(Line::from(Span::styled(
                    " OPERATIONAL STATE ",
                    Style::default().fg(MINERAL),
                )))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn inspect(&self, frame: &mut Frame, area: Rect) {
        let record = &self.records[self.selected];

        let widget = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Occurrence  ", Style::default().fg(MUTED)),
                Span::styled(record.occurrence.as_str(), Style::default().fg(AMBER)),
            ]),
            Line::from(vec![
                Span::styled("State       ", Style::default().fg(MUTED)),
                Span::styled(record.state.as_str(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Trust       ", Style::default().fg(MUTED)),
                Span::styled(record.trust.as_str(), Style::default().fg(PURPLE)),
            ]),
            Line::from(vec![
                Span::styled("Allocation  ", Style::default().fg(MUTED)),
                Span::styled(record.allocation.as_str(), Style::default().fg(TEXT)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                record.note.as_str(),
                Style::default().fg(TEXT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "fixture data only · no writes",
                Style::default().fg(MUTED),
            )),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(Line::from(Span::styled(
                    " I // INSPECT ",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER)),
        );

        frame.render_widget(widget, area);
    }

    fn allocation(&self, frame: &mut Frame, area: Rect) {
        let record = &self.records[self.selected];

        let (contributor, verified_claim, exact_amount, pool, status, evidence, uncertainty, why) =
            match self.selected {
                0 => (
                    "Platform / Infrastructure",
                    "EUR 6,200.00",
                    "EUR 6,200.00",
                    "CONTRIBUTION",
                    "SETTLED",
                    "7 / 8",
                    "LOW",
                    "3 causal paths + 2 corroborating sources.",
                ),
                1 => (
                    "Integration Engineering",
                    "EUR 5,800.00",
                    "EUR 5,800.00 reserved",
                    "CONTRIBUTION",
                    "RESERVED",
                    "6 / 8",
                    "MEDIUM",
                    "Claim measured; settlement remains reserved.",
                ),
                2 => (
                    "Research / Verification",
                    "---",
                    "---",
                    "CONTRIBUTION",
                    "UNRESOLVED",
                    "4 / 8",
                    "HIGH",
                    "Historical completeness is not established.",
                ),
                3 => (
                    "Operations / Delivery",
                    "CONFLICTED",
                    "---",
                    "CONTRIBUTION",
                    "REVIEW",
                    "5 / 8 + conflict",
                    "HIGH",
                    "Conflicting payload evidence blocks allocation.",
                ),
                _ => (
                    "UNMAPPED",
                    "---",
                    "---",
                    "UNMAPPED",
                    "UNRESOLVED",
                    "0 / 0",
                    "HIGH",
                    "No allocation fixture is mapped to this occurrence.",
                ),
            };

        let label = |text| Span::styled(text, Style::default().fg(MUTED));
        let value = |text| Span::styled(text, Style::default().fg(TEXT));

        let widget = Paragraph::new(vec![
            Line::from(Span::styled(
                "MONEY",
                Style::default().fg(MINERAL).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![label("Incoming       "), value("EUR 50,000.00")]),
            Line::from(vec![
                label("Distributable  "),
                Span::styled("EUR 50,000.00", Style::default().fg(AMBER)),
            ]),
            Line::from(vec![label("Available      "), value("EUR 38,000.00")]),
            Line::from(vec![label("Reserved       "), value("EUR 5,800.00")]),
            Line::from(vec![label("Settled        "), value("EUR 6,200.00")]),
            Line::from(""),
            Line::from(Span::styled(
                "SELECTED ALLOCATION",
                Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                label("Occurrence     "),
                Span::styled(record.occurrence.as_str(), Style::default().fg(AMBER)),
            ]),
            Line::from(vec![label("Contributor    "), value(contributor)]),
            Line::from(vec![label("Verified claim "), value(verified_claim)]),
            Line::from(vec![
                label("Share          "),
                Span::styled(record.allocation.as_str(), Style::default().fg(AMBER)),
            ]),
            Line::from(vec![label("Exact amount   "), value(exact_amount)]),
            Line::from(vec![label("Pool           "), value(pool)]),
            Line::from(vec![label("Status         "), value(status)]),
            Line::from(vec![label("Evidence       "), value(evidence)]),
            Line::from(vec![label("Uncertainty    "), value(uncertainty)]),
            Line::from(""),
            Line::from(Span::styled("WHY", Style::default().fg(MINERAL))),
            Line::from(value(why)),
            Line::from(""),
            Line::from(Span::styled(
                "preview fixture · no payment executed",
                Style::default().fg(MUTED),
            )),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(Line::from(Span::styled(
                    " A // ALLOCATION ",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER)),
        );

        frame.render_widget(widget, area);
    }

    fn counterexamples(&self, frame: &mut Frame, area: Rect) {
        let widget = Paragraph::new(vec![
            Line::from(Span::styled(
                "ALLOCATION COUNTEREXAMPLES",
                Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("FIND-026  ", Style::default().fg(AMBER)),
                Span::raw("ABSENCE != NOVELTY"),
            ]),
            Line::from(vec![
                Span::styled("FIND-029  ", Style::default().fg(AMBER)),
                Span::raw("effect history remains externally ambiguous"),
            ]),
            Line::from(vec![
                Span::styled("FIND-036  ", Style::default().fg(AMBER)),
                Span::raw("STRUCTURAL CONSISTENCY != AUTHENTICITY"),
            ]),
            Line::from(vec![
                Span::styled("FIND-037  ", Style::default().fg(AMBER)),
                Span::raw("SURVIVAL / AGREEMENT != COMPLETENESS"),
            ]),
            Line::from(vec![
                Span::styled("FIND-038  ", Style::default().fg(AMBER)),
                Span::raw("CURRENT AT T != CURRENT AT USE"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "counterexample surface · read-only",
                Style::default().fg(MUTED),
            )),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(Line::from(Span::styled(
                    " X // COUNTEREXAMPLES ",
                    Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
                )))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PURPLE)),
        );

        frame.render_widget(widget, area);
    }

    fn footer(&self, frame: &mut Frame, area: Rect) {
        let widget = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("[↑/↓]", Style::default().fg(AMBER)),
                Span::raw(" Select   "),
                Span::styled("[I]", Style::default().fg(AMBER)),
                Span::raw(" Inspect   "),
                Span::styled("[X]", Style::default().fg(PURPLE)),
                Span::raw(" Counterexamples   "),
                Span::styled("[Esc]", Style::default().fg(MUTED)),
                Span::raw(" Close   "),
                Span::styled("[Q]", Style::default().fg(MINERAL)),
                Span::raw(" Quit"),
            ]),
            Line::from(vec![
                Span::styled("[G]", Style::default().fg(MUTED)),
                Span::raw(" Value Graph   "),
                Span::styled("[M]", Style::default().fg(MUTED)),
                Span::raw(" Money Graph   "),
                Span::styled("[R]", Style::default().fg(MUTED)),
                Span::raw(" Replay   "),
                Span::styled("[P]", Style::default().fg(MUTED)),
                Span::raw(" Provenance   "),
                Span::styled("[A]", Style::default().fg(AMBER)),
                Span::raw(" Allocation"),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED)),
        );

        frame.render_widget(widget, area);
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| {
        let mut app = App::new();
        app.run(terminal)
    })
}
