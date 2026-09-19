use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row, types::Json};

use crate::{
    contribution::ContributionWeights,
    distribution::ContributionAllocation,
    ledger::MoneyLedger,
    protocol::{DistributionPools, PoolAmounts},
    run::{ContributorMeasurement, DistributionDraft, DraftPreview},
};

#[derive(Debug)]
pub enum DraftStoreError {
    Validation(String),
    InitialRevisionMustBeZero(u64),
    AlreadyExists(String),
    AlreadyFinalized { id: String, revision: u64 },
    RevisionOutOfRange(u64),
    RevisionExhausted { id: String, revision: u64 },
    CorruptRevision(i64),
    CorruptDraft { id: String, reason: String },
    NotFound(String),
    RunNotFound(i64),
    Conflict { id: String, expected_revision: u64 },
    Serialization(serde_json::Error),
    Database(sqlx::Error),
}

impl fmt::Display for DraftStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "draft validation failed: {message}"),
            Self::InitialRevisionMustBeZero(revision) => {
                write!(f, "new draft revision must be 0, received {revision}")
            }
            Self::AlreadyExists(id) => write!(f, "draft already exists: {id}"),
            Self::AlreadyFinalized { id, revision } => {
                write!(f, "draft {id} revision {revision} is already finalized")
            }
            Self::RevisionOutOfRange(revision) => {
                write!(
                    f,
                    "draft revision {revision} exceeds PostgreSQL BIGINT range"
                )
            }
            Self::RevisionExhausted { id, revision } => {
                write!(
                    f,
                    "draft {id} revision {revision} cannot be incremented further"
                )
            }
            Self::CorruptRevision(revision) => {
                write!(f, "database contains invalid negative revision {revision}")
            }
            Self::CorruptDraft { id, reason } => {
                write!(f, "database contains invalid draft {id}: {reason}")
            }
            Self::NotFound(id) => write!(f, "draft not found: {id}"),
            Self::RunNotFound(id) => write!(f, "distribution run not found: {id}"),
            Self::Conflict {
                id,
                expected_revision,
            } => write!(
                f,
                "draft revision conflict for {id}: expected revision {expected_revision}"
            ),
            Self::Serialization(error) => write!(f, "draft serialization failed: {error}"),
            Self::Database(error) => write!(f, "database operation failed: {error}"),
        }
    }
}

impl Error for DraftStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
            Self::Database(error) => Some(error),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for DraftStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

impl From<sqlx::Error> for DraftStoreError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DistributionDraftBody {
    money: MoneyLedger,
    pools: DistributionPools,
    contribution_weights: ContributionWeights,
    contributors: Vec<ContributorMeasurement>,
}

