use std::{
    env, io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use distribution_machine::{
    contribution::{ContributionEvent, ContributionFactors, ContributionWeights},
    distribution::{ContributionAllocation, allocate_contribution_pool},
    ledger::MoneyLedger,
    operator_api::{
        OperatorAllocationExplanation, OperatorApiClient, OperatorRunContributions,
        OperatorRunDetail, OperatorRunHistory,
    },
    protocol::DistributionPools,
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, LineGauge, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Sparkline, Table, Tabs, Wrap,
    },
};

// SUBNOR CRT palette. Each color carries one meaning across every view:
// AMBER    active selection / mutable operator focus / identity warmth
// MINERAL  reconciled, accepted, immutable server-owned record
// PURPLE   contributions / WHY inspection state
// COBALT   structural framing of mutable sandbox and broadcast chrome
// FAULT    refused, failed, unavailable, not reconciled
// MUTED    secondary structure and labels
// TEXT     primary phosphor text on the CRT_BACKGROUND field
const MINERAL: Color = Color::Rgb(112, 178, 134);
const PURPLE: Color = Color::Rgb(158, 138, 196);
const AMBER: Color = Color::Rgb(226, 172, 84);
const MUTED: Color = Color::Rgb(116, 128, 122);
const TEXT: Color = Color::Rgb(208, 214, 200);
const COBALT: Color = Color::Rgb(84, 116, 184);
const FAULT: Color = Color::Rgb(212, 98, 82);
const CRT_BACKGROUND: Color = Color::Rgb(11, 13, 16);

const SUBNOR_NAME: &str = "SUBNOR";
const SUBNOR_SUBTITLE: &str = "Distribution Engine Operator Terminal";
const SUBNOR_PROMPT: &str = "Press Enter to continue";
const CURSOR_BLINK_HALF_PERIOD_MS: u128 = 530;

const OPERATOR_API_BASE_URL: &str = "http://127.0.0.1:18080";

