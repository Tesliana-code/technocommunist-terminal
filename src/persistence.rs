use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row, types::Json};

use crate::{
    contribution::{ContributionFactorTerms, ContributionWeights},
    distribution::ContributionAllocation,
    ledger::MoneyLedger,
    protocol::{DistributionPools, PoolAmounts},
    run::{ContributorMeasurement, DistributionDraft, DraftPreview},
};

/// Schema identity for newly finalized `draft_snapshot` documents. Existing unversioned
/// finalized runs remain valid legacy runs and are never rewritten or required to carry this.
const DISTRIBUTION_DRAFT_SNAPSHOT_SCHEMA_VERSION: &str = "distribution-draft-snapshot/1";

/// Schema identity for newly finalized `preview_snapshot` documents. Existing unversioned
/// finalized runs remain valid legacy runs and are never rewritten or required to carry this.
const DRAFT_PREVIEW_SNAPSHOT_SCHEMA_VERSION: &str = "draft-preview-snapshot/1";

#[derive(Debug)]
pub enum DraftStoreError {
    Validation(String),
    InitialRevisionMustBeZero(u64),
    AlreadyExists(String),
    AlreadyFinalized {
        id: String,
        revision: u64,
    },
    RevisionOutOfRange(u64),
    RevisionExhausted {
        id: String,
        revision: u64,
    },
    CorruptRevision(i64),
    CorruptDraft {
        id: String,
        reason: String,
    },
    CorruptRunSnapshot {
        id: i64,
        reason: String,
    },
    NotFound(String),
    RunNotFound(i64),
    ContributionNotFound {
        run_id: i64,
        contribution_id: String,
    },
    CalculationTraceUnavailable {
        id: i64,
        reason: String,
    },
    Conflict {
        id: String,
        expected_revision: u64,
    },
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
            Self::CorruptRunSnapshot { id, reason } => {
                write!(
                    f,
                    "database contains an invalid immutable snapshot for run {id}: {reason}"
                )
            }
            Self::NotFound(id) => write!(f, "draft not found: {id}"),
            Self::RunNotFound(id) => write!(f, "distribution run not found: {id}"),
            Self::ContributionNotFound {
                run_id,
                contribution_id,
            } => write!(
                f,
                "contribution {contribution_id} not found for run {run_id}"
            ),
            Self::CalculationTraceUnavailable { id, .. } => write!(
                f,
                "calculation trace unavailable for legacy finalized run: {id}"
            ),
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

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedRunMetadata {
    pub id: i64,
    pub source_revision: u64,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
    pub finalized_at: String,
    pub telemetry: FinalizedRunTelemetry,
}

/// A typed summary derived strictly from a run's persisted, immutable `preview_snapshot`.
/// Never recomputed from the current draft or the current Distribution Engine.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedRunTelemetry {
    pub available_for_distribution: f64,
    pub contribution_pool: f64,
    pub allocation_count: u64,
}

/// A typed "why this amount?" read model for one contribution within one finalized run.
/// Every field is read STRICTLY from the run's immutable `draft_snapshot`/`preview_snapshot` —
/// never from the current draft, never from `DistributionDraft::preview()`, never recomputed.
#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedAllocationExplanation {
    pub run_id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,

    pub draft_snapshot_schema_version: String,
    pub preview_snapshot_schema_version: String,

    pub contributor_id: String,
    pub contribution_id: String,

    pub base_work_value: f64,

    pub weights: FinalizedContributionWeights,
    pub factors: FinalizedContributionFactors,
    pub factor_terms: FinalizedContributionFactorTerms,

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

/// Mirrors `ContributionWeights` as read from `draft_snapshot.contribution_weights`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinalizedContributionWeights {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

/// Mirrors the matching contributor's `factors` as read from `draft_snapshot.contributors[]`.
/// Deliberately excludes `stewardship`: it exists in the historical draft input structure but
/// the contribution formula does not use it, so it has no place in a calculation explanation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinalizedContributionFactors {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
    pub evidence_confidence: f64,
}

/// Mirrors `ContributionFactorTerms` as read from the matching allocation's
/// `preview_snapshot.contribution_allocations[].factor_terms`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinalizedContributionFactorTerms {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

#[derive(Debug, Serialize)]
struct DistributionDraftSnapshot {
    schema_version: &'static str,
    id: String,
    revision: u64,

    #[serde(flatten)]
    body: DistributionDraftBody,
}

