use distribution_machine::{
    contribution::{ContributionFactors, ContributionWeights},
    ledger::MoneyLedger,
    persistence::{DraftStore, DraftStoreError, FinalizationMetadata},
    protocol::DistributionPools,
    run::{ContributorMeasurement, DistributionDraft},
};
use sqlx::PgPool;

fn draft(id: &str) -> DistributionDraft {
    DistributionDraft {
        id: id.into(),
        revision: 0,
        money: MoneyLedger {
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
        contribution_weights: ContributionWeights {
            responsibility: 0.20,
            criticality: 0.30,
            deal_impact: 0.25,
            dependency_reach: 0.25,
        },
        contributors: vec![ContributorMeasurement {
            contributor_id: "backend".into(),
            contribution_id: "backend-engineering".into(),
            base_work_value: 1.2,
            factors: ContributionFactors {
                responsibility: 0.80,
                criticality: 0.90,
                deal_impact: 0.85,
                stewardship: 0.0,
                dependency_reach: 0.75,
                evidence_confidence: 1.0,
            },
        }],
    }
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn save_load_update_and_stale_revision_are_enforced() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-roundtrip";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let original = draft(id);

    store.insert(&original).await.unwrap();

    let loaded = store.load(id).await.unwrap().unwrap();
    assert_eq!(loaded, original);

    let mut changed = loaded.clone();
    changed.money.revenue = 125_000.0;

    let updated = store.update(&changed, 0).await.unwrap();

    assert_eq!(updated.revision, 1);
    assert_eq!(updated.money.revenue, 125_000.0);

    let stored = store.load(id).await.unwrap().unwrap();
    assert_eq!(stored, updated);

    let stale = store.update(&changed, 0).await.unwrap_err();

    assert!(matches!(
        stale,
        DraftStoreError::Conflict {
            expected_revision: 0,
            ..
        }
    ));

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn insert_requires_server_owned_initial_revision_zero() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool);

    let mut invalid = draft("test-store-nonzero-initial-revision");
    invalid.revision = 7;

    let error = store.insert(&invalid).await.unwrap_err();

    assert!(matches!(
        error,
        DraftStoreError::InitialRevisionMustBeZero(7)
    ));
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn corrupt_database_draft_is_rejected_on_read_and_finalize() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-corrupt-domain-draft";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        r#"
        INSERT INTO distribution_drafts (id, revision, body)
        VALUES (
            $1,
            0,
            '{
                "money": {
                    "revenue": -1.0,
                    "direct_costs": 0.0,
                    "shared_costs": 0.0,
                    "labor": 0.0,
                    "reserve_reinvestment": 0.0
                },
                "pools": {
                    "contribution": 0.35,
                    "stewardship": 0.20,
                    "capital": 0.25,
                    "organizational_reserve": 0.20
                },
                "contribution_weights": {
                    "responsibility": 0.20,
                    "criticality": 0.30,
                    "deal_impact": 0.25,
                    "dependency_reach": 0.25
                },
                "contributors": []
            }'::jsonb
        )
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();

    let load_error = store.load(id).await.unwrap_err();

    assert!(matches!(
        load_error,
        DraftStoreError::CorruptDraft {
            ref id,
            ..
        } if id == "test-store-corrupt-domain-draft"
    ));

    let metadata = FinalizationMetadata {
        protocol_version: "PP-0.1-draft".into(),
        engine_version: "distribution-machine/0.1.0".into(),
        manual_version: "manual/0.1-draft".into(),
    };

    let finalize_error = store.finalize(id, 0, &metadata).await.unwrap_err();

    assert!(matches!(
        finalize_error,
        DraftStoreError::CorruptDraft {
            ref id,
            ..
        } if id == "test-store-corrupt-domain-draft"
    ));

    let run_count: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM distribution_runs
         WHERE source_draft_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(run_count, 0);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn maximum_database_revision_is_explicitly_exhausted_without_mutation() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-revision-exhausted";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let original = draft(id);
    store.insert(&original).await.unwrap();

    sqlx::query(
        "UPDATE distribution_drafts
         SET revision = $2
         WHERE id = $1",
    )
    .bind(id)
    .bind(i64::MAX)
    .execute(&pool)
    .await
    .unwrap();

    let loaded = store.load(id).await.unwrap().unwrap();

    assert_eq!(loaded.revision, i64::MAX as u64);

    let mut changed = loaded.clone();
    changed.money.revenue = 999_999.0;

    let error = store.update(&changed, i64::MAX as u64).await.unwrap_err();

    assert!(matches!(
        error,
        DraftStoreError::RevisionExhausted {
            ref id,
            revision
        } if id == "test-store-revision-exhausted"
            && revision == i64::MAX as u64
    ));

    let row: (i64, serde_json::Value) = sqlx::query_as(
        "SELECT revision, body
         FROM distribution_drafts
         WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, i64::MAX);

    assert_eq!(row.1["money"]["revenue"].as_f64().unwrap(), 120_000.0);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn concurrent_updates_are_serialized_by_revision_lock() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-concurrent-update-lock";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let original = draft(id);
    store.insert(&original).await.unwrap();

    let mut first = original.clone();
    first.money.revenue = 125_000.0;

    let mut second = original.clone();
    second.money.revenue = 130_000.0;

    let (first_result, second_result) =
        tokio::join!(store.update(&first, 0), store.update(&second, 0),);

    let outcomes = [&first_result, &second_result];

    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);

    assert_eq!(
        outcomes
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(DraftStoreError::Conflict {
                        expected_revision: 0,
                        ..
                    })
                )
            })
            .count(),
        1
    );

    let stored = store.load(id).await.unwrap().unwrap();

    assert_eq!(stored.revision, 1);
    assert!(stored.money.revenue == 125_000.0 || stored.money.revenue == 130_000.0);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn missing_draft_at_max_revision_is_not_misclassified_as_exhausted() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-missing-max-revision";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let mut missing = draft(id);
    missing.revision = i64::MAX as u64;

    let error = store.update(&missing, i64::MAX as u64).await.unwrap_err();

    assert!(matches!(
        error,
        DraftStoreError::NotFound(ref missing_id)
            if missing_id == "test-store-missing-max-revision"
    ));
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn insert_returns_authoritative_persisted_draft() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-insert-returning";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let original = draft(id);

    let inserted = store.insert(&original).await.unwrap();

    assert_eq!(inserted, original);
    assert_eq!(inserted.revision, 0);

    let stored = store.load(id).await.unwrap().unwrap();
    assert_eq!(stored, inserted);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn concurrent_inserts_have_one_authoritative_winner() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL persistence test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "test-store-concurrent-insert";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let candidate = draft(id);

    let (first, second) = tokio::join!(store.insert(&candidate), store.insert(&candidate),);

    let outcomes = [&first, &second];

    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);

    assert_eq!(
        outcomes
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(DraftStoreError::AlreadyExists(existing_id))
                        if existing_id == id
                )
            })
            .count(),
        1
    );

    let winner = outcomes
        .iter()
        .find_map(|result| result.as_ref().ok())
        .unwrap();

    assert_eq!(winner, &candidate);

    let stored = store.load(id).await.unwrap().unwrap();
    assert_eq!(stored, candidate);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}