/// Width of the "share% + EUR amount" column in the allocation chart: `{:>6.2}%   {:>13}`.
const ALLOCATION_NUMBERS_WIDTH: u16 = 23;

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
    RunDetail,
    RunContributions,
    AllocationExplanation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotPane {
    Draft,
    Preview,
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
    run_detail: Option<OperatorRunDetail>,
    run_contributions: Option<OperatorRunContributions>,
    contribution_selected: usize,
    allocation_explanation: Option<OperatorAllocationExplanation>,
    snapshot_pane: SnapshotPane,
    draft_snapshot_scroll: u16,
    preview_snapshot_scroll: u16,
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
            run_detail: None,
            run_contributions: None,
            contribution_selected: 0,
            allocation_explanation: None,
            snapshot_pane: SnapshotPane::Draft,
            draft_snapshot_scroll: 0,
            preview_snapshot_scroll: 0,
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

        let client = match OperatorApiClient::new(OPERATOR_API_BASE_URL) {
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
                self.run_detail = None;
                self.run_contributions = None;
                self.contribution_selected = 0;
                self.allocation_explanation = None;
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

    fn selected_history_run_identity(&self) -> Option<(i64, String, u64)> {
        let history = self.run_history.as_ref()?;
        let run = history.runs.get(self.history_selected)?;

        Some((run.id, history.draft_id.clone(), run.source_revision))
    }

    fn accept_run_detail(
        &mut self,
        expected_run_id: i64,
        expected_draft_id: &str,
        expected_revision: u64,
        run: OperatorRunDetail,
    ) -> bool {
        if run.id != expected_run_id
            || run.source_draft_id != expected_draft_id
            || run.source_revision != expected_revision
        {
            self.message = format!(
                "RUN DETAIL IDENTITY ERROR · expected run {expected_run_id} · draft {expected_draft_id} · rev {expected_revision}"
            );
            return false;
        }

        self.message = format!(
            "IMMUTABLE RUN {} · draft {} · rev {}",
            run.id, run.source_draft_id, run.source_revision
        );
        self.run_detail = Some(run);
        self.snapshot_pane = SnapshotPane::Draft;
        self.draft_snapshot_scroll = 0;
        self.preview_snapshot_scroll = 0;
        self.view = OperatorView::RunDetail;
        true
    }

    fn open_selected_run(&mut self) {
        let Some((run_id, draft_id, revision)) = self.selected_history_run_identity() else {
            self.message = "RUN DETAIL unavailable · no finalized run selected".into();
            return;
        };

        let client = match OperatorApiClient::new(OPERATOR_API_BASE_URL) {
            Ok(client) => client,
            Err(error) => {
                self.message = format!("RUN DETAIL CONFIG ERROR · {error}");
                return;
            }
        };

        match client.get_finalized_run(run_id) {
            Ok(run) => {
                self.accept_run_detail(run_id, &draft_id, revision, run);
            }
            Err(error) => {
                self.message = format!("RUN DETAIL ERROR · {error}");
            }
        }
    }

    fn next_contribution(&mut self) {
        let count = self
            .run_contributions
            .as_ref()
            .map(|contributions| contributions.contributions.len())
            .unwrap_or(0);

        if count > 0 {
            self.contribution_selected = (self.contribution_selected + 1).min(count - 1);
        }
    }

    fn previous_contribution(&mut self) {
        self.contribution_selected = self.contribution_selected.saturating_sub(1);
    }

    fn open_selected_run_contributions(&mut self) {
        let Some((run_id, draft_id, revision)) = self.selected_history_run_identity() else {
            self.message = "CONTRIBUTIONS unavailable · no finalized run selected".into();
            return;
        };

        let client = match OperatorApiClient::new(OPERATOR_API_BASE_URL) {
            Ok(client) => client,
            Err(error) => {
                self.message = format!("CONTRIBUTIONS CONFIG ERROR · {error}");
                return;
            }
        };

        match client.list_run_contributions(run_id) {
            Ok(contributions) => {
                self.accept_run_contributions(run_id, &draft_id, revision, contributions);
            }
            Err(error) => {
                self.message = format!("CONTRIBUTIONS ERROR · {error}");
            }
        }
    }

    /// Accepts a `list_run_contributions` response only if it identifies the exact
    /// history-selected run; on mismatch, stays in `RunHistory` and retains no stale selector
    /// state from either this attempt or a previous one.
    fn accept_run_contributions(
        &mut self,
        expected_run_id: i64,
        expected_draft_id: &str,
        expected_revision: u64,
        contributions: OperatorRunContributions,
    ) -> bool {
        if contributions.run_id != expected_run_id
            || contributions.source_draft_id != expected_draft_id
            || contributions.source_revision != expected_revision
        {
            self.message = format!(
                "CONTRIBUTION SELECTOR IDENTITY ERROR · expected run {expected_run_id} · draft {expected_draft_id} · rev {expected_revision}"
            );
            self.run_contributions = None;
            self.contribution_selected = 0;
            self.allocation_explanation = None;
            return false;
        }

        let count = contributions.contributions.len();

        self.message = format!(
            "CONTRIBUTIONS · run {} · rev {} · {count} contribution(s)",
            contributions.run_id, contributions.source_revision
        );
        self.run_contributions = Some(contributions);
        self.contribution_selected = 0;
        self.allocation_explanation = None;
        self.view = OperatorView::RunContributions;
        true
    }

    fn open_selected_contribution_explanation(&mut self) {
        let Some(contributions) = self.run_contributions.as_ref() else {
            self.message = "WHY unavailable · no contribution selector loaded".into();
            return;
        };

        let Some(option) = contributions.contributions.get(self.contribution_selected) else {
            self.message = "WHY unavailable · no contribution selected".into();
            return;
        };

        let run_id = contributions.run_id;
        let draft_id = contributions.source_draft_id.clone();
        let revision = contributions.source_revision;
        let contribution_id = option.contribution_id.clone();
        let contributor_id = option.contributor_id.clone();

        let client = match OperatorApiClient::new(OPERATOR_API_BASE_URL) {
            Ok(client) => client,
            Err(error) => {
                self.message = format!("WHY CONFIG ERROR · {error}");
                return;
            }
        };

        match client.get_allocation_explanation(run_id, &contribution_id) {
            Ok(explanation) => {
                self.accept_allocation_explanation(
                    run_id,
                    &draft_id,
                    revision,
                    &contribution_id,
                    &contributor_id,
                    explanation,
                );
            }
            Err(error) => {
                self.message = format!("WHY ERROR · {error}");
            }
        }
    }

    /// Accepts a `get_allocation_explanation` response only if it identifies the exact
    /// selected run/draft/revision/contribution/contributor; on mismatch, stays in
    /// `RunContributions` and `allocation_explanation` remains `None`.
    fn accept_allocation_explanation(
        &mut self,
        expected_run_id: i64,
        expected_draft_id: &str,
        expected_revision: u64,
        expected_contribution_id: &str,
        expected_contributor_id: &str,
        explanation: OperatorAllocationExplanation,
    ) -> bool {
        if explanation.run_id != expected_run_id
            || explanation.source_draft_id != expected_draft_id
            || explanation.source_revision != expected_revision
            || explanation.contribution_id != expected_contribution_id
            || explanation.contributor_id != expected_contributor_id
        {
            self.message = format!(
                "ALLOCATION EXPLANATION IDENTITY ERROR · expected run {expected_run_id} · contribution {expected_contribution_id}"
            );
            self.allocation_explanation = None;
            return false;
        }

        self.message = format!(
            "WHY THIS AMOUNT? · {} · {}",
            explanation.contributor_id, explanation.contribution_id
        );
        self.allocation_explanation = Some(explanation);
        self.view = OperatorView::AllocationExplanation;
        true
    }

    fn snapshot_scroll_limit(&self, pane: SnapshotPane) -> u16 {
        let Some(run) = self.run_detail.as_ref() else {
            return 0;
        };

        let value = match pane {
            SnapshotPane::Draft => &run.draft_snapshot,
            SnapshotPane::Preview => &run.preview_snapshot,
        };

        let line_count = serde_json::to_string_pretty(value)
            .map(|text| text.lines().count())
            .unwrap_or(1);

        line_count.saturating_sub(1).min(u16::MAX as usize) as u16
    }

    fn scroll_run_detail(&mut self, delta: i32) {
        let limit = self.snapshot_scroll_limit(self.snapshot_pane);
        let amount = delta.unsigned_abs().min(u16::MAX as u32) as u16;

        let scroll = match self.snapshot_pane {
            SnapshotPane::Draft => &mut self.draft_snapshot_scroll,
            SnapshotPane::Preview => &mut self.preview_snapshot_scroll,
        };

        if delta < 0 {
            *scroll = scroll.saturating_sub(amount);
        } else {
            *scroll = scroll.saturating_add(amount).min(limit);
        }
    }

    fn toggle_snapshot_pane(&mut self) {
        self.snapshot_pane = match self.snapshot_pane {
            SnapshotPane::Draft => SnapshotPane::Preview,
            SnapshotPane::Preview => SnapshotPane::Draft,
        };

        self.message = match self.snapshot_pane {
            SnapshotPane::Draft => "IMMUTABLE RUN · DRAFT SNAPSHOT active".into(),
            SnapshotPane::Preview => "IMMUTABLE RUN · PREVIEW SNAPSHOT active".into(),
        };
    }

    fn handle_run_detail_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.view = OperatorView::RunHistory;
                self.message = "RUN HISTORY · immutable detail closed".into();
            }
            KeyCode::Tab | KeyCode::BackTab => self.toggle_snapshot_pane(),
            KeyCode::Up => self.scroll_run_detail(-1),
            KeyCode::Down => self.scroll_run_detail(1),
            KeyCode::PageUp => self.scroll_run_detail(-10),
            KeyCode::PageDown => self.scroll_run_detail(10),
            KeyCode::Home => match self.snapshot_pane {
                SnapshotPane::Draft => self.draft_snapshot_scroll = 0,
                SnapshotPane::Preview => self.preview_snapshot_scroll = 0,
            },
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'k') => self.scroll_run_detail(-1),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'j') => self.scroll_run_detail(1),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => self.quit = true,
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.view = OperatorView::Sandbox;
                self.message = "SANDBOX READY · deterministic preview · no writes".into();
            }
            KeyCode::Up => self.previous_history_run(),
            KeyCode::Down => self.next_history_run(),
            KeyCode::Enter => self.open_selected_run(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'k') => self.previous_history_run(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'j') => self.next_history_run(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'w') => {
                self.open_selected_run_contributions()
            }
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'h') => self.load_run_history(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => self.quit = true,
            _ => {}
        }
    }

    fn handle_run_contributions_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.view = OperatorView::RunHistory;
                self.message = "RUN HISTORY · contribution selector closed".into();
            }
            KeyCode::Up => self.previous_contribution(),
            KeyCode::Down => self.next_contribution(),
            KeyCode::Enter => self.open_selected_contribution_explanation(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'k') => self.previous_contribution(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'j') => self.next_contribution(),
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => self.quit = true,
            _ => {}
        }
    }

    fn handle_allocation_explanation_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.view = OperatorView::RunContributions;
                self.message = "CONTRIBUTIONS · allocation explanation closed".into();
            }
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
        match self.view {
            OperatorView::RunHistory => {
                self.handle_history_key(key);
                return;
            }
            OperatorView::RunDetail => {
                self.handle_run_detail_key(key);
                return;
            }
            OperatorView::RunContributions => {
                self.handle_run_contributions_key(key);
                return;
            }
            OperatorView::AllocationExplanation => {
                self.handle_allocation_explanation_key(key);
                return;
            }
            OperatorView::Sandbox => {}
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

    fn message_style(&self) -> Style {
        let message = self.message.as_str();
        if message.contains("ERROR")
            || message.contains("unavailable")
            || message.starts_with("Invalid")
        {
            Style::default().fg(FAULT)
        } else {
            Style::default().fg(PURPLE)
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(Block::default().style(crt_base_style()), frame.area());

        let root = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(20),
                Constraint::Length(4),
            ])
            .split(frame.area());

        self.draw_header(frame, root[0]);

        match self.view {
            OperatorView::RunHistory => {
                self.draw_run_history(frame, root[1]);
                self.draw_footer(frame, root[2]);
                return;
            }
            OperatorView::RunDetail => {
                self.draw_run_detail(frame, root[1]);
                self.draw_footer(frame, root[2]);
                return;
            }
            OperatorView::RunContributions => {
                self.draw_run_contributions(frame, root[1]);
                self.draw_footer(frame, root[2]);
                return;
            }
            OperatorView::AllocationExplanation => {
                self.draw_allocation_explanation(frame, root[1]);
                self.draw_footer(frame, root[2]);
                return;
            }
            OperatorView::Sandbox => {}
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
                Constraint::Min(10),
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
        let selected = match self.view {
            OperatorView::Sandbox => 0,
            OperatorView::RunHistory => 1,
            OperatorView::RunDetail => 2,
            OperatorView::RunContributions => 3,
            OperatorView::AllocationExplanation => 4,
        };

        let title = Line::from(vec![
            Span::styled(
                " SUBNOR ",
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "DISTRIBUTION-ENGINE ",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("// PP-0.1  DETERMINISTIC", Style::default().fg(PURPLE)),
            Span::styled("  NO AI / NO LLM ", Style::default().fg(MUTED)),
        ]);

        let tabs = Tabs::new(vec![
            "SANDBOX",
            "HISTORY",
            "IMMUTABLE",
            "CONTRIBUTIONS",
            "WHY",
        ])
        .select(selected)
        .style(Style::default().fg(MUTED))
        .highlight_style(Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
        .divider(" │ ")
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COBALT)),
        );

        frame.render_widget(tabs, area);
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
            Style::default().fg(FAULT).add_modifier(Modifier::BOLD)
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
        ];

        let reconciled = (pool_total - 1.0).abs() < 1e-9;

        let block = Block::default()
            .title(" MONEY + POOLS ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COBALT));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let ledger = Paragraph::new(lines).wrap(Wrap { trim: false });
        frame.render_widget(ledger, sections[0]);

        let gauge = LineGauge::default()
            .ratio(pool_total.clamp(0.0, 1.0))
            .label(format!(
                "POOL TOTAL {:>3.0}% · {}",
                pool_total * 100.0,
                if reconciled {
                    "RECONCILED"
                } else {
                    "MUST EQUAL 100%"
                }
            ))
            .filled_style(total_style)
            .unfilled_style(Style::default().fg(MUTED))
            .filled_symbol("━")
            .unfilled_symbol("─");

        frame.render_widget(gauge, sections[1]);
    }

    fn draw_weights(&self, frame: &mut Frame, area: Rect) {
        let total = self.weights.responsibility
            + self.weights.criticality
            + self.weights.deal_impact
            + self.weights.dependency_reach;

        let total_style = if (total - 1.0).abs() < 1e-9 {
            Style::default().fg(MINERAL).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FAULT).add_modifier(Modifier::BOLD)
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
                .border_style(Style::default().fg(COBALT)),
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
        let block = Block::default()
            .title(" LIVE DISTRIBUTION PREVIEW ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COBALT));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.preview() {
            Ok(preview) => self.draw_preview_chart(frame, inner, &preview),
            Err(error) => self.draw_preview_error(frame, inner, &error),
        }
    }

    fn draw_preview_error(&self, frame: &mut Frame, area: Rect, error: &str) {
        let lines = vec![
            Line::from(Span::styled(
                "PREVIEW UNAVAILABLE",
                Style::default().fg(FAULT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(error.to_string(), Style::default().fg(TEXT))),
            Line::from(""),
            Line::from(Span::styled(
                "Reconcile pool/weight totals and valid inputs to restore preview.",
                Style::default().fg(MUTED),
            )),
        ];

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    /// A valid `Preview` reconciled `allocated_amount` from `contribution_pool` at construction
    /// time (see `allocate_contribution_pool`), so this re-check only guards against float drift
    /// before the pool is trusted enough to render as a chart.
    fn draw_preview_chart(&self, frame: &mut Frame, area: Rect, preview: &Preview) {
        let selected = self.selected_contributor().id;

        let total: f64 = preview
            .allocations
            .iter()
            .map(|allocation| allocation.allocated_amount)
            .sum();
        let reconciled = (total - preview.contribution_pool).abs() < 0.005;

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(preview.allocations.len() as u16),
                Constraint::Length(1),
                Constraint::Length(3),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
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
            ])),
            sections[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "CONTRIBUTION ALLOCATION",
                Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
            ))),
            sections[2],
        );

        // Thin LineGauge geometry (name + gauge) on the left, a stable share/EUR column on the
        // right. Each gauge's ratio is the contributor's own normalized_share against the full
        // pool (0.0..=1.0) — never relative to the largest contributor — so a 38.43% share always
        // renders as ~38% of the row, not as the tallest bar in the chart.
        const ROW_PALETTE: [Color; 3] = [MINERAL, PURPLE, TEXT];

        let allocation_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                preview
                    .allocations
                    .iter()
                    .map(|_| Constraint::Length(1))
                    .collect::<Vec<_>>(),
            )
            .split(sections[3]);

        for (index, allocation) in preview.allocations.iter().enumerate() {
            let is_selected = allocation.contributor_id == selected;
            let style = if is_selected {
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ROW_PALETTE[index % ROW_PALETTE.len()])
            };

            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(12),
                    Constraint::Min(10),
                    Constraint::Length(2),
                    Constraint::Length(ALLOCATION_NUMBERS_WIDTH),
                ])
                .split(allocation_rows[index]);

            frame.render_widget(
                Paragraph::new(Line::styled(allocation.contributor_id.clone(), style)),
                columns[0],
            );

            let gauge = LineGauge::default()
                .ratio(allocation.normalized_share.clamp(0.0, 1.0))
                .label("")
                .filled_style(style)
                .unfilled_style(Style::default().fg(MUTED))
                .filled_symbol("━")
                .unfilled_symbol("─");

            frame.render_widget(gauge, columns[1]);

            frame.render_widget(
                Paragraph::new(Line::styled(
                    format!(
                        "{:>6.2}%   {:>13}",
                        allocation.normalized_share * 100.0,
                        format_money(allocation.allocated_amount),
                    ),
                    Style::default().fg(TEXT),
                )),
                columns[3],
            );
        }

        let status_style = if reconciled {
            Style::default().fg(MINERAL).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FAULT).add_modifier(Modifier::BOLD)
        };

        let footer_width = sections[5].width;
        let footer = vec![
            justified_line(
                footer_width,
                "TOTAL",
                Style::default().fg(MUTED),
                format_money(total),
                Style::default().fg(TEXT),
            ),
            justified_line(
                footer_width,
                "POOL",
                Style::default().fg(MUTED),
                format_money(preview.contribution_pool),
                Style::default().fg(TEXT),
            ),
            justified_line(
                footer_width,
                "STATUS",
                Style::default().fg(MUTED),
                if reconciled {
                    "RECONCILED".to_string()
                } else {
                    "NOT RECONCILED".to_string()
                },
                status_style,
            ),
        ];

        frame.render_widget(Paragraph::new(footer), sections[5]);
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

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(8)])
            .split(area);

        self.draw_black_box_telemetry(frame, sections[0], history);

        let area = sections[1];

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

    /// Renders the immutable telemetry black box: chronological (oldest -> newest) sparklines
    /// built from `history.runs.iter().rev()`, with the right-hand readout following whichever
    /// row is currently selected in the table below, not necessarily the newest run.
    fn draw_black_box_telemetry(
        &self,
        frame: &mut Frame,
        area: Rect,
        history: &OperatorRunHistory,
    ) {
        let (surplus_series, pool_series) = history_telemetry_series(history);
        let selected = &history.runs[self.history_selected].telemetry;

        let block = Block::default()
            .title(" BLACK BOX TELEMETRY · IMMUTABLE · OLDEST → NEWEST ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MINERAL));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        self.draw_telemetry_row(
            frame,
            rows[0],
            "SURPLUS",
            &surplus_series,
            selected.available_for_distribution,
            MINERAL,
        );

        self.draw_telemetry_row(
            frame,
            rows[1],
            "CONTRIBUTION POOL",
            &pool_series,
            selected.contribution_pool,
            PURPLE,
        );

        let selected_run = &history.runs[self.history_selected];

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("RUNS ", Style::default().fg(MUTED)),
                Span::styled(history.runs.len().to_string(), Style::default().fg(TEXT)),
                Span::styled(" · SELECTED REV ", Style::default().fg(MUTED)),
                Span::styled(
                    selected_run.source_revision.to_string(),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" · RUN ", Style::default().fg(MUTED)),
                Span::styled(selected_run.id.to_string(), Style::default().fg(TEXT)),
                Span::styled(" · ALLOCATIONS ", Style::default().fg(MUTED)),
                Span::styled(
                    selected.allocation_count.to_string(),
                    Style::default().fg(TEXT),
                ),
            ])),
            rows[2],
        );
    }

    fn draw_telemetry_row(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        series: &[u64],
        selected_amount: f64,
        accent: Color,
    ) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Min(10),
                Constraint::Length(16),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(label, Style::default().fg(MUTED)))),
            columns[0],
        );

        frame.render_widget(
            Sparkline::default()
                .data(series)
                .style(Style::default().fg(accent)),
            columns[1],
        );

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format_money(selected_amount),
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            )))
            .alignment(Alignment::Right),
            columns[2],
        );
    }

    fn draw_run_detail(&self, frame: &mut Frame, area: Rect) {
        let Some(run) = self.run_detail.as_ref() else {
            let widget = Paragraph::new(vec![
                Line::from(Span::styled(
                    "NO IMMUTABLE RUN LOADED",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Return to run history and select a finalized run.",
                    Style::default().fg(TEXT),
                )),
            ])
            .block(
                Block::default()
                    .title(" IMMUTABLE RUN ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MINERAL)),
            );

            frame.render_widget(widget, area);
            return;
        };

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(10)])
            .split(area);

        let metadata = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Run ID       ", Style::default().fg(MUTED)),
                Span::styled(
                    run.id.to_string(),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Draft        ", Style::default().fg(MUTED)),
                Span::styled(run.source_draft_id.as_str(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Revision     ", Style::default().fg(MUTED)),
                Span::styled(run.source_revision.to_string(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Finalized    ", Style::default().fg(MUTED)),
                Span::styled(run.finalized_at.as_str(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Protocol     ", Style::default().fg(MUTED)),
                Span::styled(run.protocol_version.as_str(), Style::default().fg(PURPLE)),
            ]),
            Line::from(vec![
                Span::styled("Engine       ", Style::default().fg(MUTED)),
                Span::styled(run.engine_version.as_str(), Style::default().fg(TEXT)),
            ]),
            Line::from(vec![
                Span::styled("Manual       ", Style::default().fg(MUTED)),
                Span::styled(run.manual_version.as_str(), Style::default().fg(TEXT)),
            ]),
        ])
        .block(
            Block::default()
                .title(" IMMUTABLE RUN METADATA ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(AMBER)),
        );

        frame.render_widget(metadata, sections[0]);

        let snapshots = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[1]);

        let draft_snapshot = serde_json::to_string_pretty(&run.draft_snapshot)
            .unwrap_or_else(|error| format!("snapshot rendering error: {error}"));

        let preview_snapshot = serde_json::to_string_pretty(&run.preview_snapshot)
            .unwrap_or_else(|error| format!("snapshot rendering error: {error}"));

        let draft_line_count = draft_snapshot.lines().count();
        let preview_line_count = preview_snapshot.lines().count();

        let draft_active = self.snapshot_pane == SnapshotPane::Draft;
        let preview_active = self.snapshot_pane == SnapshotPane::Preview;

        let draft_widget = Paragraph::new(draft_snapshot)
            .scroll((self.draft_snapshot_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(if draft_active {
                        " DRAFT SNAPSHOT · ACTIVE "
                    } else {
                        " DRAFT SNAPSHOT "
                    })
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if draft_active { AMBER } else { MINERAL })),
            );

        let preview_widget = Paragraph::new(preview_snapshot)
            .scroll((self.preview_snapshot_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(if preview_active {
                        " PREVIEW SNAPSHOT · ACTIVE "
                    } else {
                        " PREVIEW SNAPSHOT "
                    })
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if preview_active { AMBER } else { PURPLE })),
            );

        frame.render_widget(draft_widget, snapshots[0]);
        frame.render_widget(preview_widget, snapshots[1]);

        let scrollbar_margin = Margin {
            vertical: 1,
            horizontal: 0,
        };

        let mut draft_scrollbar_state =
            ScrollbarState::new(draft_line_count).position(self.draft_snapshot_scroll as usize);

        let draft_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .thumb_symbol("█")
            .thumb_style(Style::default().fg(if draft_active { AMBER } else { MINERAL }));

        frame.render_stateful_widget(
            draft_scrollbar,
            snapshots[0].inner(scrollbar_margin),
            &mut draft_scrollbar_state,
        );

        let mut preview_scrollbar_state =
            ScrollbarState::new(preview_line_count).position(self.preview_snapshot_scroll as usize);

        let preview_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .thumb_symbol("█")
            .thumb_style(Style::default().fg(if preview_active { AMBER } else { PURPLE }));

        frame.render_stateful_widget(
            preview_scrollbar,
            snapshots[1].inner(scrollbar_margin),
            &mut preview_scrollbar_state,
        );
    }

    /// Renders the typed contribution selector for one finalized run. Reads only
    /// `OperatorRunContributions`/`OperatorContributionOption` — never raw snapshot JSON — and
    /// performs no economic recomputation.
    fn draw_run_contributions(&self, frame: &mut Frame, area: Rect) {
        let Some(contributions) = self.run_contributions.as_ref() else {
            let widget = Paragraph::new(vec![
                Line::from(Span::styled(
                    "NO CONTRIBUTIONS LOADED",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Return to run history and press W on a finalized run.",
                    Style::default().fg(TEXT),
                )),
            ])
            .block(
                Block::default()
                    .title(" FINALIZED RUN CONTRIBUTIONS ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(PURPLE)),
            );

            frame.render_widget(widget, area);
            return;
        };

        let title = Line::from(vec![
            Span::styled(
                " FINALIZED RUN CONTRIBUTIONS · RUN ",
                Style::default().fg(MINERAL),
            ),
            Span::styled(
                contributions.run_id.to_string(),
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" · DRAFT ", Style::default().fg(MUTED)),
            Span::styled(
                contributions.source_draft_id.clone(),
                Style::default().fg(TEXT),
            ),
            Span::styled(" · REV ", Style::default().fg(MUTED)),
            Span::styled(
                contributions.source_revision.to_string(),
                Style::default().fg(TEXT),
            ),
            Span::styled(" ", Style::default()),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PURPLE));

        if contributions.contributions.is_empty() {
            let widget = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "NO CONTRIBUTIONS IN FINALIZED RUN",
                    Style::default().fg(TEXT),
                )),
            ])
            .block(block);

            frame.render_widget(widget, area);
            return;
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        const ROW_PALETTE: [Color; 3] = [MINERAL, PURPLE, TEXT];

        let header = Row::new(vec!["CONTRIBUTOR", "CONTRIBUTION ID", "SHARE", "ALLOCATED"])
            .style(Style::default().fg(PURPLE).add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let rows = contributions
            .contributions
            .iter()
            .enumerate()
            .map(|(index, option)| {
                let is_selected = index == self.contribution_selected;
                let marker = if is_selected { "▶" } else { " " };
                let style = if is_selected {
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(ROW_PALETTE[index % ROW_PALETTE.len()])
                };

                Row::new(vec![
                    format!("{marker} {}", option.contributor_id),
                    option.contribution_id.clone(),
                    format!("{:>6.2}%", option.normalized_share * 100.0),
                    format_money(option.allocated_amount),
                ])
                .style(style)
            });

        let widget = Table::new(
            rows,
            [
                Constraint::Length(18),
                Constraint::Length(28),
                Constraint::Length(10),
                Constraint::Min(16),
            ],
        )
        .header(header)
        .column_spacing(1);

        frame.render_widget(widget, inner);
    }

    /// Renders the immutable calculation trace for one contribution. Every displayed value is
    /// read directly off the already-fetched `OperatorAllocationExplanation` — this function
    /// performs no arithmetic beyond presentational formatting (percentage/money display), never
    /// calls `ContributionEvent::calculate()`, `allocate_contribution_pool()`, or
    /// `DistributionDraft::preview()`, and never touches raw snapshot JSON.
    fn draw_allocation_explanation(&self, frame: &mut Frame, area: Rect) {
        let Some(explanation) = self.allocation_explanation.as_ref() else {
            let widget = Paragraph::new(vec![
                Line::from(Span::styled(
                    "NO ALLOCATION EXPLANATION LOADED",
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Return to contributions and press Enter on a contribution.",
                    Style::default().fg(TEXT),
                )),
            ])
            .block(
                Block::default()
                    .title(" WHY THIS AMOUNT? · IMMUTABLE CALCULATION TRACE ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(PURPLE)),
            );

            frame.render_widget(widget, area);
            return;
        };

        let block = Block::default()
            .title(" WHY THIS AMOUNT? · IMMUTABLE CALCULATION TRACE ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PURPLE));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(10)])
            .split(inner);

        let identity = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("RUN ", Style::default().fg(MUTED)),
                Span::styled(explanation.run_id.to_string(), Style::default().fg(TEXT)),
                Span::styled("   DRAFT ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.source_draft_id.clone(),
                    Style::default().fg(TEXT),
                ),
                Span::styled("   REV ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.source_revision.to_string(),
                    Style::default().fg(TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled("CONTRIBUTOR ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.contributor_id.clone(),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
                Span::styled("   CONTRIBUTION ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.contribution_id.clone(),
                    Style::default().fg(TEXT),
                ),
            ]),
            Line::from(vec![
                Span::styled("FINALIZED ", Style::default().fg(MUTED)),
                Span::styled(explanation.finalized_at.clone(), Style::default().fg(TEXT)),
                Span::styled("   PROTOCOL ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.protocol_version.clone(),
                    Style::default().fg(PURPLE),
                ),
                Span::styled("   ENGINE ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.engine_version.clone(),
                    Style::default().fg(TEXT),
                ),
                Span::styled("   MANUAL ", Style::default().fg(MUTED)),
                Span::styled(
                    explanation.manual_version.clone(),
                    Style::default().fg(TEXT),
                ),
            ]),
        ]);

        frame.render_widget(identity, sections[0]);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(32)])
            .split(sections[1]);

        let mut ladder = vec![ladder_line(
            "BASE WORK VALUE",
            format!("{:.3}", explanation.base_work_value),
            Style::default().fg(TEXT),
        )];
        ladder.push(Line::from(""));

        ladder.extend(ladder_factor_lines(
            "RESPONSIBILITY",
            explanation.weights.responsibility,
            explanation.factors.responsibility,
            explanation.factor_terms.responsibility,
            MINERAL,
        ));
        ladder.extend(ladder_factor_lines(
            "CRITICALITY",
            explanation.weights.criticality,
            explanation.factors.criticality,
            explanation.factor_terms.criticality,
            PURPLE,
        ));
        ladder.extend(ladder_factor_lines(
            "DEAL IMPACT",
            explanation.weights.deal_impact,
            explanation.factors.deal_impact,
            explanation.factor_terms.deal_impact,
            MINERAL,
        ));
        ladder.extend(ladder_factor_lines(
            "DEPENDENCY REACH",
            explanation.weights.dependency_reach,
            explanation.factors.dependency_reach,
            explanation.factor_terms.dependency_reach,
            PURPLE,
        ));

        ladder.push(Line::from(""));
        ladder.push(ladder_line(
            "WEIGHTED FACTORS",
            format!("{:.3}", explanation.weighted_factors),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "EVIDENCE CONFIDENCE",
            format!("{:.3}", explanation.factors.evidence_confidence),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "CONTRIBUTION VALUE",
            format!("{:.3}", explanation.contribution_value),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "TOTAL CONTRIBUTION VALUE",
            format!("{:.3}", explanation.total_contribution_value),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "NORMALIZED SHARE",
            format!("{:.2}%", explanation.normalized_share * 100.0),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "CONTRIBUTION POOL",
            format_money(explanation.contribution_pool),
            Style::default().fg(TEXT),
        ));
        ladder.push(ladder_line(
            "ALLOCATED AMOUNT",
            format_money(explanation.allocated_amount),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ));

        frame.render_widget(Paragraph::new(ladder), columns[0]);

        let summary = vec![
            Line::from(Span::styled("ALLOCATED", Style::default().fg(MUTED))),
            Line::from(Span::styled(
                format_money(explanation.allocated_amount),
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "IMMUTABLE CALCULATION TRACE",
                Style::default().fg(MUTED),
            )),
            Line::from(""),
            Line::from(Span::styled(
                explanation.draft_snapshot_schema_version.clone(),
                Style::default().fg(MUTED),
            )),
            Line::from(Span::styled(
                explanation.preview_snapshot_schema_version.clone(),
                Style::default().fg(MUTED),
            )),
        ];

        frame.render_widget(Paragraph::new(summary), columns[1]);
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let lines = if self.view == OperatorView::RunDetail {
            vec![
                Line::from(vec![
                    Span::styled("[Tab]", Style::default().fg(AMBER)),
                    Span::raw(" Pane   "),
                    Span::styled("[J/K]", Style::default().fg(MINERAL)),
                    Span::raw(" Scroll   "),
                    Span::styled("[PgUp/PgDn]", Style::default().fg(PURPLE)),
                    Span::raw(" Page   "),
                    Span::styled("[Home]", Style::default().fg(MUTED)),
                    Span::raw(" Top   "),
                    Span::styled("[Esc]", Style::default().fg(PURPLE)),
                    Span::raw(" History   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
            ]
        } else if self.view == OperatorView::RunHistory {
            vec![
                Line::from(vec![
                    Span::styled("[J/K]", Style::default().fg(AMBER)),
                    Span::raw(" Select   "),
                    Span::styled("[Enter]", Style::default().fg(AMBER)),
                    Span::raw(" Inspect   "),
                    Span::styled("[W]", Style::default().fg(MINERAL)),
                    Span::raw(" Why   "),
                    Span::styled("[H]", Style::default().fg(MINERAL)),
                    Span::raw(" Refresh   "),
                    Span::styled("[Esc]", Style::default().fg(PURPLE)),
                    Span::raw(" Sandbox   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
            ]
        } else if self.view == OperatorView::RunContributions {
            vec![
                Line::from(vec![
                    Span::styled("[J/K]", Style::default().fg(AMBER)),
                    Span::raw(" Select   "),
                    Span::styled("[Enter]", Style::default().fg(AMBER)),
                    Span::raw(" Explain   "),
                    Span::styled("[Esc]", Style::default().fg(PURPLE)),
                    Span::raw(" History   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
            ]
        } else if self.view == OperatorView::AllocationExplanation {
            vec![
                Line::from(vec![
                    Span::styled("[Esc]", Style::default().fg(PURPLE)),
                    Span::raw(" Contributions   "),
                    Span::styled("[Q]", Style::default().fg(MUTED)),
                    Span::raw(" Quit"),
                ]),
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
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
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
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
                Line::from(Span::styled(self.message.as_str(), self.message_style())),
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

/// Top-level application phase. `Startup` is the SUBNOR transmission ident and owns all key
/// input until Enter; `Operator` delegates to the unchanged `OperatorState` views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppPhase {
    Startup,
    Operator,
}

struct App {
    phase: AppPhase,
    operator: OperatorState,
    started: Instant,
    quit: bool,
}

impl App {
    fn new(operator: OperatorState) -> Self {
        Self {
            phase: AppPhase::Startup,
            operator,
            started: Instant::now(),
            quit: false,
        }
    }

    fn should_quit(&self) -> bool {
        self.quit || self.operator.quit
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.phase {
            AppPhase::Startup => self.handle_startup_key(key),
            AppPhase::Operator => self.operator.handle_key(key),
        }
    }

    /// Only Enter, Q and Esc act; every other key is swallowed so nothing reaches the
    /// operator state behind the ident.
    fn handle_startup_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.phase = AppPhase::Operator,
            KeyCode::Esc => self.quit = true,
            KeyCode::Char(c) if c.eq_ignore_ascii_case(&'q') => self.quit = true,
            _ => {}
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit() {
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
        match self.phase {
            AppPhase::Startup => draw_startup(frame, cursor_visible(self.started.elapsed())),
            AppPhase::Operator => self.operator.draw(frame),
        }
    }
}

fn crt_base_style() -> Style {
    Style::default().fg(TEXT).bg(CRT_BACKGROUND)
}

/// The startup cursor is a pure function of time since launch; the existing 120 ms poll loop
/// redraws often enough for the phase change to show.
fn cursor_visible(elapsed: Duration) -> bool {
    (elapsed.as_millis() / CURSOR_BLINK_HALF_PERIOD_MS) % 2 == 0
}

const WORDMARK_LETTER_WIDTH: usize = 7;
const WORDMARK_HELMET_WIDTH: usize = 11;
/// S U B N [helmet] R, one column between glyphs.
const WORDMARK_WIDTH: usize = 5 * WORDMARK_LETTER_WIDTH + WORDMARK_HELMET_WIDTH + 5;

const GLYPH_S: [&str; 7] = [
    " █████ ",
    "██   ██",
    "██     ",
    " █████ ",
    "     ██",
    "██   ██",
    " █████ ",
];
const GLYPH_U: [&str; 7] = [
    "██   ██",
    "██   ██",
    "██   ██",
    "██   ██",
    "██   ██",
    "██   ██",
    " █████ ",
];
const GLYPH_B: [&str; 7] = [
    "██████ ",
    "██   ██",
    "██   ██",
    "██████ ",
    "██   ██",
    "██   ██",
    "██████ ",
];
const GLYPH_N: [&str; 7] = [
    "██   ██",
    "███  ██",
    "████ ██",
    "██ ████",
    "██  ███",
    "██   ██",
    "██   ██",
];
const GLYPH_R: [&str; 7] = [
    "██████ ",
    "██   ██",
    "██   ██",
    "██████ ",
    "██ ██  ",
    "██  ██ ",
    "██   ██",
];

/// The O of SUBNOR as a cosmonaut helmet with a cat inside, after the reference ident.
/// The shell uses the same two-column block stroke and rounded top/bottom rows as the
/// letters, so it reads as a glyph of the wordmark. Each row is (shell-left, interior,
/// shell-right); the star sits in the crown band.
const HELMET: [(&str, &str, &str); 7] = [
    (" ▄███", "*", "███▄ "),
    ("██▀", "     ", "▀██"),
    ("██ ", "/\\_/\\", " ██"),
    ("██ ", "(o.o)", " ██"),
    ("██ ", "> ^ <", " ██"),
    ("██▄", "     ", "▄██"),
    (" ▀", "███████", "▀ "),
];

/// Column of the helmet's centre within the full wordmark: S U B N plus separators.
const WORDMARK_HELMET_CENTRE: usize = 4 * (WORDMARK_LETTER_WIDTH + 1) + WORDMARK_HELMET_WIDTH / 2;

/// Control-room rule spanning exactly the wordmark, with a tick under the helmet centre.
fn wordmark_rule() -> String {
    (0..WORDMARK_WIDTH)
        .map(|column| match column {
            0 => '╺',
            c if c == WORDMARK_WIDTH - 1 => '╸',
            c if c == WORDMARK_HELMET_CENTRE => '┳',
            _ => '━',
        })
        .collect()
}

/// Prompt with a hanging cursor: the leading pad equals the cursor cell plus its gap, so the
/// text itself is centred and the cursor sits one space after it.
fn prompt_line(cursor_on: bool) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(SUBNOR_PROMPT, Style::default().fg(MUTED)),
        Span::raw(" "),
        Span::styled(
            if cursor_on { "█" } else { " " },
            Style::default().fg(AMBER),
        ),
    ])
}

fn wordmark_lines() -> Vec<Line<'static>> {
    let letter = Style::default().fg(AMBER).add_modifier(Modifier::BOLD);
    let outline = Style::default().fg(MINERAL);
    let face = Style::default().fg(TEXT).add_modifier(Modifier::BOLD);
    let star = Style::default().fg(AMBER).add_modifier(Modifier::BOLD);

    (0..7)
        .map(|row| {
            let (left, interior, right) = HELMET[row];
            let interior_style = if row == 0 { star } else { face };
            let interior_style = if row == 6 { outline } else { interior_style };
            Line::from(vec![
                Span::styled(
                    format!(
                        "{} {} {} {} ",
                        GLYPH_S[row], GLYPH_U[row], GLYPH_B[row], GLYPH_N[row]
                    ),
                    letter,
                ),
                Span::styled(left, outline),
                Span::styled(interior, interior_style),
                Span::styled(right, outline),
                Span::styled(format!(" {}", GLYPH_R[row]), letter),
            ])
        })
        .collect()
}

fn compact_wordmark_lines() -> Vec<Line<'static>> {
    let outline = Style::default().fg(MINERAL);
    vec![
        Line::from(vec![
            Span::styled("▄█", outline),
            Span::styled("*", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
            Span::styled("█▄", outline),
        ]),
        Line::from(vec![
            Span::styled("█", outline),
            Span::styled(
                "o.o",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", outline),
        ]),
        Line::from(Span::styled("▀███▀", outline)),
        Line::from(""),
        Line::from(Span::styled(
            "S  U  B  N  O  R",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )),
    ]
}

/// Startup content, degrading decoration before information: full block wordmark, then a
/// compact helmet mark, then the bare identity. Subtitle and prompt are always present.
fn startup_lines(width: u16, height: u16, cursor_on: bool) -> Vec<Line<'static>> {
    let subtitle = Line::from(Span::styled(SUBNOR_SUBTITLE, Style::default().fg(TEXT)));
    let prompt = prompt_line(cursor_on);

    let width = width as usize;
    let height = height as usize;

    // Identity group (wordmark, rule, subtitle) is held together; the prompt stands apart.
    if width >= WORDMARK_WIDTH + 2 && height >= 15 {
        let mut lines = wordmark_lines();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            wordmark_rule(),
            Style::default().fg(COBALT),
        )));
        lines.push(Line::from(""));
        lines.push(subtitle);
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(prompt);
        lines
    } else if width >= SUBNOR_SUBTITLE.len() + 2 && height >= 9 {
        let mut lines = compact_wordmark_lines();
        lines.push(Line::from(""));
        lines.push(subtitle);
        lines.push(Line::from(""));
        lines.push(prompt);
        lines
    } else {
        vec![
            Line::from(Span::styled(
                SUBNOR_NAME,
                Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
            )),
            subtitle,
            prompt,
        ]
    }
}

fn draw_startup(frame: &mut Frame, cursor_on: bool) {
    let area = frame.area();
    frame.render_widget(Block::default().style(crt_base_style()), area);

    let frame_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COBALT))
        .title(Line::from(Span::styled(
            format!(" {SUBNOR_NAME} "),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )))
        .title(
            Line::from(Span::styled(
                " DISTRIBUTION MACHINE ",
                Style::default().fg(MUTED),
            ))
            .right_aligned(),
        )
        .title_bottom(
            Line::from(Span::styled(
                " AWAITING TRANSMISSION ",
                Style::default().fg(MUTED),
            ))
            .centered(),
        );
    let inner = frame_block.inner(area);
    frame.render_widget(frame_block, area);

    let lines = startup_lines(inner.width, inner.height, cursor_on);
    let height = (lines.len() as u16).min(inner.height);
    let content = Rect {
        x: inner.x,
        y: inner.y + (inner.height - height) / 2,
        width: inner.width,
        height,
    };
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), content);
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

