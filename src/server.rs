use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    contribution::ContributionWeights,
    ledger::MoneyLedger,
    persistence::{DraftStore, DraftStoreError},
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
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    pub id: String,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDraftRequest {
    pub expected_revision: u64,
    pub money: MoneyLedger,
    pub pools: DistributionPools,
    pub contribution_weights: ContributionWeights,
    pub contributors: Vec<ContributorMeasurement>,
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

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    state.store.ping().await?;

    Ok(Json(HealthResponse { status: "ok" }))
}

async fn create_draft(
    State(state): State<AppState>,
    Json(request): Json<CreateDraftRequest>,
) -> Result<(StatusCode, Json<DraftResponse>), ApiError> {
    let draft = DistributionDraft {
        id: request.id,
        revision: 0,
        money: request.money,
        pools: request.pools,
        contribution_weights: request.contribution_weights,
        contributors: request.contributors,
    };

    state.store.insert(&draft).await?;

    let stored = state
        .store
        .load(&draft.id)
        .await?
        .ok_or_else(|| DraftStoreError::NotFound(draft.id.clone()))?;

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

async fn update_draft(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<UpdateDraftRequest>,
) -> Result<Json<DraftResponse>, ApiError> {
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

enum ApiError {
    Store(DraftStoreError),
    Preview(String),
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

            Self::Store(error) => {
                let status = match &error {
                    DraftStoreError::Validation(_)
                    | DraftStoreError::InitialRevisionMustBeZero(_)
                    | DraftStoreError::RevisionOutOfRange(_) => StatusCode::UNPROCESSABLE_ENTITY,

                    DraftStoreError::AlreadyExists(_)
                    | DraftStoreError::AlreadyFinalized { .. }
                    | DraftStoreError::Conflict { .. } => StatusCode::CONFLICT,

                    DraftStoreError::NotFound(_) => StatusCode::NOT_FOUND,

                    DraftStoreError::CorruptRevision(_)
                    | DraftStoreError::Serialization(_)
                    | DraftStoreError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
                };

                (status, error.to_string())
            }
        };

        (status, Json(ErrorResponse { error })).into_response()
    }
}