impl From<&DistributionDraft> for DistributionDraftBody {
    fn from(draft: &DistributionDraft) -> Self {
        Self {
            money: draft.money,
            pools: draft.pools,
            contribution_weights: draft.contribution_weights,
            contributors: draft.contributors.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizationMetadata {
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedRun {
    pub id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedRunRecord {
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

#[derive(Debug, Serialize)]
struct DistributionDraftSnapshot {
    id: String,
    revision: u64,

    #[serde(flatten)]
    body: DistributionDraftBody,
}

impl From<&DistributionDraft> for DistributionDraftSnapshot {
    fn from(draft: &DistributionDraft) -> Self {
        Self {
            id: draft.id.clone(),
            revision: draft.revision,
            body: DistributionDraftBody::from(draft),
        }
    }
}

#[derive(Debug, Serialize)]
struct PoolAmountsSnapshot {
    contribution: f64,
    stewardship: f64,
    capital: f64,
    organizational_reserve: f64,
}

impl From<PoolAmounts> for PoolAmountsSnapshot {
    fn from(amounts: PoolAmounts) -> Self {
        Self {
            contribution: amounts.contribution,
            stewardship: amounts.stewardship,
            capital: amounts.capital,
            organizational_reserve: amounts.organizational_reserve,
        }
    }
}

#[derive(Debug, Serialize)]
struct ContributionAllocationSnapshot {
    contributor_id: String,
    contribution_id: String,
    contribution_value: f64,
    normalized_share: f64,
    allocated_amount: f64,
}

impl From<ContributionAllocation> for ContributionAllocationSnapshot {
    fn from(allocation: ContributionAllocation) -> Self {
        Self {
            contributor_id: allocation.contributor_id,
            contribution_id: allocation.contribution_id,
            contribution_value: allocation.contribution_value,
            normalized_share: allocation.normalized_share,
            allocated_amount: allocation.allocated_amount,
        }
    }
}

#[derive(Debug, Serialize)]
struct DraftPreviewSnapshot {
    available_for_distribution: f64,
    pool_amounts: PoolAmountsSnapshot,
    contribution_allocations: Vec<ContributionAllocationSnapshot>,
}

impl From<DraftPreview> for DraftPreviewSnapshot {
    fn from(preview: DraftPreview) -> Self {
        Self {
            available_for_distribution: preview.available_for_distribution,
            pool_amounts: preview.pool_amounts.into(),
            contribution_allocations: preview
                .contribution_allocations
                .into_iter()
                .map(ContributionAllocationSnapshot::from)
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct DraftStore {
    pool: PgPool,
}

impl DraftStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, draft: &DistributionDraft) -> Result<(), DraftStoreError> {
        draft
            .validate_persistable()
            .map_err(DraftStoreError::Validation)?;

        if draft.revision != 0 {
            return Err(DraftStoreError::InitialRevisionMustBeZero(draft.revision));
        }

        let body = body_to_json(draft)?;

        let result = sqlx::query(
            r#"
            INSERT INTO distribution_drafts (id, body)
            VALUES ($1, $2)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&draft.id)
        .bind(Json(body))
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DraftStoreError::AlreadyExists(draft.id.clone()));
        }

        Ok(())
    }

    pub async fn ping(&self) -> Result<(), DraftStoreError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn load(&self, id: &str) -> Result<Option<DistributionDraft>, DraftStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, revision, body
            FROM distribution_drafts
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let id: String = row.try_get("id")?;
        let revision: i64 = row.try_get("revision")?;
        let Json(body): Json<Value> = row.try_get("body")?;

        let revision = revision_from_db(revision)?;

        Ok(Some(draft_from_storage(id, revision, body)?))
    }

    pub async fn load_finalized_run(
        &self,
        id: i64,
    ) -> Result<Option<FinalizedRunRecord>, DraftStoreError> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                source_draft_id,
                source_revision,
                draft_snapshot,
                preview_snapshot,
                protocol_version,
                engine_version,
                manual_version,
                finalized_at::text AS finalized_at
            FROM distribution_runs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let run_id: i64 = row.try_get("id")?;
        let source_draft_id: String = row.try_get("source_draft_id")?;
        let source_revision: i64 = row.try_get("source_revision")?;
        let Json(draft_snapshot): Json<Value> = row.try_get("draft_snapshot")?;
        let Json(preview_snapshot): Json<Value> = row.try_get("preview_snapshot")?;
        let protocol_version: String = row.try_get("protocol_version")?;
        let engine_version: String = row.try_get("engine_version")?;
        let manual_version: String = row.try_get("manual_version")?;
        let finalized_at: String = row.try_get("finalized_at")?;

        Ok(Some(FinalizedRunRecord {
            id: run_id,
            source_draft_id,
            source_revision: revision_from_db(source_revision)?,
            draft_snapshot,
            preview_snapshot,
            protocol_version,
            engine_version,
            manual_version,
            finalized_at,
        }))
    }

    pub async fn finalize(
        &self,
        id: &str,
        expected_revision: u64,
        metadata: &FinalizationMetadata,
    ) -> Result<FinalizedRun, DraftStoreError> {
        validate_finalization_metadata(metadata)?;

        let expected_db_revision = revision_to_db(expected_revision)?;
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
            SELECT id, revision, body
            FROM distribution_drafts
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(DraftStoreError::NotFound(id.to_string()));
        };

        let stored_id: String = row.try_get("id")?;
        let stored_revision: i64 = row.try_get("revision")?;
        let Json(body): Json<Value> = row.try_get("body")?;

        let revision = revision_from_db(stored_revision)?;

        if revision != expected_revision {
            return Err(DraftStoreError::Conflict {
                id: stored_id,
                expected_revision,
            });
        }

        let draft = draft_from_storage(stored_id, revision, body)?;

        let preview = draft.preview().map_err(DraftStoreError::Validation)?;

        let draft_snapshot = serde_json::to_value(DistributionDraftSnapshot::from(&draft))?;

        let preview_snapshot = serde_json::to_value(DraftPreviewSnapshot::from(preview))?;

        let run_id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO distribution_runs (
                source_draft_id,
                source_revision,
                draft_snapshot,
                preview_snapshot,
                protocol_version,
                engine_version,
                manual_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (source_draft_id, source_revision) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(&draft.id)
        .bind(expected_db_revision)
        .bind(Json(draft_snapshot))
        .bind(Json(preview_snapshot))
        .bind(&metadata.protocol_version)
        .bind(&metadata.engine_version)
        .bind(&metadata.manual_version)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(run_id) = run_id else {
            return Err(DraftStoreError::AlreadyFinalized {
                id: draft.id,
                revision: draft.revision,
            });
        };

        tx.commit().await?;

        Ok(FinalizedRun {
            id: run_id,
            source_draft_id: draft.id,
            source_revision: draft.revision,
            protocol_version: metadata.protocol_version.clone(),
            engine_version: metadata.engine_version.clone(),
            manual_version: metadata.manual_version.clone(),
        })
    }

    pub async fn update(
        &self,
        draft: &DistributionDraft,
        expected_revision: u64,
    ) -> Result<DistributionDraft, DraftStoreError> {
        draft
            .validate_persistable()
            .map_err(DraftStoreError::Validation)?;

        draft
            .ensure_revision(expected_revision)
            .map_err(DraftStoreError::Validation)?;

        let expected_db_revision = revision_to_db(expected_revision)?;

        if expected_db_revision == i64::MAX {
            return Err(DraftStoreError::RevisionExhausted {
                id: draft.id.clone(),
                revision: expected_revision,
            });
        }

        let body = body_to_json(draft)?;

        let row = sqlx::query(
            r#"
            UPDATE distribution_drafts
            SET body = $1,
                revision = revision + 1,
                updated_at = now()
            WHERE id = $2
              AND revision = $3
            RETURNING revision
            "#,
        )
        .bind(Json(body))
        .bind(&draft.id)
        .bind(expected_db_revision)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let revision: i64 = row.try_get("revision")?;
            let mut updated = draft.clone();
            updated.revision = revision_from_db(revision)?;
            return Ok(updated);
        }

        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM distribution_drafts
                WHERE id = $1
            )
            "#,
        )
        .bind(&draft.id)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            Err(DraftStoreError::Conflict {
                id: draft.id.clone(),
                expected_revision,
            })
        } else {
            Err(DraftStoreError::NotFound(draft.id.clone()))
        }
    }
}