/// `Sparkline` bar magnitudes are `u64`. This converts an EUR amount into whole cents for that
/// purpose only; the textual EUR amount shown alongside a series is always rendered separately
/// from the original `f64` via `format_money`, never derived back from this value.
fn money_cents(value: f64) -> u64 {
    (value.max(0.0) * 100.0).round() as u64
}

/// Builds the black-box sparkline series in chronological (oldest -> newest) order from
/// `OperatorRunHistory`, whose `runs` are authoritatively newest-first. Never sorts by value
/// and never touches `draft_snapshot`/`preview_snapshot` or recomputes via the engine — the
/// series is read straight off each run's already-persisted, typed telemetry.
fn history_telemetry_series(history: &OperatorRunHistory) -> (Vec<u64>, Vec<u64>) {
    history
        .runs
        .iter()
        .rev()
        .map(|run| {
            (
                money_cents(run.telemetry.available_for_distribution),
                money_cents(run.telemetry.contribution_pool),
            )
        })
        .unzip()
}

/// Renders `left` flush against the area's left edge and `right` flush against its right edge,
/// so a value's last character always lands at the same X coordinate as the area's right border
/// regardless of terminal width.
fn justified_line(
    width: u16,
    left: &str,
    left_style: Style,
    right: String,
    right_style: Style,
) -> Line<'static> {
    let left_width = left.chars().count();
    let right_width = right.chars().count();
    let pad = (width as usize)
        .saturating_sub(left_width + right_width)
        .max(1);

    Line::from(vec![
        Span::styled(left.to_string(), left_style),
        Span::raw(" ".repeat(pad)),
        Span::styled(right, right_style),
    ])
}

