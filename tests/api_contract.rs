use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        HeaderMap, Method, Request, StatusCode,
        header::{ALLOW, CONTENT_TYPE},
    },
    response::Response,
};
use distribution_machine::{
    persistence::DraftStore,
    server::{AppState, router},
};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

fn lazy_app() -> Router {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://test:test@127.0.0.1:1/distribution_contract_test")
        .unwrap();

    router(AppState {
        store: DraftStore::new(pool),
    })
}

fn app_with_pool(pool: PgPool) -> Router {
    router(AppState {
        store: DraftStore::new(pool),
    })
}

fn draft_payload(id: &str) -> Value {
    json!({
        "id": id,
        "money": {
            "revenue": 120000.0,
            "direct_costs": 20000.0,
            "shared_costs": 10000.0,
            "labor": 40000.0,
            "reserve_reinvestment": 20000.0
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
    })
}

fn update_payload(expected_revision: u64) -> Value {
    json!({
        "expected_revision": expected_revision,
        "money": {
            "revenue": 125000.0,
            "direct_costs": 20000.0,
            "shared_costs": 10000.0,
            "labor": 40000.0,
            "reserve_reinvestment": 20000.0
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
    })
}

fn request(
    method: Method,
    uri: &str,
    body: impl Into<Body>,
    json_content_type: bool,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    if json_content_type {
        builder = builder.header(CONTENT_TYPE, "application/json");
    }

    builder.body(body.into()).unwrap()
}

async fn call(app: &Router, request: Request<Body>) -> Response {
    app.clone().oneshot(request).await.unwrap()
}

fn assert_json_content_type(headers: &HeaderMap) {
    let content_type = headers
        .get(CONTENT_TYPE)
        .expect("response must include content-type")
        .to_str()
        .unwrap();

    assert!(
        content_type.starts_with("application/json"),
        "expected application/json, got {content_type}"
    );
}

async fn json_response(response: Response, expected_status: StatusCode) -> Value {
    assert_eq!(response.status(), expected_status);
    assert_json_content_type(response.headers());

    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();

    serde_json::from_slice(&bytes).unwrap()
}

async fn error_response(response: Response, expected_status: StatusCode) -> Value {
    let body = json_response(response, expected_status).await;

    assert!(
        body.get("error").and_then(Value::as_str).is_some(),
        "error response must contain string .error: {body}"
    );

    body
}

#[tokio::test]
async fn json_request_rejections_preserve_status_and_closed_contracts() {
    let app = lazy_app();

    let response = call(&app, request(Method::POST, "/api/drafts", "{}", false)).await;

    error_response(response, StatusCode::UNSUPPORTED_MEDIA_TYPE).await;

    let response = call(
        &app,
        request(Method::POST, "/api/drafts", r#"{"id":"#, true),
    )
    .await;

    error_response(response, StatusCode::BAD_REQUEST).await;

    let response = call(&app, request(Method::POST, "/api/drafts", "{}", true)).await;

    error_response(response, StatusCode::UNPROCESSABLE_ENTITY).await;

    let mut create = draft_payload("api-contract-create-unknown-field");
    create["revision"] = json!(99);

    let response = call(
        &app,
        request(
            Method::POST,
            "/api/drafts",
            serde_json::to_vec(&create).unwrap(),
            true,
        ),
    )
    .await;

    let body = error_response(response, StatusCode::UNPROCESSABLE_ENTITY).await;
    let error = body["error"].as_str().unwrap();

    assert!(error.contains("unknown field"));
    assert!(error.contains("revision"));

    let mut update = update_payload(0);
    update["id"] = json!("client-attempts-to-own-id");

    let response = call(
        &app,
        request(
            Method::PUT,
            "/api/drafts/path-owned-id",
            serde_json::to_vec(&update).unwrap(),
            true,
        ),
    )
    .await;

    let body = error_response(response, StatusCode::UNPROCESSABLE_ENTITY).await;
    let error = body["error"].as_str().unwrap();

    assert!(error.contains("unknown field"));
    assert!(error.contains("id"));

    let response = call(
        &app,
        request(
            Method::POST,
            "/api/drafts/finalize-contract/finalize",
            r#"{"expected_revision":0,"protocol_version":"CLIENT-OWNS-NOTHING"}"#,
            true,
        ),
    )
    .await;

    let body = error_response(response, StatusCode::UNPROCESSABLE_ENTITY).await;
    let error = body["error"].as_str().unwrap();

    assert!(error.contains("unknown field"));
    assert!(error.contains("protocol_version"));
}

#[tokio::test]
async fn router_method_and_path_rejections_are_json_contracts() {
    let app = lazy_app();

    let response = call(
        &app,
        request(Method::GET, "/api/central-committee/nonexistent", "", false),
    )
    .await;

    let body = error_response(response, StatusCode::NOT_FOUND).await;
    assert_eq!(body["error"], "route not found");

    let response = call(
        &app,
        request(Method::DELETE, "/api/drafts/method-proof-001", "", false),
    )
    .await;

    let allow = response
        .headers()
        .get(ALLOW)
        .expect("405 must preserve Allow header")
        .to_str()
        .unwrap()
        .to_string();

    assert!(allow.contains("GET"));
    assert!(allow.contains("HEAD"));
    assert!(allow.contains("PUT"));

    let body = error_response(response, StatusCode::METHOD_NOT_ALLOWED).await;
    assert_eq!(body["error"], "method not allowed");

    let response = call(
        &app,
        request(Method::GET, "/api/runs/not-a-number", "", false),
    )
    .await;

    let body = error_response(response, StatusCode::BAD_REQUEST).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Cannot parse `not-a-number` to a `i64`")
    );
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn duplicate_create_is_json_conflict() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL API contract test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let app = app_with_pool(pool.clone());

    let id = "api-contract-duplicate-create";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let payload = serde_json::to_vec(&draft_payload(id)).unwrap();

    let response = call(
        &app,
        request(Method::POST, "/api/drafts", payload.clone(), true),
    )
    .await;

    let created = json_response(response, StatusCode::CREATED).await;
    assert_eq!(created["id"], id);
    assert_eq!(created["revision"], 0);

    let response = call(&app, request(Method::POST, "/api/drafts", payload, true)).await;

    let body = error_response(response, StatusCode::CONFLICT).await;

    assert_eq!(body["error"], format!("draft already exists: {id}"));

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn postgres_domain_not_found_and_stale_update_are_json_contracts() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL API contract test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let app = app_with_pool(pool.clone());

    let missing_id = "api-contract-domain-missing";
    let update_id = "api-contract-stale-update";

    for id in [missing_id, update_id] {
        sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let response = call(
        &app,
        request(Method::GET, &format!("/api/drafts/{missing_id}"), "", false),
    )
    .await;

    let body = error_response(response, StatusCode::NOT_FOUND).await;

    assert_eq!(body["error"], format!("draft not found: {missing_id}"));

    let create_body = serde_json::to_vec(&draft_payload(update_id)).unwrap();

    let response = call(
        &app,
        request(Method::POST, "/api/drafts", create_body, true),
    )
    .await;

    let created = json_response(response, StatusCode::CREATED).await;
    assert_eq!(created["revision"], 0);

    let first_update = serde_json::to_vec(&update_payload(0)).unwrap();

    let response = call(
        &app,
        request(
            Method::PUT,
            &format!("/api/drafts/{update_id}"),
            first_update.clone(),
            true,
        ),
    )
    .await;

    let updated = json_response(response, StatusCode::OK).await;
    assert_eq!(updated["revision"], 1);
    assert_eq!(updated["money"]["revenue"], 125000.0);

    let response = call(
        &app,
        request(
            Method::PUT,
            &format!("/api/drafts/{update_id}"),
            first_update,
            true,
        ),
    )
    .await;

    let body = error_response(response, StatusCode::CONFLICT).await;

    assert_eq!(
        body["error"],
        format!("draft revision conflict for {update_id}: expected revision 0")
    );

    let revision: i64 = sqlx::query_scalar(
        "SELECT revision
         FROM distribution_drafts
         WHERE id = $1",
    )
    .bind(update_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(revision, 1);

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(update_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn corrupt_persisted_draft_is_sanitized_at_api_boundary() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL API contract test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let app = app_with_pool(pool.clone());

    let id = "api-contract-corrupt-persisted-draft";

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let mut corrupt_body = draft_payload(id);
    corrupt_body.as_object_mut().unwrap().remove("id");

    corrupt_body["money"]["revenue"] = json!(-1.0);

    sqlx::query(
        r#"
        INSERT INTO distribution_drafts (id, revision, body)
        VALUES ($1, 0, $2)
        "#,
    )
    .bind(id)
    .bind(sqlx::types::Json(corrupt_body))
    .execute(&pool)
    .await
    .unwrap();

    let response = call(
        &app,
        request(Method::GET, &format!("/api/drafts/{id}"), "", false),
    )
    .await;

    let body = error_response(response, StatusCode::INTERNAL_SERVER_ERROR).await;

    assert_eq!(body["error"], "internal server error");

    let serialized = serde_json::to_string(&body).unwrap();

    assert!(!serialized.contains("negative"));
    assert!(!serialized.contains("database contains invalid draft"));
    assert!(!serialized.contains(id));

    sqlx::query("DELETE FROM distribution_drafts WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
}

fn finalizable_draft_payload(id: &str) -> Value {
    let mut payload = draft_payload(id);

    payload["contributors"] = json!([
        {
            "contributor_id": "backend",
            "contribution_id": "backend-engineering",
            "base_work_value": 1.0,
            "factors": {
                "responsibility": 1.0,
                "criticality": 1.0,
                "deal_impact": 1.0,
                "stewardship": 0.0,
                "dependency_reach": 1.0,
                "evidence_confidence": 1.0
            }
        }
    ]);

    payload
}

async fn cleanup_finalized_api_contract(pool: &PgPool, id: &str) {
    let mut tx = pool.begin().await.unwrap();

    sqlx::query(
        "ALTER TABLE distribution_runs
         DISABLE TRIGGER distribution_runs_reject_delete",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    sqlx::query(
        "DELETE FROM distribution_runs
         WHERE source_draft_id = $1",
    )
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
async fn finalized_run_http_round_trip_preserves_authoritative_snapshot() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL API contract test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let app = app_with_pool(pool.clone());

    let id = "api-contract-finalized-round-trip";

    cleanup_finalized_api_contract(&pool, id).await;

    let response = call(
        &app,
        request(
            Method::POST,
            "/api/drafts",
            serde_json::to_vec(&finalizable_draft_payload(id)).unwrap(),
            true,
        ),
    )
    .await;

    let created = json_response(response, StatusCode::CREATED).await;

    assert_eq!(created["id"], id);
    assert_eq!(created["revision"], 0);
    assert_eq!(created["money"]["revenue"], 120000.0);

    let response = call(
        &app,
        request(
            Method::POST,
            &format!("/api/drafts/{id}/finalize"),
            r#"{"expected_revision":0}"#,
            true,
        ),
    )
    .await;

    let finalized = json_response(response, StatusCode::CREATED).await;

    assert_eq!(finalized["source_draft_id"], id);
    assert_eq!(finalized["source_revision"], 0);
    assert_eq!(finalized["protocol_version"], "PP-0.1-draft");
    assert_eq!(
        finalized["engine_version"],
        concat!("distribution-machine/", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(finalized["manual_version"], "manual/0.1-draft");

    let run_id = finalized["id"]
        .as_i64()
        .expect("finalized response must expose numeric run id");

    let response = call(
        &app,
        request(Method::GET, &format!("/api/runs/{run_id}"), "", false),
    )
    .await;

    let run = json_response(response, StatusCode::OK).await;

    assert_eq!(run["id"], run_id);
    assert_eq!(run["source_draft_id"], id);
    assert_eq!(run["source_revision"], 0);
    assert_eq!(run["protocol_version"], "PP-0.1-draft");
    assert_eq!(
        run["engine_version"],
        concat!("distribution-machine/", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(run["manual_version"], "manual/0.1-draft");

    assert_eq!(run["draft_snapshot"]["id"], id);
    assert_eq!(run["draft_snapshot"]["revision"], 0);
    assert_eq!(run["draft_snapshot"]["money"]["revenue"], 120000.0);

    assert_eq!(
        run["preview_snapshot"]["available_for_distribution"],
        30000.0
    );
    assert_eq!(
        run["preview_snapshot"]["pool_amounts"]["contribution"],
        10500.0
    );

    assert!(
        run["finalized_at"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    let response = call(
        &app,
        request(
            Method::PUT,
            &format!("/api/drafts/{id}"),
            serde_json::to_vec(&update_payload(0)).unwrap(),
            true,
        ),
    )
    .await;

    let updated = json_response(response, StatusCode::OK).await;

    assert_eq!(updated["revision"], 1);
    assert_eq!(updated["money"]["revenue"], 125000.0);

    let response = call(
        &app,
        request(Method::GET, &format!("/api/runs/{run_id}"), "", false),
    )
    .await;

    let historical = json_response(response, StatusCode::OK).await;

    assert_eq!(historical["source_revision"], 0);
    assert_eq!(historical["draft_snapshot"]["money"]["revenue"], 120000.0);
    assert_eq!(
        historical["preview_snapshot"]["available_for_distribution"],
        30000.0
    );

    cleanup_finalized_api_contract(&pool, id).await;
}

fn finalizable_update_payload(expected_revision: u64, revenue: f64) -> Value {
    let mut payload = finalizable_draft_payload("body-id-not-authoritative");

    payload.as_object_mut().unwrap().remove("id");
    payload["expected_revision"] = json!(expected_revision);
    payload["money"]["revenue"] = json!(revenue);

    payload
}

#[tokio::test]
#[ignore = "requires Commie PostgreSQL via DATABASE_URL"]
async fn finalized_run_history_is_metadata_only_and_newest_revision_first() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for the PostgreSQL API contract test");

    let pool = PgPool::connect(&database_url).await.unwrap();
    let app = app_with_pool(pool.clone());

    let missing_id = "api-contract-run-history-missing";
    let empty_id = "api-contract-run-history-empty";
    let history_id = "api-contract-run-history";

    for id in [missing_id, empty_id, history_id] {
        cleanup_finalized_api_contract(&pool, id).await;
    }

    let response = call(
        &app,
        request(
            Method::GET,
            &format!("/api/drafts/{missing_id}/runs"),
            "",
            false,
        ),
    )
    .await;

    let body = error_response(response, StatusCode::NOT_FOUND).await;

    assert_eq!(body["error"], format!("draft not found: {missing_id}"));

    let response = call(
        &app,
        request(
            Method::POST,
            "/api/drafts",
            serde_json::to_vec(&draft_payload(empty_id)).unwrap(),
            true,
        ),
    )
    .await;

    json_response(response, StatusCode::CREATED).await;

    let response = call(
        &app,
        request(
            Method::GET,
            &format!("/api/drafts/{empty_id}/runs"),
            "",
            false,
        ),
    )
    .await;

    let empty = json_response(response, StatusCode::OK).await;

    assert_eq!(empty["draft_id"], empty_id);
    assert_eq!(empty["runs"], json!([]));

    let response = call(
        &app,
        request(
            Method::POST,
            "/api/drafts",
            serde_json::to_vec(&finalizable_draft_payload(history_id)).unwrap(),
            true,
        ),
    )
    .await;

    json_response(response, StatusCode::CREATED).await;

    let response = call(
        &app,
        request(
            Method::POST,
            &format!("/api/drafts/{history_id}/finalize"),
            r#"{"expected_revision":0}"#,
            true,
        ),
    )
    .await;

    let revision_zero = json_response(response, StatusCode::CREATED).await;
    let revision_zero_run_id = revision_zero["id"].as_i64().unwrap();

    let response = call(
        &app,
        request(
            Method::PUT,
            &format!("/api/drafts/{history_id}"),
            serde_json::to_vec(&finalizable_update_payload(0, 125000.0)).unwrap(),
            true,
        ),
    )
    .await;

    let updated = json_response(response, StatusCode::OK).await;

    assert_eq!(updated["revision"], 1);
    assert_eq!(updated["money"]["revenue"], 125000.0);

    let response = call(
        &app,
        request(
            Method::POST,
            &format!("/api/drafts/{history_id}/finalize"),
            r#"{"expected_revision":1}"#,
            true,
        ),
    )
    .await;

    let revision_one = json_response(response, StatusCode::CREATED).await;
    let revision_one_run_id = revision_one["id"].as_i64().unwrap();

    let response = call(
        &app,
        request(
            Method::GET,
            &format!("/api/drafts/{history_id}/runs"),
            "",
            false,
        ),
    )
    .await;

    let history = json_response(response, StatusCode::OK).await;

    assert_eq!(history["draft_id"], history_id);

    let runs = history["runs"]
        .as_array()
        .expect("run history must expose a runs array");

    assert_eq!(runs.len(), 2);

    assert_eq!(runs[0]["id"], revision_one_run_id);
    assert_eq!(runs[0]["source_revision"], 1);

    assert_eq!(runs[1]["id"], revision_zero_run_id);
    assert_eq!(runs[1]["source_revision"], 0);

    for run in runs {
        assert_eq!(run["protocol_version"], "PP-0.1-draft");
        assert_eq!(
            run["engine_version"],
            concat!("distribution-machine/", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(run["manual_version"], "manual/0.1-draft");

        assert!(
            run["finalized_at"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );

        assert!(run.get("draft_snapshot").is_none());
        assert!(run.get("preview_snapshot").is_none());
        assert!(run.get("source_draft_id").is_none());
    }

    for id in [empty_id, history_id] {
        cleanup_finalized_api_contract(&pool, id).await;
    }
}
