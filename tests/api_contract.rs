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