/// A single "LABEL   value" row in the WHY calculation ladder. `value` is always a stored
/// `OperatorAllocationExplanation` field already formatted for display — never a freshly
/// derived economic quantity.
fn ladder_line(label: &str, value: String, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<26}"), Style::default().fg(MUTED)),
        Span::styled(value, style),
    ])
}

/// Two ladder rows for one contribution factor, placing the already-stored `weight`, `factor`,
/// and `term` side by side purely for display — no arithmetic is performed here.
fn ladder_factor_lines(
    label: &str,
    weight: f64,
    factor: f64,
    term: f64,
    accent: Color,
) -> [Line<'static>; 2] {
    [
        Line::from(Span::styled(
            label.to_string(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  WEIGHT {weight:.3}   FACTOR {factor:.3}   STORED TERM {term:.3}"),
            Style::default().fg(TEXT),
        )),
    ]
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| {
        let mut app = App::new(OperatorState::from_environment());
        app.run(terminal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_cents_rounds_to_whole_cents_and_clamps_negative() {
        assert_eq!(money_cents(4_213.554), 421_355);
        assert_eq!(money_cents(0.0), 0);
        assert_eq!(money_cents(-5.0), 0);
    }

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
        use distribution_machine::operator_api::{OperatorRunMetadata, OperatorRunTelemetry};

        let run = |id, revision| OperatorRunMetadata {
            id,
            source_revision: revision,
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
            telemetry: OperatorRunTelemetry {
                available_for_distribution: 30_000.0,
                contribution_pool: 10_500.0,
                allocation_count: 1,
            },
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

    #[test]
    fn vim_style_keys_alias_history_selection() {
        use distribution_machine::operator_api::{OperatorRunMetadata, OperatorRunTelemetry};

        let run = |id, revision| OperatorRunMetadata {
            id,
            source_revision: revision,
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
            telemetry: OperatorRunTelemetry {
                available_for_distribution: 30_000.0,
                contribution_pool: 10_500.0,
                allocation_count: 1,
            },
        };

        let mut state = OperatorState::new();

        state.run_history = Some(OperatorRunHistory {
            draft_id: "vim-keys-proof".into(),
            runs: vec![run(2, 1), run(1, 0)],
        });

        state.view = OperatorView::RunHistory;

        state.handle_history_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(state.history_selected, 1);

        state.handle_history_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));
        assert_eq!(state.history_selected, 1);

        state.handle_history_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(state.history_selected, 0);

        state.handle_history_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        assert_eq!(state.history_selected, 0);

        // Arrow keys keep working unmodified.
        state.handle_history_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(state.history_selected, 1);

        state.handle_history_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(state.history_selected, 0);
    }

    #[test]
    fn contribution_selection_is_bounded() {
        use distribution_machine::operator_api::{
            OperatorContributionOption, OperatorRunContributions,
        };

        let option = |contributor_id: &str, contribution_id: &str| OperatorContributionOption {
            contributor_id: contributor_id.into(),
            contribution_id: contribution_id.into(),
            normalized_share: 0.5,
            allocated_amount: 5_250.0,
        };

        let mut state = OperatorState::new();

        state.run_contributions = Some(OperatorRunContributions {
            run_id: 54,
            source_draft_id: "bounded-proof".into(),
            source_revision: 0,
            contributions: vec![
                option("operations", "operations-delivery"),
                option("backend", "backend-engineering"),
            ],
        });

        state.view = OperatorView::RunContributions;

        state.next_contribution();
        assert_eq!(state.contribution_selected, 1);

        state.next_contribution();
        assert_eq!(state.contribution_selected, 1);

        state.previous_contribution();
        assert_eq!(state.contribution_selected, 0);

        state.previous_contribution();
        assert_eq!(state.contribution_selected, 0);
    }

    #[test]
    fn vim_style_keys_alias_contribution_selection() {
        use distribution_machine::operator_api::{
            OperatorContributionOption, OperatorRunContributions,
        };

        let option = |contributor_id: &str, contribution_id: &str| OperatorContributionOption {
            contributor_id: contributor_id.into(),
            contribution_id: contribution_id.into(),
            normalized_share: 0.5,
            allocated_amount: 5_250.0,
        };

        let mut state = OperatorState::new();

        state.run_contributions = Some(OperatorRunContributions {
            run_id: 54,
            source_draft_id: "vim-keys-proof".into(),
            source_revision: 0,
            contributions: vec![
                option("operations", "operations-delivery"),
                option("backend", "backend-engineering"),
            ],
        });

        state.view = OperatorView::RunContributions;

        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 1);

        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 1);

        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 0);

        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 0);

        // Arrow keys keep working unmodified.
        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 1);

        state.handle_run_contributions_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(state.contribution_selected, 0);
    }

    #[test]
    fn exact_selector_identity_opens_run_contributions() {
        use distribution_machine::operator_api::{
            OperatorContributionOption, OperatorRunContributions,
        };

        let mut state = OperatorState::new();
        state.view = OperatorView::RunHistory;

        let contributions = OperatorRunContributions {
            run_id: 54,
            source_draft_id: "selector-proof".into(),
            source_revision: 0,
            contributions: vec![OperatorContributionOption {
                contributor_id: "backend".into(),
                contribution_id: "backend-engineering".into(),
                normalized_share: 1.0,
                allocated_amount: 10_500.0,
            }],
        };

        let accepted = state.accept_run_contributions(54, "selector-proof", 0, contributions);

        assert!(accepted);
        assert_eq!(state.view, OperatorView::RunContributions);
        assert_eq!(state.contribution_selected, 0);
        assert!(state.allocation_explanation.is_none());
        assert_eq!(state.run_contributions.as_ref().unwrap().run_id, 54);
    }

    #[test]
    fn mismatched_selector_identity_is_rejected() {
        use distribution_machine::operator_api::{
            OperatorContributionOption, OperatorRunContributions,
        };

        let mut state = OperatorState::new();
        state.view = OperatorView::RunHistory;

        let contributions = OperatorRunContributions {
            run_id: 54,
            source_draft_id: "selector-proof".into(),
            source_revision: 99,
            contributions: vec![OperatorContributionOption {
                contributor_id: "backend".into(),
                contribution_id: "backend-engineering".into(),
                normalized_share: 1.0,
                allocated_amount: 10_500.0,
            }],
        };

        let accepted = state.accept_run_contributions(54, "selector-proof", 0, contributions);

        assert!(!accepted);
        assert_eq!(state.view, OperatorView::RunHistory);
        assert!(state.run_contributions.is_none());
        assert!(
            state
                .message
                .contains("CONTRIBUTION SELECTOR IDENTITY ERROR")
        );
    }

    fn sample_allocation_explanation(
        contribution_id: &str,
    ) -> distribution_machine::operator_api::OperatorAllocationExplanation {
        use distribution_machine::operator_api::{
            OperatorAllocationExplanation, OperatorContributionFactorTerms,
            OperatorContributionFactors, OperatorContributionWeights,
        };

        OperatorAllocationExplanation {
            run_id: 54,
            source_draft_id: "why-proof".into(),
            source_revision: 0,
            draft_snapshot_schema_version: "distribution-draft-snapshot/1".into(),
            preview_snapshot_schema_version: "draft-preview-snapshot/1".into(),
            contributor_id: "backend".into(),
            contribution_id: contribution_id.into(),
            base_work_value: 1.0,
            weights: OperatorContributionWeights {
                responsibility: 0.20,
                criticality: 0.30,
                deal_impact: 0.25,
                dependency_reach: 0.25,
            },
            factors: OperatorContributionFactors {
                responsibility: 1.0,
                criticality: 1.0,
                deal_impact: 1.0,
                dependency_reach: 1.0,
                evidence_confidence: 1.0,
            },
            factor_terms: OperatorContributionFactorTerms {
                responsibility: 0.20,
                criticality: 0.30,
                deal_impact: 0.25,
                dependency_reach: 0.25,
            },
            weighted_factors: 1.0,
            contribution_value: 1.0,
            total_contribution_value: 1.0,
            normalized_share: 1.0,
            contribution_pool: 10_500.0,
            allocated_amount: 10_500.0,
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
        }
    }

    #[test]
    fn exact_explanation_identity_opens_allocation_explanation() {
        let mut state = OperatorState::new();
        state.view = OperatorView::RunContributions;

        let explanation = sample_allocation_explanation("backend-engineering");

        let accepted = state.accept_allocation_explanation(
            54,
            "why-proof",
            0,
            "backend-engineering",
            "backend",
            explanation,
        );

        assert!(accepted);
        assert_eq!(state.view, OperatorView::AllocationExplanation);
        assert_eq!(
            state
                .allocation_explanation
                .as_ref()
                .unwrap()
                .allocated_amount,
            10_500.0
        );
    }

    #[test]
    fn mismatched_explanation_identity_is_rejected() {
        let mut state = OperatorState::new();
        state.view = OperatorView::RunContributions;

        let explanation = sample_allocation_explanation("backend-engineering");

        let accepted = state.accept_allocation_explanation(
            54,
            "why-proof",
            0,
            "operations-delivery",
            "backend",
            explanation,
        );

        assert!(!accepted);
        assert_eq!(state.view, OperatorView::RunContributions);
        assert!(state.allocation_explanation.is_none());
        assert!(
            state
                .message
                .contains("ALLOCATION EXPLANATION IDENTITY ERROR")
        );
    }

    #[test]
    fn empty_contribution_selector_does_not_open_explanation() {
        use distribution_machine::operator_api::OperatorRunContributions;

        let mut state = OperatorState::new();

        state.run_contributions = Some(OperatorRunContributions {
            run_id: 54,
            source_draft_id: "empty-proof".into(),
            source_revision: 0,
            contributions: Vec::new(),
        });

        state.view = OperatorView::RunContributions;

        state.open_selected_contribution_explanation();

        assert_eq!(state.view, OperatorView::RunContributions);
        assert!(state.allocation_explanation.is_none());
        assert!(state.message.contains("WHY unavailable"));
    }

    #[test]
    fn history_telemetry_series_is_chronological_oldest_to_newest() {
        use distribution_machine::operator_api::{OperatorRunMetadata, OperatorRunTelemetry};

        let run = |id, revision, surplus, contribution| OperatorRunMetadata {
            id,
            source_revision: revision,
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
            telemetry: OperatorRunTelemetry {
                available_for_distribution: surplus,
                contribution_pool: contribution,
                allocation_count: 1,
            },
        };

        // OperatorRunHistory is authoritatively newest-first, matching the real API order.
        let history = OperatorRunHistory {
            draft_id: "chronology-proof".into(),
            runs: vec![
                run(48, 1, 35_000.0, 12_250.0),
                run(47, 0, 30_000.0, 10_500.0),
            ],
        };

        let (surplus_series, contribution_series) = history_telemetry_series(&history);

        assert_eq!(surplus_series, vec![3_000_000, 3_500_000]);
        assert_eq!(contribution_series, vec![1_050_000, 1_225_000]);
    }

    #[test]
    fn exact_run_detail_identity_opens_immutable_view() {
        use serde_json::json;

        let mut state = OperatorState::new();
        state.view = OperatorView::RunHistory;

        let run = OperatorRunDetail {
            id: 42,
            source_draft_id: "history-proof".into(),
            source_revision: 3,
            draft_snapshot: json!({
                "id": "history-proof",
                "revision": 3
            }),
            preview_snapshot: json!({
                "available_for_distribution": 35000.0
            }),
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
        };

        let accepted = state.accept_run_detail(42, "history-proof", 3, run);

        assert!(accepted);
        assert_eq!(state.view, OperatorView::RunDetail);
        assert_eq!(state.run_detail.as_ref().unwrap().id, 42);
    }

    #[test]
    fn snapshot_panes_scroll_independently_and_bound_position() {
        use serde_json::json;

        let mut state = OperatorState::new();

        state.run_detail = Some(OperatorRunDetail {
            id: 42,
            source_draft_id: "scroll-proof".into(),
            source_revision: 3,
            draft_snapshot: json!({
                "a": 1,
                "b": {
                    "c": 2,
                    "d": 3
                }
            }),
            preview_snapshot: json!({
                "available": 30000.0,
                "pool": {
                    "contribution": 10500.0,
                    "capital": 7500.0
                }
            }),
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 18:00:00+00".into(),
        });

        state.view = OperatorView::RunDetail;
        state.snapshot_pane = SnapshotPane::Draft;

        state.scroll_run_detail(10_000);

        let draft_limit = state.snapshot_scroll_limit(SnapshotPane::Draft);

        assert_eq!(state.draft_snapshot_scroll, draft_limit);
        assert_eq!(state.preview_snapshot_scroll, 0);

        state.toggle_snapshot_pane();
        assert_eq!(state.snapshot_pane, SnapshotPane::Preview);

        state.scroll_run_detail(2);

        assert_eq!(state.preview_snapshot_scroll, 2);
        assert_eq!(state.draft_snapshot_scroll, draft_limit);

        state.scroll_run_detail(-10_000);
        assert_eq!(state.preview_snapshot_scroll, 0);
    }

    #[test]
    fn vim_style_keys_alias_run_detail_scroll() {
        use serde_json::json;

        let mut state = OperatorState::new();

        state.run_detail = Some(OperatorRunDetail {
            id: 42,
            source_draft_id: "vim-scroll-proof".into(),
            source_revision: 3,
            draft_snapshot: json!({
                "a": 1,
                "b": {
                    "c": 2,
                    "d": 3
                }
            }),
            preview_snapshot: json!({}),
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 18:00:00+00".into(),
        });

        state.view = OperatorView::RunDetail;
        state.snapshot_pane = SnapshotPane::Draft;

        state.handle_run_detail_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 1);

        state.handle_run_detail_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 2);

        state.handle_run_detail_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 1);

        state.handle_run_detail_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 0);

        // Arrow keys keep working unmodified.
        state.handle_run_detail_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 1);

        state.handle_run_detail_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(state.draft_snapshot_scroll, 0);
    }

    #[test]
    fn mismatched_run_detail_identity_is_rejected() {
        use serde_json::json;

        let mut state = OperatorState::new();
        state.view = OperatorView::RunHistory;

        let run = OperatorRunDetail {
            id: 42,
            source_draft_id: "history-proof".into(),
            source_revision: 99,
            draft_snapshot: json!({}),
            preview_snapshot: json!({}),
            protocol_version: "PP-0.1-draft".into(),
            engine_version: "distribution-machine/0.1.0".into(),
            manual_version: "manual/0.1-draft".into(),
            finalized_at: "2026-09-19 16:00:00+00".into(),
        };

        let accepted = state.accept_run_detail(42, "history-proof", 3, run);

        assert!(!accepted);
        assert_eq!(state.view, OperatorView::RunHistory);
        assert!(state.run_detail.is_none());
        assert!(state.message.contains("RUN DETAIL IDENTITY ERROR"));
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render_startup(width: u16, height: u16, cursor_on: bool) -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw_startup(frame, cursor_on))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn app_begins_in_startup_state() {
        let app = App::new(OperatorState::new());

        assert_eq!(app.phase, AppPhase::Startup);
        assert!(!app.should_quit());
    }

    #[test]
    fn enter_leaves_startup_for_operator_sandbox_without_editing() {
        let mut app = App::new(OperatorState::new());

        app.handle_key(press(KeyCode::Enter));

        assert_eq!(app.phase, AppPhase::Operator);
        assert_eq!(app.operator.view, OperatorView::Sandbox);
        assert!(!app.operator.editing);
        assert_eq!(
            app.operator.message,
            "SANDBOX READY · deterministic preview · no writes"
        );
    }

    #[test]
    fn startup_swallows_non_action_keys() {
        let mut app = App::new(OperatorState::new());
        let before_message = app.operator.message.clone();

        for code in [
            KeyCode::Char('e'),
            KeyCode::Char('E'),
            KeyCode::Char('h'),
            KeyCode::Char('c'),
            KeyCode::Char('w'),
            KeyCode::Char(']'),
            KeyCode::Char('}'),
            KeyCode::Char('7'),
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Backspace,
        ] {
            app.handle_key(press(code));
        }

        assert_eq!(app.phase, AppPhase::Startup);
        assert!(!app.should_quit());
        assert_eq!(app.operator.view, OperatorView::Sandbox);
        assert!(!app.operator.editing);
        assert!(app.operator.edit_buffer.is_empty());
        assert_eq!(app.operator.field_index, 0);
        assert_eq!(app.operator.contributor_index, 0);
        assert!(app.operator.run_history.is_none());
        assert_eq!(app.operator.message, before_message);
        assert_eq!(app.operator.pools.contribution, 0.35);
        assert_eq!(app.operator.weights.criticality, 0.30);
    }

    #[test]
    fn quit_keys_work_from_startup() {
        for code in [KeyCode::Char('q'), KeyCode::Char('Q'), KeyCode::Esc] {
            let mut app = App::new(OperatorState::new());
            app.handle_key(press(code));

            assert!(app.should_quit());
            assert_eq!(app.phase, AppPhase::Startup);
        }
    }

    #[test]
    fn startup_identity_strings_are_exact() {
        assert_eq!(SUBNOR_NAME, "SUBNOR");
        assert_eq!(SUBNOR_SUBTITLE, "Distribution Engine Operator Terminal");
        assert_eq!(SUBNOR_PROMPT, "Press Enter to continue");

        // Full, compact and minimal layouts all carry the exact identity strings.
        for (width, height) in [(120, 36), (80, 24), (60, 16), (42, 12), (40, 8)] {
            let screen = render_startup(width, height, true);
            assert!(screen.contains(SUBNOR_NAME), "{width}x{height}");
            assert!(screen.contains(SUBNOR_SUBTITLE), "{width}x{height}");
            assert!(screen.contains(SUBNOR_PROMPT), "{width}x{height}");
        }
    }

    #[test]
    fn startup_cursor_blinks_without_blinking_the_prompt() {
        assert!(cursor_visible(Duration::ZERO));
        assert!(!cursor_visible(Duration::from_millis(
            CURSOR_BLINK_HALF_PERIOD_MS as u64
        )));
        assert!(cursor_visible(Duration::from_millis(
            2 * CURSOR_BLINK_HALF_PERIOD_MS as u64
        )));

        let on = render_startup(80, 24, true);
        let off = render_startup(80, 24, false);
        assert!(on.contains(&format!("{SUBNOR_PROMPT} █")));
        assert!(!off.contains(&format!("{SUBNOR_PROMPT} █")));
        assert!(off.contains(SUBNOR_PROMPT));
    }

    #[test]
    fn wordmark_rows_are_aligned() {
        for glyph in [GLYPH_S, GLYPH_U, GLYPH_B, GLYPH_N, GLYPH_R] {
            for row in glyph {
                assert_eq!(row.chars().count(), WORDMARK_LETTER_WIDTH);
            }
        }
        for (left, interior, right) in HELMET {
            assert_eq!(
                left.chars().count() + interior.chars().count() + right.chars().count(),
                WORDMARK_HELMET_WIDTH
            );
        }
        for line in wordmark_lines() {
            assert_eq!(line.width(), WORDMARK_WIDTH);
        }
    }

    #[test]
    fn operator_behavior_is_unchanged_after_startup() {
        let mut app = App::new(OperatorState::new());
        app.handle_key(press(KeyCode::Enter));

        app.handle_key(press(KeyCode::Down));
        assert_eq!(app.operator.field_index, 1);

        // Enter inside the sandbox keeps its existing meaning: edit a numeric field.
        app.handle_key(press(KeyCode::Enter));
        assert!(app.operator.editing);
        app.handle_key(press(KeyCode::Esc));
        assert!(!app.operator.editing);
        assert!(!app.should_quit());

        app.handle_key(press(KeyCode::Char('q')));
        assert!(app.should_quit());
    }

    #[test]
    fn fault_messages_use_fault_style() {
        let mut state = OperatorState::new();
        assert_eq!(state.message_style(), Style::default().fg(PURPLE));

        state.message = "RUN HISTORY ERROR · refused".into();
        assert_eq!(state.message_style(), Style::default().fg(FAULT));

        state.message = "WHY unavailable · no contribution selected".into();
        assert_eq!(state.message_style(), Style::default().fg(FAULT));
    }
}
