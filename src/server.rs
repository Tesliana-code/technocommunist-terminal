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
    protocol::DistributionPools,
    run::{ContributorMeasurement, DistributionDraft},
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

struct ApiError(DraftStoreError);

impl From<DraftStoreError> for ApiError {
    fn from(error: DraftStoreError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            DraftStoreError::Validation(_)
            | DraftStoreError::InitialRevisionMustBeZero(_)
            | DraftStoreError::RevisionOutOfRange(_) => StatusCode::UNPROCESSABLE_ENTITY,

            DraftStoreError::AlreadyExists(_) | DraftStoreError::Conflict { .. } => {
                StatusCode::CONFLICT
            }

            DraftStoreError::NotFound(_) => StatusCode::NOT_FOUND,

            DraftStoreError::CorruptRevision(_)
            | DraftStoreError::Serialization(_)
            | DraftStoreError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            Json(ErrorResponse {
                error: self.0.to_string(),
            }),
        )
            .into_response()
    }
}
