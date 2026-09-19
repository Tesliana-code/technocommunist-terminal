use distribution_machine::{
    contribution::{ContributionFactors, ContributionWeights},
    ledger::MoneyLedger,
    persistence::{DraftStore, DraftStoreError, FinalizationMetadata},
    protocol::DistributionPools,
    run::{ContributorMeasurement, DistributionDraft},
};
use serde_json::Value;
use sqlx::{PgPool, Row, types::Json};

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
            base_work_value: 1.0,
            factors: ContributionFactors {
                responsibility: 1.0,
                criticality: 1.0,
                deal_impact: 1.0,
                stewardship: 0.0,
                dependency_reach: 1.0,
                evidence_confidence: 1.0,
            },
        }],
    }
}

fn metadata() -> FinalizationMetadata {
    FinalizationMetadata {
        protocol_version: "PP-0.1-draft".into(),
        engine_version: "distribution-machine/0.1.0".into(),
        manual_version: "manual/0.1-draft".into(),
    }
}

async fn cleanup(pool: &PgPool, id: &str) {
    let mut tx = pool.begin().await.unwrap();

    sqlx::query(
        "ALTER TABLE distribution_runs
         DISABLE TRIGGER distribution_runs_reject_delete",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::query("DELETE FROM distribution_runs WHERE source_draft_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();

    sqlx::query(
        "ALTER TABLE distribution_runs
         ENABLE TRIGGER distribution_runs_reject_delete",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn finalization_is_transactional_single_writer_and_snapshot_preserving() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL finalization test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let store = DraftStore::new(pool.clone());

    let id = "finalize-proof-001";

    cleanup(&pool, id).await;

    let original = draft(id);
    store.insert(&original).await.unwrap();

    let metadata = metadata();

    let (first, second) = tokio::join!(
        store.finalize(id, 0, &metadata),
        store.finalize(id, 0, &metadata),
    );

    let outcomes = [&first, &second];

    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);

    assert_eq!(
        outcomes
            .iter()
            .filter(|result| {
                matches!(
                    result,
                    Err(DraftStoreError::AlreadyFinalized { revision: 0, .. })
                )
            })
            .count(),
        1
    );

    let run_count: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM distribution_runs
         WHERE source_draft_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(run_count, 1);

    let row = sqlx::query(
        r#"
        SELECT
            source_revision,
            draft_snapshot,
            preview_snapshot,
            protocol_version,
            engine_version,
            manual_version
        FROM distribution_runs
        WHERE source_draft_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let source_revision: i64 = row.try_get("source_revision").unwrap();
    let Json(draft_snapshot): Json<Value> = row.try_get("draft_snapshot").unwrap();
    let Json(preview_snapshot): Json<Value> = row.try_get("preview_snapshot").unwrap();

    let protocol_version: String = row.try_get("protocol_version").unwrap();
    let engine_version: String = row.try_get("engine_version").unwrap();
    let manual_version: String = row.try_get("manual_version").unwrap();

    assert_eq!(source_revision, 0);
    assert_eq!(draft_snapshot["id"], id);
    assert_eq!(draft_snapshot["revision"], 0);
    assert_eq!(
        draft_snapshot["money"]["revenue"].as_f64().unwrap(),
        120_000.0
    );

    assert_eq!(
        preview_snapshot["available_for_distribution"]
            .as_f64()
            .unwrap(),
        30_000.0
    );

    assert_eq!(
        preview_snapshot["pool_amounts"]["contribution"]
            .as_f64()
            .unwrap(),
        10_500.0
    );

    assert_eq!(protocol_version, "PP-0.1-draft");
    assert_eq!(engine_version, "distribution-machine/0.1.0");
    assert_eq!(manual_version, "manual/0.1-draft");

    let loaded = store.load(id).await.unwrap().unwrap();

    let mut changed = loaded.clone();
    changed.money.revenue = 125_000.0;

    let updated = store.update(&changed, 0).await.unwrap();
    assert_eq!(updated.revision, 1);

    let row = sqlx::query(
        "SELECT draft_snapshot
         FROM distribution_runs
         WHERE source_draft_id = $1
           AND source_revision = 0",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let Json(old_snapshot): Json<Value> = row.try_get("draft_snapshot").unwrap();

    assert_eq!(
        old_snapshot["money"]["revenue"].as_f64().unwrap(),
        120_000.0
    );

    let stale = store.finalize(id, 0, &metadata).await.unwrap_err();

    assert!(matches!(
        stale,
        DraftStoreError::Conflict {
            expected_revision: 0,
            ..
        }
    ));

    let mut unreconciled = updated;
    unreconciled.contribution_weights.criticality = 0.31;

    let unreconciled = store.update(&unreconciled, 1).await.unwrap();
    assert_eq!(unreconciled.revision, 2);

    let rejected = store.finalize(id, 2, &metadata).await.unwrap_err();

    assert!(matches!(rejected, DraftStoreError::Validation(_)));

    let run_count: i64 = sqlx::query_scalar(
        "SELECT count(*)
         FROM distribution_runs
         WHERE source_draft_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(run_count, 1);

    cleanup(&pool, id).await;
}