fn draft_from_storage(
    id: String,
    revision: u64,
    body: Value,
) -> Result<DistributionDraft, DraftStoreError> {
    let body: DistributionDraftBody = serde_json::from_value(body)?;

    let draft = DistributionDraft {
        id,
        revision,
        money: body.money,
        pools: body.pools,
        contribution_weights: body.contribution_weights,
        contributors: body.contributors,
    };

    draft
        .validate_persistable()
        .map_err(|reason| DraftStoreError::CorruptDraft {
            id: draft.id.clone(),
            reason,
        })?;

    Ok(draft)
}

fn validate_finalization_metadata(metadata: &FinalizationMetadata) -> Result<(), DraftStoreError> {
    for (label, value) in [
        ("protocol_version", metadata.protocol_version.as_str()),
        ("engine_version", metadata.engine_version.as_str()),
        ("manual_version", metadata.manual_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(DraftStoreError::Validation(format!(
                "{label} cannot be empty"
            )));
        }
    }

    Ok(())
}

fn body_to_json(draft: &DistributionDraft) -> Result<Value, DraftStoreError> {
    let body = DistributionDraftBody::from(draft);
    Ok(serde_json::to_value(body)?)
}

fn revision_to_db(revision: u64) -> Result<i64, DraftStoreError> {
    i64::try_from(revision).map_err(|_| DraftStoreError::RevisionOutOfRange(revision))
}

fn revision_from_db(revision: i64) -> Result<u64, DraftStoreError> {
    u64::try_from(revision).map_err(|_| DraftStoreError::CorruptRevision(revision))
}
