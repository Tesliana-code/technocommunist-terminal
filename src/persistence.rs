use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row, types::Json};

use crate::{
    contribution::ContributionWeights,
    ledger::MoneyLedger,
    protocol::DistributionPools,
    run::{ContributorMeasurement, DistributionDraft},
};

#[derive(Debug)]
pub enum DraftStoreError {
    Validation(String),
    InitialRevisionMustBeZero(u64),
    RevisionOutOfRange(u64),
    CorruptRevision(i64),
    NotFound(String),
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
            Self::RevisionOutOfRange(revision) => {
                write!(
                    f,
                    "draft revision {revision} exceeds PostgreSQL BIGINT range"
                )
            }
            Self::CorruptRevision(revision) => {
                write!(f, "database contains invalid negative revision {revision}")
            }
            Self::NotFound(id) => write!(f, "draft not found: {id}"),
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

        sqlx::query(
            r#"
            INSERT INTO distribution_drafts (id, body)
            VALUES ($1, $2)
            "#,
        )
        .bind(&draft.id)
        .bind(Json(body))
        .execute(&self.pool)
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

        let body: DistributionDraftBody = serde_json::from_value(body)?;

        Ok(Some(DistributionDraft {
            id,
            revision: revision_from_db(revision)?,
            money: body.money,
            pools: body.pools,
            contribution_weights: body.contribution_weights,
            contributors: body.contributors,
        }))
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
