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
