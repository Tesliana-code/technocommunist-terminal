use std::{env, io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use distribution_machine::{
    contribution::{ContributionEvent, ContributionFactors, ContributionWeights},
    distribution::{ContributionAllocation, allocate_contribution_pool},
    ledger::MoneyLedger,
    operator_api::{OperatorApiClient, OperatorRunHistory},
    protocol::DistributionPools,
};
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
struct ContributorDraft {
    id: &'static str,
    contribution_id: &'static str,
    base_work_value: f64,
    responsibility: f64,
    criticality: f64,
    deal_impact: f64,
    dependency_reach: f64,
    evidence_confidence: f64,
}

impl ContributorDraft {
    fn to_event(&self) -> ContributionEvent {
        ContributionEvent {
            contributor_id: self.id.to_string(),
            contribution_id: self.contribution_id.to_string(),
            base_work_value: self.base_work_value,
            factors: ContributionFactors {
                responsibility: self.responsibility,
                criticality: self.criticality,
                deal_impact: self.deal_impact,
                stewardship: 0.0,
                dependency_reach: self.dependency_reach,
                evidence_confidence: self.evidence_confidence,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Revenue,
    DirectCosts,
    SharedCosts,
    Labor,
    ReserveReinvestment,
    ContributionPool,
    StewardshipPool,
    CapitalPool,
    OrganizationalReservePool,
    ResponsibilityWeight,
    CriticalityWeight,
    DealImpactWeight,
    DependencyReachWeight,
    BaseWorkValue,
    ContributorResponsibility,
    ContributorCriticality,
    ContributorDealImpact,
    ContributorDependencyReach,
    EvidenceConfidence,
}

impl Field {
    fn label(self) -> &'static str {
        match self {
            Self::Revenue => "Revenue",
            Self::DirectCosts => "Direct Costs",
            Self::SharedCosts => "Shared Costs",
            Self::Labor => "Labor",
            Self::ReserveReinvestment => "Reserve Reinvestment",
            Self::ContributionPool => "Contribution Pool",
            Self::StewardshipPool => "Stewardship Pool",
            Self::CapitalPool => "Capital Pool",
            Self::OrganizationalReservePool => "Org Reserve Pool",
            Self::ResponsibilityWeight => "Responsibility",
            Self::CriticalityWeight => "Criticality",
            Self::DealImpactWeight => "Deal Impact",
            Self::DependencyReachWeight => "Dependency Reach",
            Self::BaseWorkValue => "Base Work Value",
            Self::ContributorResponsibility => "Responsibility",
            Self::ContributorCriticality => "Criticality",
            Self::ContributorDealImpact => "Deal Impact",
            Self::ContributorDependencyReach => "Dependency Reach",
            Self::EvidenceConfidence => "Evidence Confidence",
        }
    }
}

const FIELDS: [Field; 19] = [
    Field::Revenue,
    Field::DirectCosts,
    Field::SharedCosts,
    Field::Labor,
    Field::ReserveReinvestment,
    Field::ContributionPool,
    Field::StewardshipPool,
    Field::CapitalPool,
    Field::OrganizationalReservePool,
    Field::ResponsibilityWeight,
    Field::CriticalityWeight,
    Field::DealImpactWeight,
    Field::DependencyReachWeight,
    Field::BaseWorkValue,
    Field::ContributorResponsibility,
    Field::ContributorCriticality,
    Field::ContributorDealImpact,
    Field::ContributorDependencyReach,
    Field::EvidenceConfidence,
];

struct Preview {
    surplus: f64,
    contribution_pool: f64,
    allocations: Vec<ContributionAllocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorView {
    Sandbox,
    RunHistory,
}

struct OperatorState {
    ledger: MoneyLedger,
    pools: DistributionPools,
    weights: ContributionWeights,
    contributors: Vec<ContributorDraft>,
    contributor_index: usize,
    field_index: usize,
    editing: bool,
    edit_buffer: String,
    view: OperatorView,
    history_target: Option<String>,
    run_history: Option<OperatorRunHistory>,
    history_selected: usize,
    message: String,
    quit: bool,
}

impl OperatorState {
    fn new() -> Self {
        Self {
            ledger: MoneyLedger {
                revenue: 120_000.0,
                direct_costs: 20_000.0,
                shared_costs: 10_000.0,
                labor: 40_000.0,
                reserve_reinvestment: 20_000.0,
            },
            pools: DistributionPools {
                contribution: 0.35,
                stewardship: 0.20,
                capital: 0.25,
                organizational_reserve: 0.20,
            },
            weights: ContributionWeights {
                responsibility: 0.20,
                criticality: 0.30,
                deal_impact: 0.25,
                dependency_reach: 0.25,
            },
            contributors: vec![
                ContributorDraft {
                    id: "operations",
                    contribution_id: "operations-delivery",
                    base_work_value: 1.00,
                    responsibility: 0.90,
                    criticality: 0.80,
                    deal_impact: 0.75,
                    dependency_reach: 0.70,
                    evidence_confidence: 1.00,
                },
                ContributorDraft {
                    id: "backend",
                    contribution_id: "backend-engineering",
                    base_work_value: 1.20,
                    responsibility: 0.80,
                    criticality: 0.95,
                    deal_impact: 0.80,
                    dependency_reach: 0.95,
                    evidence_confidence: 1.00,
                },
                ContributorDraft {
                    id: "integration",
                    contribution_id: "integration-engineering",
                    base_work_value: 1.10,
                    responsibility: 0.85,
                    criticality: 0.85,
                    deal_impact: 0.95,
                    dependency_reach: 0.85,
                    evidence_confidence: 0.95,
                },
            ],
            contributor_index: 0,
            field_index: 0,
            editing: false,
            edit_buffer: String::new(),
            view: OperatorView::Sandbox,
            history_target: None,
            run_history: None,
            history_selected: 0,
            message: "SANDBOX READY · deterministic preview · no writes".into(),
            quit: false,
        }
    }

    fn from_environment() -> Self {
        let mut state = Self::new();

        state.history_target = env::var("DISTRIBUTION_ENGINE_DRAFT_ID")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(id) = state.history_target.as_deref() {
            state.message = format!("HISTORY TARGET · {id} · [H] load authoritative runs");
        }

        state
    }

    fn load_run_history(&mut self) {
        let Some(draft_id) = self.history_target.clone() else {
            self.message =
                "RUN HISTORY requires DISTRIBUTION_ENGINE_DRAFT_ID · no request sent".into();
            return;
        };

        let client = match OperatorApiClient::new("http://127.0.0.1:18080") {
            Ok(client) => client,
            Err(error) => {
                self.message = format!("RUN HISTORY CONFIG ERROR · {error}");
                return;
            }
        };

        match client.list_finalized_runs_for_draft(&draft_id) {
            Ok(history) => {
                let count = history.runs.len();
                self.run_history = Some(history);
                self.history_selected = 0;
                self.view = OperatorView::RunHistory;
                self.message = format!("RUN HISTORY · {draft_id} · {count} finalized run(s)");
            }
            Err(error) => {
                self.message = format!("RUN HISTORY ERROR · {error}");
            }
        }
    }

    fn next_history_run(&mut self) {
        let count = self
            .run_history
            .as_ref()
            .map(|history| history.runs.len())
            .unwrap_or(0);

        if count > 0 {
            self.history_selected = (self.history_selected + 1).min(count - 1);
        }
    }

    fn previous_history_run(&mut self) {
        self.history_selected = self.history_selected.saturating_sub(1);
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.view = OperatorView::Sandbox;
                self.message = "SANDBOX READY · deterministic preview · no writes".into();
            }
            KeyCode::Up => self.previous_history_run(),
            KeyCode::Down => self.next_history_run(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'h') => self.load_run_history(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => self.quit = true,
            _ => {}
        }
    }

    fn active_field(&self) -> Field {
        FIELDS[self.field_index]
    }

    fn selected_contributor(&self) -> &ContributorDraft {
        &self.contributors[self.contributor_index]
    }

    fn next_field(&mut self) {
        self.field_index = (self.field_index + 1) % FIELDS.len();
    }

    fn previous_field(&mut self) {
        self.field_index = if self.field_index == 0 {
            FIELDS.len() - 1
        } else {
            self.field_index - 1
        };
    }

    fn cycle_contributor(&mut self) {
        self.contributor_index = (self.contributor_index + 1) % self.contributors.len();
        self.message = format!("Contributor: {}", self.selected_contributor().id);
    }

    fn numeric_value(&self, field: Field) -> Option<f64> {
        match field {
            Field::Revenue => Some(self.ledger.revenue),
            Field::DirectCosts => Some(self.ledger.direct_costs),
            Field::SharedCosts => Some(self.ledger.shared_costs),
            Field::Labor => Some(self.ledger.labor),
            Field::ReserveReinvestment => Some(self.ledger.reserve_reinvestment),
            Field::BaseWorkValue => Some(self.selected_contributor().base_work_value),
            _ => None,
        }
    }

    fn set_numeric_value(&mut self, field: Field, value: f64) {
        match field {
            Field::Revenue => self.ledger.revenue = value,
            Field::DirectCosts => self.ledger.direct_costs = value,
            Field::SharedCosts => self.ledger.shared_costs = value,
            Field::Labor => self.ledger.labor = value,
            Field::ReserveReinvestment => self.ledger.reserve_reinvestment = value,
            Field::BaseWorkValue => {
                self.contributors[self.contributor_index].base_work_value = value
            }
            _ => {}
        }
    }

    fn begin_edit(&mut self) {
        let field = self.active_field();

        if self.numeric_value(field).is_some() {
            self.editing = true;
            self.edit_buffer.clear();
            self.message = format!("EDIT {} · type replacement value", field.label());
        } else {
            self.message = "Slider field · use Left / Right".into();
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.editing = false;
                self.edit_buffer.clear();
                self.message = "Edit cancelled".into();
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            KeyCode::Enter => {
                if self.edit_buffer.is_empty() {
                    self.message = "No value entered".into();
                    self.editing = false;
                    return;
                }

                match self.edit_buffer.parse::<f64>() {
                    Ok(value) if value.is_finite() && value >= 0.0 => {
                        let field = self.active_field();
                        self.set_numeric_value(field, value);
                        self.message = format!("{} updated", field.label());
                        self.editing = false;
                        self.edit_buffer.clear();
                    }
                    _ => {
                        self.message = "Invalid non-negative numeric value".into();
                    }
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => self.edit_buffer.push(c),
            KeyCode::Char('.') if !self.edit_buffer.contains('.') => self.edit_buffer.push('.'),
            _ => {}
        }
    }

    fn adjust_active(&mut self, delta: f64) {
        let field = self.active_field();

        fn adjust(value: &mut f64, delta: f64) {
            *value = (*value + delta).clamp(0.0, 1.0);
        }

        match field {
            Field::ContributionPool => adjust(&mut self.pools.contribution, delta),
            Field::StewardshipPool => adjust(&mut self.pools.stewardship, delta),
            Field::CapitalPool => adjust(&mut self.pools.capital, delta),
            Field::OrganizationalReservePool => {
                adjust(&mut self.pools.organizational_reserve, delta)
            }
            Field::ResponsibilityWeight => adjust(&mut self.weights.responsibility, delta),
            Field::CriticalityWeight => adjust(&mut self.weights.criticality, delta),
            Field::DealImpactWeight => adjust(&mut self.weights.deal_impact, delta),
            Field::DependencyReachWeight => adjust(&mut self.weights.dependency_reach, delta),
            Field::ContributorResponsibility => adjust(
                &mut self.contributors[self.contributor_index].responsibility,
                delta,
            ),
            Field::ContributorCriticality => adjust(
                &mut self.contributors[self.contributor_index].criticality,
                delta,
            ),
            Field::ContributorDealImpact => adjust(
                &mut self.contributors[self.contributor_index].deal_impact,
                delta,
            ),
            Field::ContributorDependencyReach => adjust(
                &mut self.contributors[self.contributor_index].dependency_reach,
                delta,
            ),
            Field::EvidenceConfidence => adjust(
                &mut self.contributors[self.contributor_index].evidence_confidence,
                delta,
            ),
            _ => {
                self.message = "Numeric field · press E or Enter to edit".into();
                return;
            }
        }

        self.message = format!("{} {:+.0}%", field.label(), delta * 100.0);
    }

    fn preview(&self) -> Result<Preview, String> {
        self.ledger.validate().map_err(str::to_string)?;

        let surplus = self.ledger.calculate_surplus().distributable_surplus;

        if surplus <= 0.0 {
            return Err(format!(
                "No positive distributable surplus ({})",
                format_money(surplus)
            ));
        }

        let pool_amounts = self.pools.allocate(surplus).map_err(str::to_string)?;
        self.weights.validate().map_err(str::to_string)?;

        let events: Vec<ContributionEvent> = self
            .contributors
            .iter()
            .map(ContributorDraft::to_event)
            .collect();

        let allocations =
            allocate_contribution_pool(&events, &self.weights, pool_amounts.contribution)
                .map_err(str::to_string)?;

        Ok(Preview {
            surplus,
            contribution_pool: pool_amounts.contribution,
            allocations,
        })
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.view == OperatorView::RunHistory {
            self.handle_history_key(key);
            return;
        }

        if self.editing {
            self.handle_edit_key(key);
            return;
        }

        if let KeyCode::Char(c) = key.code
            && c.eq_ignore_ascii_case(&'q')
        {
            self.quit = true;
            return;
        }

        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            0.05
        } else {
            0.01
        };

        match key.code {
            KeyCode::Tab | KeyCode::Down => self.next_field(),
            KeyCode::BackTab | KeyCode::Up => self.previous_field(),
            KeyCode::Left => self.adjust_active(-step),
            KeyCode::Right => self.adjust_active(step),
            KeyCode::Char('[') => self.adjust_active(-0.01),
            KeyCode::Char(']') => self.adjust_active(0.01),
            KeyCode::Char('{') => self.adjust_active(-0.05),
            KeyCode::Char('}') => self.adjust_active(0.05),
            KeyCode::Enter => self.begin_edit(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'e') => self.begin_edit(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'c') => self.cycle_contributor(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'h') => self.load_run_history(),
            _ => {}
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(120))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_key(key);
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(20),
                Constraint::Length(4),
            ])
            .split(frame.area());

        self.draw_header(frame, root[0]);

        if self.view == OperatorView::RunHistory {
            self.draw_run_history(frame, root[1]);
            self.draw_footer(frame, root[2]);
            return;
        }

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
            .split(root[1]);

        self.draw_money_and_pools(frame, columns[0]);

        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Min(8),
            ])
            .split(columns[1]);

        self.draw_weights(frame, right[0]);
        self.draw_contributor(frame, right[1]);
        self.draw_preview(frame, right[2]);
        self.draw_footer(frame, root[2]);
    }

    fn selected_style(&self, field: Field) -> Style {
        if self.active_field() == field {
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT)
        }
    }

    fn money_line(&self, field: Field, label: &str, value: f64) -> Line<'static> {
        let selected = self.active_field() == field;
        let shown = if self.editing && selected {
            format!("> {}_", self.edit_buffer)
        } else {
            format_money(value)
        };

        Line::from(vec![
            Span::styled(format!("{label:<22}"), self.selected_style(field)),
            Span::styled(
                shown,
                if selected {
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                },
            ),
        ])
    }

    fn slider_line(&self, field: Field, label: &str, value: f64) -> Line<'static> {
        let style = self.selected_style(field);

        Line::from(vec![
            Span::styled(format!("{label:<19}"), style),
            Span::styled(format!("{:>3.0}% ", value * 100.0), style),
            Span::styled(slider(value), style),
        ])
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let mode = match self.view {
            OperatorView::Sandbox => "// OPERATOR SANDBOX",
            OperatorView::RunHistory => "// RUN HISTORY",
        };

        let widget = Paragraph::new(Line::from(vec![
            Span::styled(
                " DISTRIBUTION-ENGINE ",
                Style::default().fg(MINERAL).add_modifier(Modifier::BOLD),
            ),
            Span::styled(mode, Style::default().fg(AMBER)),
            Span::styled("  PP-0.1  DETERMINISTIC", Style::default().fg(PURPLE)),
            Span::styled("  NO AI / NO LLM", Style::default().fg(MUTED)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_money_and_pools(&self, frame: &mut Frame, area: Rect) {
        let surplus = self.ledger.calculate_surplus().distributable_surplus;
        let pool_total = self.pools.contribution
            + self.pools.stewardship
            + self.pools.capital
            + self.pools.organizational_reserve;

        let total_style = if (pool_total - 1.0).abs() < 1e-9 {
            Style::default().fg(MINERAL).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
        };

        let lines = vec![
            Line::from(Span::styled(
                "MONEY LEDGER",
                Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
            )),
            self.money_line(Field::Revenue, "Revenue", self.ledger.revenue),
            self.money_line(Field::DirectCosts, "Direct Costs", self.ledger.direct_costs),
            self.money_line(Field::SharedCosts, "Shared Costs", self.ledger.shared_costs),
            self.money_line(Field::Labor, "Labor", self.ledger.labor),
            self.money_line(
                Field::ReserveReinvestment,
                "Reserve Reinvestment",
                self.ledger.reserve_reinvestment,
            ),
            Line::from(""),
            Line::from(vec![
                Span::styled("AVAILABLE FOR DISTRIBUTION ", Style::default().fg(MUTED)),
                Span::styled(
                    format_money(surplus),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "POOL CONFIG",
                Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
            )),
            self.slider_line(
                Field::ContributionPool,
                "Contribution",
                self.pools.contribution,
            ),
            self.slider_line(
                Field::StewardshipPool,
                "Stewardship",
                self.pools.stewardship,
            ),
            self.slider_line(Field::CapitalPool, "Capital", self.pools.capital),
            self.slider_line(
                Field::OrganizationalReservePool,
                "Org Reserve",
                self.pools.organizational_reserve,
            ),
            Line::from(vec![
                Span::styled("Pool total            ", Style::default().fg(MUTED)),
                Span::styled(format!("{:.0}%", pool_total * 100.0), total_style),
                Span::styled(
                    if (pool_total - 1.0).abs() < 1e-9 {
                        "  ✓ reconciled"
                    } else {
                        "  ! must equal 100%"
                    },
                    total_style,
                ),
            ]),
        ];

        let widget = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" MONEY + POOLS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_weights(&self, frame: &mut Frame, area: Rect) {
        let total = self.weights.responsibility
            + self.weights.criticality
            + self.weights.deal_impact
            + self.weights.dependency_reach;

        let total_style = if (total - 1.0).abs() < 1e-9 {
            Style::default().fg(MINERAL).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
        };

        let widget = Paragraph::new(vec![
            self.slider_line(
                Field::ResponsibilityWeight,
                "Responsibility",
                self.weights.responsibility,
            ),
            self.slider_line(
                Field::CriticalityWeight,
                "Criticality",
                self.weights.criticality,
            ),
            self.slider_line(
                Field::DealImpactWeight,
                "Deal Impact",
                self.weights.deal_impact,
            ),
            self.slider_line(
                Field::DependencyReachWeight,
                "Dependency Reach",
                self.weights.dependency_reach,
            ),
            Line::from(vec![
                Span::styled("Weight total          ", Style::default().fg(MUTED)),
                Span::styled(format!("{:.0}%", total * 100.0), total_style),
                Span::styled(
                    if (total - 1.0).abs() < 1e-9 {
                        "  ✓ reconciled"
                    } else {
                        "  ! preview blocked"
                    },
                    total_style,
                ),
            ]),
        ])
        .block(
            Block::default()
                .title(" CONTRIBUTION WEIGHTS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PURPLE)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_contributor(&self, frame: &mut Frame, area: Rect) {
        let contributor = self.selected_contributor();

        let widget = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Active contributor  ", Style::default().fg(MUTED)),
                Span::styled(
                    contributor.id,
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  [C] next", Style::default().fg(MUTED)),
            ]),
            self.money_line(
                Field::BaseWorkValue,
                "Base Work Value",
                contributor.base_work_value,
            ),
            self.slider_line(
                Field::ContributorResponsibility,
                "Responsibility",
                contributor.responsibility,
            ),
            self.slider_line(
                Field::ContributorCriticality,
                "Criticality",
                contributor.criticality,
            ),
            self.slider_line(
                Field::ContributorDealImpact,
                "Deal Impact",
                contributor.deal_impact,
            ),
            self.slider_line(
                Field::ContributorDependencyReach,
                "Dependency Reach",
                contributor.dependency_reach,
            ),
            self.slider_line(
                Field::EvidenceConfidence,
                "Evidence Confidence",
                contributor.evidence_confidence,
            ),
        ])
        .block(
            Block::default()
                .title(" CONTRIBUTOR MEASUREMENT ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_preview(&self, frame: &mut Frame, area: Rect) {
        let lines = match self.preview() {
            Ok(preview) => {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Available       ", Style::default().fg(MUTED)),
                        Span::styled(
                            format_money(preview.surplus),
                            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("   Contribution pool   ", Style::default().fg(MUTED)),
                        Span::styled(
                            format_money(preview.contribution_pool),
                            Style::default().fg(TEXT),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        "CONTRIBUTOR          SHARE        AMOUNT        VERIFIED VALUE",
                        Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
                    )),
                ];

                let selected = self.selected_contributor().id;

                for allocation in &preview.allocations {
                    let marker = if allocation.contributor_id == selected {
                        "▶"
                    } else {
                        " "
                    };

                    let style = if allocation.contributor_id == selected {
                        Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(TEXT)
                    };

                    lines.push(Line::from(Span::styled(
                        format!(
                            "{marker} {:<17} {:>6.2}%   {:>13}   {:>8.4}",
                            allocation.contributor_id,
                            allocation.normalized_share * 100.0,
                            format_money(allocation.allocated_amount),
                            allocation.contribution_value,
                        ),
                        style,
                    )));
                }

                let total: f64 = preview
                    .allocations
                    .iter()
                    .map(|allocation| allocation.allocated_amount)
                    .sum();

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Reconcile        ", Style::default().fg(MUTED)),
                    Span::styled(
                        format!(
                            "{} / {}",
                            format_money(total),
                            format_money(preview.contribution_pool)
                        ),
                        Style::default().fg(MINERAL).add_modifier(Modifier::BOLD),
                    ),
                ]));

                lines
            }
            Err(error) => vec![
                Line::from(Span::styled(
                    "PREVIEW UNAVAILABLE",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(error, Style::default().fg(TEXT))),
                Line::from(""),
                Line::from(Span::styled(
                    "Reconcile pool/weight totals and valid inputs to restore preview.",
                    Style::default().fg(MUTED),
                )),
            ],
        };

        let widget = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" LIVE DISTRIBUTION PREVIEW ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_run_history(&self, frame: &mut Frame, area: Rect) {
        let Some(history) = self.run_history.as_ref() else {
            let widget = Paragraph::new(vec![
                Line::from(Span::styled(
                    "NO RUN HISTORY LOADED",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press H to load authoritative finalized-run metadata.",
                    Style::default().fg(TEXT),
                )),
            ])
            .block(
                Block::default()
                    .title(" FINALIZED RUN HISTORY ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MINERAL)),
            );

            frame.render_widget(widget, area);
            return;
        };

        if history.runs.is_empty() {
            let widget = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Draft  ", Style::default().fg(MUTED)),
                    Span::styled(
                        history.draft_id.as_str(),
                        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled("NO FINALIZED RUNS", Style::default().fg(TEXT))),
                Line::from(""),
                Line::from(Span::styled(
                    "Authoritative history is currently empty.",
                    Style::default().fg(MUTED),
                )),
            ])
            .block(
                Block::default()
                    .title(" FINALIZED RUN HISTORY ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MINERAL)),
            );

            frame.render_widget(widget, area);
            return;
        }

        let header = Row::new(vec![
            "REV",
            "RUN",
            "FINALIZED",
            "PROTOCOL",
            "ENGINE",
            "MANUAL",
        ])
        .style(Style::default().fg(PURPLE).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

        let rows = history.runs.iter().enumerate().map(|(index, run)| {
            let marker = if index == self.history_selected {
                "▶"
            } else {
                " "
            };

            let style = if index == self.history_selected {
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };

            Row::new(vec![
                format!("{marker} {}", run.source_revision),
                run.id.to_string(),
                run.finalized_at.clone(),
                run.protocol_version.clone(),
                run.engine_version.clone(),
                run.manual_version.clone(),
            ])
            .style(style)
        });

        let title = Line::from(vec![
            Span::styled(" FINALIZED RUN HISTORY · ", Style::default().fg(MINERAL)),
            Span::styled(
                history.draft_id.clone(),
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default()),
        ]);

        let widget = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(10),
                Constraint::Length(28),
                Constraint::Length(16),
                Constraint::Min(24),
                Constraint::Length(20),
            ],
        )
        .header(header)
        .column_spacing(1)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MINERAL)),
        );

        frame.render_widget(widget, area);
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let lines = if self.view == OperatorView::RunHistory {
            vec![
                Line::from(vec![
                    Span::styled("[↑/↓]", Style::default().fg(AMBER)),
                    Span::raw(" Select   "),
                    Span::styled("[H]", Style::default().fg(MINERAL)),
                    Span::raw(" Refresh   "),
                    Span::styled("[Esc]", Style::default().fg(PURPLE)),
                    Span::raw(" Sandbox   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(
                    self.message.as_str(),
                    Style::default().fg(PURPLE),
                )),
            ]
        } else if self.editing {
            vec![
                Line::from(vec![
                    Span::styled("[0-9/.]", Style::default().fg(AMBER)),
                    Span::raw(" Input   "),
                    Span::styled("[Backspace]", Style::default().fg(MUTED)),
                    Span::raw(" Delete   "),
                    Span::styled("[Enter]", Style::default().fg(MINERAL)),
                    Span::raw(" Apply   "),
                    Span::styled("[Esc]", Style::default().fg(MUTED)),
                    Span::raw(" Cancel"),
                ]),
                Line::from(Span::styled(
                    self.message.as_str(),
                    Style::default().fg(PURPLE),
                )),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled("[Tab]", Style::default().fg(AMBER)),
                    Span::raw(" Next   "),
                    Span::styled("[Shift+Tab]", Style::default().fg(PURPLE)),
                    Span::raw(" Back   "),
                    Span::styled("[[/]]", Style::default().fg(AMBER)),
                    Span::raw(" ±1%   "),
                    Span::styled("[{/}]", Style::default().fg(PURPLE)),
                    Span::raw(" ±5%   "),
                    Span::styled("[E/Enter]", Style::default().fg(MINERAL)),
                    Span::raw(" Edit number   "),
                    Span::styled("[C]", Style::default().fg(AMBER)),
                    Span::raw(" Contributor   "),
                    Span::styled("[H]", Style::default().fg(MINERAL)),
                    Span::raw(" Run history   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(
                    self.message.as_str(),
                    Style::default().fg(PURPLE),
                )),
            ]
        };

        let widget = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED)),
        );

        frame.render_widget(widget, area);
    }
}