impl From<&DistributionDraft> for DistributionDraftSnapshot {
    fn from(draft: &DistributionDraft) -> Self {
        Self {
            schema_version: DISTRIBUTION_DRAFT_SNAPSHOT_SCHEMA_VERSION,
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
struct ContributionFactorTermsSnapshot {
    responsibility: f64,
    criticality: f64,
    deal_impact: f64,
    dependency_reach: f64,
}

impl From<ContributionFactorTerms> for ContributionFactorTermsSnapshot {
    fn from(terms: ContributionFactorTerms) -> Self {
        Self {
            responsibility: terms.responsibility,
            criticality: terms.criticality,
            deal_impact: terms.deal_impact,
            dependency_reach: terms.dependency_reach,
        }
    }
}

#[derive(Debug, Serialize)]
struct ContributionAllocationSnapshot {
    contributor_id: String,
    contribution_id: String,
    factor_terms: ContributionFactorTermsSnapshot,
    weighted_factors: f64,
    contribution_value: f64,
    total_contribution_value: f64,
    normalized_share: f64,
    allocated_amount: f64,
}

impl From<ContributionAllocation> for ContributionAllocationSnapshot {
    fn from(allocation: ContributionAllocation) -> Self {
        Self {
            contributor_id: allocation.contributor_id,
            contribution_id: allocation.contribution_id,
            factor_terms: allocation.factor_terms.into(),
            weighted_factors: allocation.weighted_factors,
            contribution_value: allocation.contribution_value,
            total_contribution_value: allocation.total_contribution_value,
            normalized_share: allocation.normalized_share,
            allocated_amount: allocation.allocated_amount,
        }
    }
}

#[derive(Debug, Serialize)]
struct DraftPreviewSnapshot {
    schema_version: &'static str,
    available_for_distribution: f64,
    pool_amounts: PoolAmountsSnapshot,
    contribution_allocations: Vec<ContributionAllocationSnapshot>,
}

impl From<DraftPreview> for DraftPreviewSnapshot {
    fn from(preview: DraftPreview) -> Self {
        Self {
            schema_version: DRAFT_PREVIEW_SNAPSHOT_SCHEMA_VERSION,
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

    pub async fn insert(
        &self,
        draft: &DistributionDraft,
    ) -> Result<DistributionDraft, DraftStoreError> {
        draft
            .validate_persistable()
            .map_err(DraftStoreError::Validation)?;

        if draft.revision != 0 {
            return Err(DraftStoreError::InitialRevisionMustBeZero(draft.revision));
        }

        let body = body_to_json(draft)?;

        let row = sqlx::query(
            r#"
            INSERT INTO distribution_drafts (id, body)
            VALUES ($1, $2)
            ON CONFLICT (id) DO NOTHING
            RETURNING id, revision, body
            "#,
        )
        .bind(&draft.id)
        .bind(Json(body))
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(DraftStoreError::AlreadyExists(draft.id.clone()));
        };

        let id: String = row.try_get("id")?;
        let revision: i64 = row.try_get("revision")?;
        let Json(body): Json<Value> = row.try_get("body")?;

        let revision = revision_from_db(revision)?;

        draft_from_storage(id, revision, body)
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

    /// Combines ONLY the immutable `draft_snapshot` and `preview_snapshot` already persisted
    /// for `run_id` into a typed calculation-trace explanation for `contribution_id`. Never
    /// loads the current draft and never recomputes via the Distribution Engine — reuses the
    /// same single-row `load_finalized_run` lookup and reads straight off the frozen JSON.
    pub async fn explain_finalized_allocation(
        &self,
        run_id: i64,
        contribution_id: &str,
    ) -> Result<Option<FinalizedAllocationExplanation>, DraftStoreError> {
        let Some(run) = self.load_finalized_run(run_id).await? else {
            return Ok(None);
        };

        Ok(Some(extract_allocation_explanation(&run, contribution_id)?))
    }

    pub async fn list_finalized_runs_for_draft(
        &self,
        id: &str,
    ) -> Result<Option<Vec<FinalizedRunMetadata>>, DraftStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT
                r.id AS run_id,
                r.source_revision,
                r.protocol_version,
                r.engine_version,
                r.manual_version,
                r.finalized_at::text AS finalized_at,
                r.preview_snapshot
            FROM distribution_drafts d
            LEFT JOIN distribution_runs r
                ON r.source_draft_id = d.id
            WHERE d.id = $1
            ORDER BY
                r.source_revision DESC NULLS LAST,
                r.id DESC NULLS LAST
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let mut runs = Vec::with_capacity(rows.len());

        for row in rows {
            let run_id: Option<i64> = row.try_get("run_id")?;

            let Some(run_id) = run_id else {
                continue;
            };

            let source_revision: i64 = row.try_get("source_revision")?;
            let protocol_version: String = row.try_get("protocol_version")?;
            let engine_version: String = row.try_get("engine_version")?;
            let manual_version: String = row.try_get("manual_version")?;
            let finalized_at: String = row.try_get("finalized_at")?;
            let Json(preview_snapshot): Json<Value> = row.try_get("preview_snapshot")?;

            let telemetry = extract_run_telemetry(run_id, &preview_snapshot)?;

            runs.push(FinalizedRunMetadata {
                id: run_id,
                source_revision: revision_from_db(source_revision)?,
                protocol_version,
                engine_version,
                manual_version,
                finalized_at,
                telemetry,
            });
        }

        Ok(Some(runs))
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
        let body = body_to_json(draft)?;

        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
            SELECT revision
            FROM distribution_drafts
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(&draft.id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(DraftStoreError::NotFound(draft.id.clone()));
        };

        let stored_db_revision: i64 = row.try_get("revision")?;
        let stored_revision = revision_from_db(stored_db_revision)?;

        if stored_revision != expected_revision {
            return Err(DraftStoreError::Conflict {
                id: draft.id.clone(),
                expected_revision,
            });
        }

        if stored_db_revision == i64::MAX {
            return Err(DraftStoreError::RevisionExhausted {
                id: draft.id.clone(),
                revision: stored_revision,
            });
        }

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
        .fetch_one(&mut *tx)
        .await?;

        let revision: i64 = row.try_get("revision")?;

        tx.commit().await?;

        let mut updated = draft.clone();
        updated.revision = revision_from_db(revision)?;

        Ok(updated)
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

/// Extracts telemetry strictly from a run's persisted, immutable `preview_snapshot` — never
/// from the current draft or the current Distribution Engine. Fails closed on any malformed or
/// missing field so a corrupt historical snapshot cannot silently render as zeros.
fn extract_run_telemetry(
    run_id: i64,
    preview_snapshot: &Value,
) -> Result<FinalizedRunTelemetry, DraftStoreError> {
    let corrupt = |reason: &str| DraftStoreError::CorruptRunSnapshot {
        id: run_id,
        reason: reason.to_string(),
    };

    let available_for_distribution = preview_snapshot
        .get("available_for_distribution")
        .and_then(Value::as_f64)
        .ok_or_else(|| corrupt("missing or non-numeric available_for_distribution"))?;

    if !available_for_distribution.is_finite() {
        return Err(corrupt("available_for_distribution is not finite"));
    }

    if available_for_distribution < 0.0 {
        return Err(corrupt("available_for_distribution is negative"));
    }

    let contribution_pool = preview_snapshot
        .get("pool_amounts")
        .and_then(|pool_amounts| pool_amounts.get("contribution"))
        .and_then(Value::as_f64)
        .ok_or_else(|| corrupt("missing or non-numeric pool_amounts.contribution"))?;

    if !contribution_pool.is_finite() {
        return Err(corrupt("contribution_pool is not finite"));
    }

    if contribution_pool < 0.0 {
        return Err(corrupt("contribution_pool is negative"));
    }

    let allocation_count = preview_snapshot
        .get("contribution_allocations")
        .and_then(Value::as_array)
        .ok_or_else(|| corrupt("missing or non-array contribution_allocations"))?
        .len() as u64;

    Ok(FinalizedRunTelemetry {
        available_for_distribution,
        contribution_pool,
        allocation_count,
    })
}

/// Reads a finite `f64` at a dot-separated path inside `value` (e.g. `"factors.responsibility"`),
/// failing closed with `CorruptRunSnapshot` on any missing/non-numeric/non-finite field.
fn json_f64(run_id: i64, value: &Value, path: &str) -> Result<f64, DraftStoreError> {
    let mut cursor = value;

    for part in path.split('.') {
        cursor = cursor
            .get(part)
            .ok_or_else(|| DraftStoreError::CorruptRunSnapshot {
                id: run_id,
                reason: format!("missing {path}"),
            })?;
    }

    let number = cursor
        .as_f64()
        .ok_or_else(|| DraftStoreError::CorruptRunSnapshot {
            id: run_id,
            reason: format!("non-numeric {path}"),
        })?;

    if !number.is_finite() {
        return Err(DraftStoreError::CorruptRunSnapshot {
            id: run_id,
            reason: format!("{path} is not finite"),
        });
    }

    Ok(number)
}

/// Same as [`json_f64`] but additionally fails closed if the value is negative, for fields that
/// are contractually non-negative in the frozen calculation trace.
fn json_f64_non_negative(run_id: i64, value: &Value, path: &str) -> Result<f64, DraftStoreError> {
    let number = json_f64(run_id, value, path)?;

    if number < 0.0 {
        return Err(DraftStoreError::CorruptRunSnapshot {
            id: run_id,
            reason: format!("{path} is negative"),
        });
    }

    Ok(number)
}

/// Combines ONLY the already-persisted `draft_snapshot` and `preview_snapshot` on `run` into a
/// typed calculation-trace explanation for `contribution_id`. Never loads the current draft,
/// never calls `DistributionDraft::preview()`, `ContributionEvent::calculate()`, or
/// `allocate_contribution_pool()` — every numeric value is read straight off the stored JSON
/// that IV-D1 froze at finalize time. Legacy/unversioned or unrecognized-version snapshots fail
/// closed with `CalculationTraceUnavailable` (an expected compatibility state, not corruption);
/// structurally malformed `/1` snapshots fail closed with `CorruptRunSnapshot`.
fn extract_allocation_explanation(
    run: &FinalizedRunRecord,
    contribution_id: &str,
) -> Result<FinalizedAllocationExplanation, DraftStoreError> {
    let corrupt = |reason: String| DraftStoreError::CorruptRunSnapshot { id: run.id, reason };

    let draft_schema_version = run
        .draft_snapshot
        .get("schema_version")
        .and_then(Value::as_str);
    let preview_schema_version = run
        .preview_snapshot
        .get("schema_version")
        .and_then(Value::as_str);

    if draft_schema_version != Some(DISTRIBUTION_DRAFT_SNAPSHOT_SCHEMA_VERSION)
        || preview_schema_version != Some(DRAFT_PREVIEW_SNAPSHOT_SCHEMA_VERSION)
    {
        return Err(DraftStoreError::CalculationTraceUnavailable {
            id: run.id,
            reason: "draft_snapshot/preview_snapshot missing or unrecognized schema_version"
                .to_string(),
        });
    }

    // Identity/integrity gates: a versioned snapshot must agree with its own run row.
    let draft_id = run
        .draft_snapshot
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| corrupt("draft_snapshot.id missing or non-string".to_string()))?;

    if draft_id != run.source_draft_id {
        return Err(corrupt(format!(
            "draft_snapshot.id {draft_id} does not match source_draft_id {}",
            run.source_draft_id
        )));
    }

    let draft_revision = run
        .draft_snapshot
        .get("revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| corrupt("draft_snapshot.revision missing or non-numeric".to_string()))?;

    if draft_revision != run.source_revision {
        return Err(corrupt(format!(
            "draft_snapshot.revision {draft_revision} does not match source_revision {}",
            run.source_revision
        )));
    }

    let draft_contributors = run
        .draft_snapshot
        .get("contributors")
        .and_then(Value::as_array)
        .ok_or_else(|| corrupt("draft_snapshot.contributors missing or non-array".to_string()))?;

    let matching_contributors: Vec<&Value> = draft_contributors
        .iter()
        .filter(|contributor| {
            contributor.get("contribution_id").and_then(Value::as_str) == Some(contribution_id)
        })
        .collect();

    let contributor = match matching_contributors.as_slice() {
        [] => {
            return Err(DraftStoreError::ContributionNotFound {
                run_id: run.id,
                contribution_id: contribution_id.to_string(),
            });
        }
        [only] => *only,
        multiple => {
            return Err(corrupt(format!(
                "draft_snapshot.contributors has {} entries for contribution_id {contribution_id}",
                multiple.len()
            )));
        }
    };

    let preview_allocations = run
        .preview_snapshot
        .get("contribution_allocations")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            corrupt("preview_snapshot.contribution_allocations missing or non-array".to_string())
        })?;

    let matching_allocations: Vec<&Value> = preview_allocations
        .iter()
        .filter(|allocation| {
            allocation.get("contribution_id").and_then(Value::as_str) == Some(contribution_id)
        })
        .collect();

    let allocation = match matching_allocations.as_slice() {
        [] => {
            return Err(corrupt(format!(
                "contribution_id {contribution_id} present in draft_snapshot but absent from preview_snapshot"
            )));
        }
        [only] => *only,
        multiple => {
            return Err(corrupt(format!(
                "preview_snapshot.contribution_allocations has {} entries for contribution_id {contribution_id}",
                multiple.len()
            )));
        }
    };

    let contributor_id = contributor
        .get("contributor_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            corrupt("draft contributor.contributor_id missing or non-string".to_string())
        })?;

    let allocation_contributor_id = allocation
        .get("contributor_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            corrupt("preview allocation.contributor_id missing or non-string".to_string())
        })?;

    if contributor_id != allocation_contributor_id {
        return Err(corrupt(format!(
            "draft contributor_id {contributor_id} does not match preview allocation contributor_id {allocation_contributor_id}"
        )));
    }

    let base_work_value = json_f64_non_negative(run.id, contributor, "base_work_value")?;

    let weights = FinalizedContributionWeights {
        responsibility: json_f64_non_negative(
            run.id,
            &run.draft_snapshot,
            "contribution_weights.responsibility",
        )?,
        criticality: json_f64_non_negative(
            run.id,
            &run.draft_snapshot,
            "contribution_weights.criticality",
        )?,
        deal_impact: json_f64_non_negative(
            run.id,
            &run.draft_snapshot,
            "contribution_weights.deal_impact",
        )?,
        dependency_reach: json_f64_non_negative(
            run.id,
            &run.draft_snapshot,
            "contribution_weights.dependency_reach",
        )?,
    };

    let factors = FinalizedContributionFactors {
        responsibility: json_f64_non_negative(run.id, contributor, "factors.responsibility")?,
        criticality: json_f64_non_negative(run.id, contributor, "factors.criticality")?,
        deal_impact: json_f64_non_negative(run.id, contributor, "factors.deal_impact")?,
        dependency_reach: json_f64_non_negative(run.id, contributor, "factors.dependency_reach")?,
        evidence_confidence: json_f64_non_negative(
            run.id,
            contributor,
            "factors.evidence_confidence",
        )?,
    };

    let factor_terms = FinalizedContributionFactorTerms {
        responsibility: json_f64_non_negative(run.id, allocation, "factor_terms.responsibility")?,
        criticality: json_f64_non_negative(run.id, allocation, "factor_terms.criticality")?,
        deal_impact: json_f64_non_negative(run.id, allocation, "factor_terms.deal_impact")?,
        dependency_reach: json_f64_non_negative(
            run.id,
            allocation,
            "factor_terms.dependency_reach",
        )?,
    };

    let weighted_factors = json_f64_non_negative(run.id, allocation, "weighted_factors")?;
    let contribution_value = json_f64_non_negative(run.id, allocation, "contribution_value")?;
    let total_contribution_value =
        json_f64_non_negative(run.id, allocation, "total_contribution_value")?;
    let normalized_share = json_f64(run.id, allocation, "normalized_share")?;

    if !(0.0..=1.0).contains(&normalized_share) {
        return Err(corrupt(format!(
            "normalized_share {normalized_share} is outside 0.0..=1.0"
        )));
    }

    let contribution_pool =
        json_f64_non_negative(run.id, &run.preview_snapshot, "pool_amounts.contribution")?;
    let allocated_amount = json_f64_non_negative(run.id, allocation, "allocated_amount")?;

    Ok(FinalizedAllocationExplanation {
        run_id: run.id,
        source_draft_id: run.source_draft_id.clone(),
        source_revision: run.source_revision,
        draft_snapshot_schema_version: DISTRIBUTION_DRAFT_SNAPSHOT_SCHEMA_VERSION.to_string(),
        preview_snapshot_schema_version: DRAFT_PREVIEW_SNAPSHOT_SCHEMA_VERSION.to_string(),
        contributor_id: contributor_id.to_string(),
        contribution_id: contribution_id.to_string(),
        base_work_value,
        weights,
        factors,
        factor_terms,
        weighted_factors,
        contribution_value,
        total_contribution_value,
        normalized_share,
        contribution_pool,
        allocated_amount,
        protocol_version: run.protocol_version.clone(),
        engine_version: run.engine_version.clone(),
        manual_version: run.manual_version.clone(),
        finalized_at: run.finalized_at.clone(),
    })
}
