use axum::{
    Json, Router,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    contribution::ContributionWeights,
    ledger::MoneyLedger,
    persistence::{
        DraftStore, DraftStoreError, FinalizationMetadata, FinalizedAllocationExplanation,
        FinalizedContributionFactorTerms, FinalizedContributionFactors,
        FinalizedContributionWeights, FinalizedRun, FinalizedRunMetadata, FinalizedRunRecord,
        FinalizedRunTelemetry,
    },
    protocol::{DistributionPools, PoolAmounts},
    run::{ContributorMeasurement, DistributionDraft, DraftPreview},
};

#[derive(Clone)]
pub struct AppState {
    pub store: DraftStore,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/drafts", post(create_draft))
        .route("/api/drafts/{id}", get(get_draft).put(update_draft))
        .route("/api/drafts/{id}/preview", post(preview_draft))
        .route("/api/drafts/{id}/finalize", post(finalize_draft))
        .route("/api/drafts/{id}/runs", get(list_finalized_runs))
        .route("/api/runs/{id}", get(get_finalized_run))
        .route(
            "/api/runs/{run_id}/contributions/{contribution_id}/explanation",
            get(get_finalized_allocation_explanation),
        )
        .fallback(api_route_not_found)
        .method_not_allowed_fallback(api_method_not_allowed)
        .with_state(state)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateDraftRequest {
    pub id: String,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateDraftRequest {
    pub expected_revision: u64,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizeDraftRequest {
    pub expected_revision: u64,
}

#[derive(Debug, Serialize)]
pub struct FinalizedRunResponse {
    pub id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
}

impl From<FinalizedRun> for FinalizedRunResponse {
    fn from(run: FinalizedRun) -> Self {
        Self {
            id: run.id,
            source_draft_id: run.source_draft_id,
            source_revision: run.source_revision,
            protocol_version: run.protocol_version,
            engine_version: run.engine_version,
            manual_version: run.manual_version,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FinalizedRunDetailResponse {
    pub id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,
    pub draft_snapshot: Value,
    pub preview_snapshot: Value,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
    pub finalized_at: String,
}

impl From<FinalizedRunRecord> for FinalizedRunDetailResponse {
    fn from(run: FinalizedRunRecord) -> Self {
        Self {
            id: run.id,
            source_draft_id: run.source_draft_id,
            source_revision: run.source_revision,
            draft_snapshot: run.draft_snapshot,
            preview_snapshot: run.preview_snapshot,
            protocol_version: run.protocol_version,
            engine_version: run.engine_version,
            manual_version: run.manual_version,
            finalized_at: run.finalized_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FinalizedRunHistoryResponse {
    pub draft_id: String,
    pub runs: Vec<FinalizedRunMetadataResponse>,
}

#[derive(Debug, Serialize)]
pub struct FinalizedRunMetadataResponse {
    pub id: i64,
    pub source_revision: u64,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
    pub finalized_at: String,
    pub telemetry: FinalizedRunTelemetryResponse,
}

impl From<FinalizedRunMetadata> for FinalizedRunMetadataResponse {
    fn from(run: FinalizedRunMetadata) -> Self {
        Self {
            id: run.id,
            source_revision: run.source_revision,
            protocol_version: run.protocol_version,
            engine_version: run.engine_version,
            manual_version: run.manual_version,
            finalized_at: run.finalized_at,
            telemetry: run.telemetry.into(),
        }
    }
}

/// A typed summary derived strictly from a run's persisted, immutable `preview_snapshot`.
/// Never recomputed from the current draft or the current Distribution Engine.
#[derive(Debug, Serialize)]
pub struct FinalizedRunTelemetryResponse {
    pub available_for_distribution: f64,
    pub contribution_pool: f64,
    pub allocation_count: u64,
}

impl From<FinalizedRunTelemetry> for FinalizedRunTelemetryResponse {
    fn from(telemetry: FinalizedRunTelemetry) -> Self {
        Self {
            available_for_distribution: telemetry.available_for_distribution,
            contribution_pool: telemetry.contribution_pool,
            allocation_count: telemetry.allocation_count,
        }
    }
}

/// A typed "why this amount?" read model for one contribution within one finalized run,
/// combining ONLY that run's immutable `draft_snapshot` and `preview_snapshot`.
#[derive(Debug, Serialize)]
pub struct FinalizedAllocationExplanationResponse {
    pub run_id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,

    pub draft_snapshot_schema_version: String,
    pub preview_snapshot_schema_version: String,

    pub contributor_id: String,
    pub contribution_id: String,

    pub base_work_value: f64,

    pub weights: FinalizedContributionWeightsResponse,
    pub factors: FinalizedContributionFactorsResponse,
    pub factor_terms: FinalizedContributionFactorTermsResponse,

    pub weighted_factors: f64,
    pub contribution_value: f64,
    pub total_contribution_value: f64,
    pub normalized_share: f64,

    pub contribution_pool: f64,
    pub allocated_amount: f64,

    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
    pub finalized_at: String,
}

#[derive(Debug, Serialize)]
pub struct FinalizedContributionWeightsResponse {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

impl From<FinalizedContributionWeights> for FinalizedContributionWeightsResponse {
    fn from(weights: FinalizedContributionWeights) -> Self {
        Self {
            responsibility: weights.responsibility,
            criticality: weights.criticality,
            deal_impact: weights.deal_impact,
            dependency_reach: weights.dependency_reach,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FinalizedContributionFactorsResponse {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
    pub evidence_confidence: f64,
}

impl From<FinalizedContributionFactors> for FinalizedContributionFactorsResponse {
    fn from(factors: FinalizedContributionFactors) -> Self {
        Self {
            responsibility: factors.responsibility,
            criticality: factors.criticality,
            deal_impact: factors.deal_impact,
            dependency_reach: factors.dependency_reach,
            evidence_confidence: factors.evidence_confidence,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FinalizedContributionFactorTermsResponse {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

impl From<FinalizedContributionFactorTerms> for FinalizedContributionFactorTermsResponse {
    fn from(terms: FinalizedContributionFactorTerms) -> Self {
        Self {
            responsibility: terms.responsibility,
            criticality: terms.criticality,
            deal_impact: terms.deal_impact,
            dependency_reach: terms.dependency_reach,
        }
    }
}

impl From<FinalizedAllocationExplanation> for FinalizedAllocationExplanationResponse {
    fn from(explanation: FinalizedAllocationExplanation) -> Self {
        Self {
            run_id: explanation.run_id,
            source_draft_id: explanation.source_draft_id,
            source_revision: explanation.source_revision,
            draft_snapshot_schema_version: explanation.draft_snapshot_schema_version,
            preview_snapshot_schema_version: explanation.preview_snapshot_schema_version,
            contributor_id: explanation.contributor_id,
            contribution_id: explanation.contribution_id,
            base_work_value: explanation.base_work_value,
            weights: explanation.weights.into(),
            factors: explanation.factors.into(),
            factor_terms: explanation.factor_terms.into(),
            weighted_factors: explanation.weighted_factors,
            contribution_value: explanation.contribution_value,
            total_contribution_value: explanation.total_contribution_value,
            normalized_share: explanation.normalized_share,
            contribution_pool: explanation.contribution_pool,
            allocated_amount: explanation.allocated_amount,
            protocol_version: explanation.protocol_version,
            engine_version: explanation.engine_version,
            manual_version: explanation.manual_version,
            finalized_at: explanation.finalized_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DraftResponse {
    pub id: String,
    pub revision: u64,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
}

impl From<DistributionDraft> for DraftResponse {
    fn from(draft: DistributionDraft) -> Self {
        Self {
            id: draft.id,
            revision: draft.revision,
            money: draft.money,
            pools: draft.pools,
            contribution_weights: draft.contribution_weights,
            contributors: draft.contributors,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DraftPreviewResponse {
    pub id: String,
    pub revision: u64,
    pub available_for_distribution: f64,
    pub pool_amounts: PoolAmountsResponse,
    pub contribution_allocations: Vec<ContributionAllocationResponse>,
}

#[derive(Debug, Serialize)]
pub struct PoolAmountsResponse {
    pub contribution: f64,
    pub stewardship: f64,
    pub capital: f64,
    pub organizational_reserve: f64,
}

#[derive(Debug, Serialize)]
pub struct ContributionAllocationResponse {
    pub contributor_id: String,
    pub contribution_id: String,
    pub contribution_value: f64,
    pub normalized_share: f64,
    pub allocated_amount: f64,
}

impl From<PoolAmounts> for PoolAmountsResponse {
    fn from(amounts: PoolAmounts) -> Self {
        Self {
            contribution: amounts.contribution,
            stewardship: amounts.stewardship,
            capital: amounts.capital,
            organizational_reserve: amounts.organizational_reserve,
        }
    }
}

impl DraftPreviewResponse {
    fn from_domain(draft: &DistributionDraft, preview: DraftPreview) -> Self {
        Self {
            id: draft.id.clone(),
            revision: draft.revision,
            available_for_distribution: preview.available_for_distribution,
            pool_amounts: preview.pool_amounts.into(),
            contribution_allocations: preview
                .contribution_allocations
                .into_iter()
                .map(|allocation| ContributionAllocationResponse {
                    contributor_id: allocation.contributor_id,
                    contribution_id: allocation.contribution_id,
                    contribution_value: allocation.contribution_value,
                    normalized_share: allocation.normalized_share,
                    allocated_amount: allocation.allocated_amount,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

async fn api_route_not_found() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "route not found".to_string(),
        }),
    )
}

async fn api_method_not_allowed() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(ErrorResponse {
            error: "method not allowed".to_string(),
        }),
    )
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    state.store.ping().await?;

    Ok(Json(HealthResponse { status: "ok" }))
}

async fn create_draft(
    State(state): State<AppState>,
    payload: Result<Json<CreateDraftRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DraftResponse>), ApiError> {
    let request = parse_json_request(payload)?;

    let draft = DistributionDraft {
        id: request.id,
        revision: 0,
        money: request.money,
        pools: request.pools,
        contribution_weights: request.contribution_weights,
        contributors: request.contributors,
    };

    let stored = state.store.insert(&draft).await?;

    Ok((StatusCode::CREATED, Json(stored.into())))
}

async fn get_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DraftResponse>, ApiError> {
    let draft = state
        .store
        .load(&id)
        .await?
        .ok_or_else(|| DraftStoreError::NotFound(id))?;

    Ok(Json(draft.into()))
}

async fn get_finalized_run(
    State(state): State<AppState>,
    path: Result<Path<i64>, PathRejection>,
) -> Result<Json<FinalizedRunDetailResponse>, ApiError> {
    let Path(id) = path.map_err(|rejection| ApiError::Request {
        status: rejection.status(),
        error: rejection.body_text(),
    })?;

    let run = state
        .store
        .load_finalized_run(id)
        .await?
        .ok_or(DraftStoreError::RunNotFound(id))?;

    Ok(Json(run.into()))
}

async fn get_finalized_allocation_explanation(
    State(state): State<AppState>,
    path: Result<Path<(i64, String)>, PathRejection>,
) -> Result<Json<FinalizedAllocationExplanationResponse>, ApiError> {
    let Path((run_id, contribution_id)) = path.map_err(|rejection| ApiError::Request {
        status: rejection.status(),
        error: rejection.body_text(),
    })?;

    let explanation = state
        .store
        .explain_finalized_allocation(run_id, &contribution_id)
        .await?
        .ok_or(DraftStoreError::RunNotFound(run_id))?;

    Ok(Json(explanation.into()))
}

async fn list_finalized_runs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FinalizedRunHistoryResponse>, ApiError> {
    let runs = state
        .store
        .list_finalized_runs_for_draft(&id)
        .await?
        .ok_or_else(|| DraftStoreError::NotFound(id.clone()))?;

    Ok(Json(FinalizedRunHistoryResponse {
        draft_id: id,
        runs: runs.into_iter().map(Into::into).collect(),
    }))
}

async fn preview_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DraftPreviewResponse>, ApiError> {
    let draft = state
        .store
        .load(&id)
        .await?
        .ok_or_else(|| DraftStoreError::NotFound(id))?;

    let preview = draft.preview().map_err(ApiError::Preview)?;

    Ok(Json(DraftPreviewResponse::from_domain(&draft, preview)))
}

async fn finalize_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<FinalizeDraftRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<FinalizedRunResponse>), ApiError> {
    let request = parse_json_request(payload)?;

    let metadata = authoritative_finalization_metadata();

    let run = state
        .store
        .finalize(&id, request.expected_revision, &metadata)
        .await?;

    Ok((StatusCode::CREATED, Json(run.into())))
}

async fn update_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<UpdateDraftRequest>, JsonRejection>,
) -> Result<Json<DraftResponse>, ApiError> {
    let request = parse_json_request(payload)?;

    let draft = DistributionDraft {
        id,
        revision: request.expected_revision,
        money: request.money,
        pools: request.pools,
        contribution_weights: request.contribution_weights,
        contributors: request.contributors,
    };

    let updated = state
        .store
        .update(&draft, request.expected_revision)
        .await?;

    Ok(Json(updated.into()))
}

fn parse_json_request<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    let Json(request) = payload.map_err(|rejection| {
        let status = rejection.status();
        let error = rejection.body_text();

        ApiError::Request { status, error }
    })?;

    Ok(request)
}

fn authoritative_finalization_metadata() -> FinalizationMetadata {
    FinalizationMetadata {
        protocol_version: "PP-0.1-draft".to_string(),
        engine_version: concat!("distribution-machine/", env!("CARGO_PKG_VERSION")).to_string(),
        manual_version: "manual/0.1-draft".to_string(),
    }
}

enum ApiError {
    Store(DraftStoreError),
    Preview(String),
    Request { status: StatusCode, error: String },
}

impl From<DraftStoreError> for ApiError {
    fn from(error: DraftStoreError) -> Self {
        Self::Store(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::Preview(error) => (StatusCode::UNPROCESSABLE_ENTITY, error),
            Self::Request { status, error } => (status, error),

            Self::Store(error) => {
                let status = match &error {
                    DraftStoreError::Validation(_)
                    | DraftStoreError::InitialRevisionMustBeZero(_)
                    | DraftStoreError::RevisionOutOfRange(_) => StatusCode::UNPROCESSABLE_ENTITY,

                    DraftStoreError::AlreadyExists(_)
                    | DraftStoreError::AlreadyFinalized { .. }
                    | DraftStoreError::RevisionExhausted { .. }
                    | DraftStoreError::CalculationTraceUnavailable { .. }
                    | DraftStoreError::Conflict { .. } => StatusCode::CONFLICT,

                    DraftStoreError::NotFound(_)
                    | DraftStoreError::RunNotFound(_)
                    | DraftStoreError::ContributionNotFound { .. } => StatusCode::NOT_FOUND,

                    DraftStoreError::CorruptRevision(_)
                    | DraftStoreError::CorruptDraft { .. }
                    | DraftStoreError::CorruptRunSnapshot { .. }
                    | DraftStoreError::Serialization(_)
                    | DraftStoreError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
                };

                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    eprintln!("internal API error: {error}");
                    (status, "internal server error".to_string())
                } else {
                    (status, error.to_string())
                }
            }
        };

        (status, Json(ErrorResponse { error })).into_response()
    }
}