fn slider(value: f64) -> String {
    let slots = 14usize;
    let position = (value.clamp(0.0, 1.0) * slots as f64).round() as usize;
    let mut result = String::new();

    for index in 0..=slots {
        if index == position {
            result.push('●');
        } else {
            result.push('─');
        }
    }

    result
}

fn format_money(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let cents_total = (value.abs() * 100.0).round() as u64;
    let whole = cents_total / 100;
    let cents = cents_total % 100;
    let digits = whole.to_string();
    let mut grouped = String::new();

    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }

    format!("{sign}EUR {grouped}.{cents:02}")
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| {
        let mut app = OperatorState::from_environment();
        app.run(terminal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preview_reconciles_contribution_pool() {
        let state = OperatorState::new();
        let preview = state.preview().unwrap();
        let total: f64 = preview
            .allocations
            .iter()
            .map(|allocation| allocation.allocated_amount)
            .sum();

        assert!((total - preview.contribution_pool).abs() < 1e-9);
    }

    #[test]
    fn money_input_changes_distributable_surplus() {
        let mut state = OperatorState::new();
        let before = state.preview().unwrap().surplus;

        state.ledger.revenue += 10_000.0;

        let after = state.preview().unwrap().surplus;

        assert!((after - before - 10_000.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_weight_total_blocks_preview() {
        let mut state = OperatorState::new();
        state.weights.criticality += 0.01;

        assert!(state.preview().is_err());
    }

    #[test]
    fn history_without_configured_draft_does_not_send_request() {
        let mut state = OperatorState::new();

        state.load_run_history();

        assert_eq!(state.view, OperatorView::Sandbox);
        assert!(state.run_history.is_none());
        assert!(state.message.contains("DISTRIBUTION_ENGINE_DRAFT_ID"));
    }

    #[test]
    fn history_selection_is_bounded_by_authoritative_rows() {
        use distribution_machine::operator_api::OperatorRunMetadata;

        let run = |id, revision| OperatorRunMetadata {
            id,
            source_revision: revision,
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
        };

        let mut state = OperatorState::new();

        state.run_history = Some(OperatorRunHistory {
            draft_id: "history-proof".into(),
            runs: vec![run(2, 1), run(1, 0)],
        });

        state.view = OperatorView::RunHistory;

        state.next_history_run();
        assert_eq!(state.history_selected, 1);

        state.next_history_run();
        assert_eq!(state.history_selected, 1);

        state.previous_history_run();
        assert_eq!(state.history_selected, 0);

        state.previous_history_run();
        assert_eq!(state.history_selected, 0);
    }
}
