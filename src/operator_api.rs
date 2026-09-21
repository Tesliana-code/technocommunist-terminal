use std::{error::Error, fmt};

use reqwest::{StatusCode, Url, blocking::Client};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorRunMetadata {
    pub id: i64,
    pub source_revision: u64,
    pub protocol_version: String,
    pub engine_version: String,
    pub manual_version: String,
    pub finalized_at: String,
    pub telemetry: OperatorRunTelemetry,
}

/// A typed summary derived strictly from a run's persisted, immutable `preview_snapshot`.
/// Never recomputed from the current draft or the current Distribution Engine.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorRunTelemetry {
    pub available_for_distribution: f64,
    pub contribution_pool: f64,
    pub allocation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorRunHistory {
    pub draft_id: String,
    pub runs: Vec<OperatorRunMetadata>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorRunDetail {
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

/// A typed "why this amount?" read model for one contribution within one finalized run,
/// combining ONLY that run's immutable `draft_snapshot` and `preview_snapshot`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorAllocationExplanation {
    pub run_id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,

    pub draft_snapshot_schema_version: String,
    pub preview_snapshot_schema_version: String,

    pub contributor_id: String,
    pub contribution_id: String,

    pub base_work_value: f64,

    pub weights: OperatorContributionWeights,
    pub factors: OperatorContributionFactors,
    pub factor_terms: OperatorContributionFactorTerms,

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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct OperatorContributionWeights {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct OperatorContributionFactors {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
    pub evidence_confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct OperatorContributionFactorTerms {
    pub responsibility: f64,
    pub criticality: f64,
    pub deal_impact: f64,
    pub dependency_reach: f64,
}

/// A lightweight selector listing which contributions exist inside one finalized run.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorRunContributions {
    pub run_id: i64,
    pub source_draft_id: String,
    pub source_revision: u64,
    pub contributions: Vec<OperatorContributionOption>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OperatorContributionOption {
    pub contributor_id: String,
    pub contribution_id: String,
    pub normalized_share: f64,
    pub allocated_amount: f64,
}

#[derive(Debug)]
pub enum OperatorApiError {
    Configuration(String),
    Transport(reqwest::Error),
    Http { status: StatusCode, error: String },
}

impl fmt::Display for OperatorApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(error) => {
                write!(f, "operator API configuration error: {error}")
            }
            Self::Transport(error) => {
                write!(f, "operator API transport error: {error}")
            }
            Self::Http { status, error } => {
                write!(f, "operator API returned {status}: {error}")
            }
        }
    }
}

impl Error for OperatorApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::Configuration(_) | Self::Http { .. } => None,
        }
    }
}

impl From<reqwest::Error> for OperatorApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error)
    }
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: String,
}

#[derive(Debug, Clone)]
pub struct OperatorApiClient {
    base_url: Url,
    client: Client,
}

impl OperatorApiClient {
    pub fn new(base_url: &str) -> Result<Self, OperatorApiError> {
        let mut base_url = Url::parse(base_url)
            .map_err(|error| OperatorApiError::Configuration(error.to_string()))?;

        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(OperatorApiError::Configuration(
                "base URL must use http or https".to_string(),
            ));
        }

        if base_url.query().is_some() || base_url.fragment().is_some() {
            return Err(OperatorApiError::Configuration(
                "base URL must not contain query or fragment".to_string(),
            ));
        }

        if !matches!(base_url.path(), "" | "/") {
            return Err(OperatorApiError::Configuration(
                "base URL must not contain an API path".to_string(),
            ));
        }

        base_url.set_path("/");

        Ok(Self {
            base_url,
            client: Client::new(),
        })
    }

    pub fn list_finalized_runs_for_draft(
        &self,
        draft_id: &str,
    ) -> Result<OperatorRunHistory, OperatorApiError> {
        let url = self.run_history_url(draft_id)?;

        let response = self.client.get(url).send()?;
        let status = response.status();

        if !status.is_success() {
            let fallback = format!("request failed with HTTP {}", status.as_u16());

            let error = response
                .json::<ErrorEnvelope>()
                .map(|body| body.error)
                .unwrap_or(fallback);

            return Err(OperatorApiError::Http { status, error });
        }

        Ok(response.json::<OperatorRunHistory>()?)
    }

    pub fn get_finalized_run(&self, run_id: i64) -> Result<OperatorRunDetail, OperatorApiError> {
        let url = self.run_detail_url(run_id)?;

        let response = self.client.get(url).send()?;
        let status = response.status();

        if !status.is_success() {
            let fallback = format!("request failed with HTTP {}", status.as_u16());

            let error = response
                .json::<ErrorEnvelope>()
                .map(|body| body.error)
                .unwrap_or(fallback);

            return Err(OperatorApiError::Http { status, error });
        }

        Ok(response.json::<OperatorRunDetail>()?)
    }

    pub fn get_allocation_explanation(
        &self,
        run_id: i64,
        contribution_id: &str,
    ) -> Result<OperatorAllocationExplanation, OperatorApiError> {
        let url = self.allocation_explanation_url(run_id, contribution_id)?;

        let response = self.client.get(url).send()?;
        let status = response.status();

        if !status.is_success() {
            let fallback = format!("request failed with HTTP {}", status.as_u16());

            let error = response
                .json::<ErrorEnvelope>()
                .map(|body| body.error)
                .unwrap_or(fallback);

            return Err(OperatorApiError::Http { status, error });
        }

        Ok(response.json::<OperatorAllocationExplanation>()?)
    }

    pub fn list_run_contributions(
        &self,
        run_id: i64,
    ) -> Result<OperatorRunContributions, OperatorApiError> {
        let url = self.run_contributions_url(run_id)?;

        let response = self.client.get(url).send()?;
        let status = response.status();

        if !status.is_success() {
            let fallback = format!("request failed with HTTP {}", status.as_u16());

            let error = response
                .json::<ErrorEnvelope>()
                .map(|body| body.error)
                .unwrap_or(fallback);

            return Err(OperatorApiError::Http { status, error });
        }

        Ok(response.json::<OperatorRunContributions>()?)
    }

    fn run_history_url(&self, draft_id: &str) -> Result<Url, OperatorApiError> {
        let mut url = self.base_url.clone();

        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                OperatorApiError::Configuration("base URL cannot accept path segments".to_string())
            })?;

            segments.clear();
            segments.push("api");
            segments.push("drafts");
            segments.push(draft_id);
            segments.push("runs");
        }

        Ok(url)
    }

    fn run_detail_url(&self, run_id: i64) -> Result<Url, OperatorApiError> {
        let mut url = self.base_url.clone();
        let run_id = run_id.to_string();

        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                OperatorApiError::Configuration("base URL cannot accept path segments".to_string())
            })?;

            segments.clear();
            segments.push("api");
            segments.push("runs");
            segments.push(&run_id);
        }

        Ok(url)
    }

    fn allocation_explanation_url(
        &self,
        run_id: i64,
        contribution_id: &str,
    ) -> Result<Url, OperatorApiError> {
        let mut url = self.base_url.clone();
        let run_id = run_id.to_string();

        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                OperatorApiError::Configuration("base URL cannot accept path segments".to_string())
            })?;

            segments.clear();
            segments.push("api");
            segments.push("runs");
            segments.push(&run_id);
            segments.push("contributions");
            segments.push(contribution_id);
            segments.push("explanation");
        }

        Ok(url)
    }

    fn run_contributions_url(&self, run_id: i64) -> Result<Url, OperatorApiError> {
        let mut url = self.base_url.clone();
        let run_id = run_id.to_string();

        {
            let mut segments = url.path_segments_mut().map_err(|_| {
                OperatorApiError::Configuration("base URL cannot accept path segments".to_string())
            })?;

            segments.clear();
            segments.push("api");
            segments.push("runs");
            segments.push(&run_id);
            segments.push("contributions");
        }

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::Duration,
    };

    use super::*;

    fn serve_once(status: &str, body: &str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let status = status.to_string();
        let body = body.to_string();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();

            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).unwrap();

            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();

            let response = format!(
                "HTTP/1.1 {status}\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {body}",
                body.len(),
            );

            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();

            request
        });

        (format!("http://{address}"), handle)
    }

    #[test]
    fn run_history_url_preserves_draft_identity_as_one_path_segment() {
        let client = OperatorApiClient::new("http://127.0.0.1:18080").unwrap();

        let url = client.run_history_url("draft one/β").unwrap();

        assert_eq!(
            url.as_str(),
            "http://127.0.0.1:18080/api/drafts/draft%20one%2F%CE%B2/runs"
        );
    }

    #[test]
    fn allocation_explanation_url_preserves_contribution_identity_as_one_path_segment() {
        let client = OperatorApiClient::new("http://127.0.0.1:18080").unwrap();

        let url = client
            .allocation_explanation_url(54, "backend/engine β")
            .unwrap();

        assert_eq!(
            url.as_str(),
            "http://127.0.0.1:18080/api/runs/54/contributions/backend%2Fengine%20%CE%B2/explanation"
        );
    }

    #[test]
    fn client_reads_authoritative_run_history_over_http() {
        let body = r#"{
            "draft_id": "operator-proof",
            "runs": [
                {
                    "id": 42,
                    "source_revision": 3,
                    "protocol_version": "PP-0.1-draft",
                    "engine_version": "distribution-machine/0.1.0",
                    "manual_version": "manual/0.1-draft",
                    "finalized_at": "2026-09-19 16:00:00+00",
                    "telemetry": {
                        "available_for_distribution": 30000.0,
                        "contribution_pool": 10500.0,
                        "allocation_count": 1
                    }
                }
            ]
        }"#;

        let (base_url, server) = serve_once("200 OK", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let history = client
            .list_finalized_runs_for_draft("operator-proof")
            .unwrap();

        assert_eq!(history.draft_id, "operator-proof");
        assert_eq!(history.runs.len(), 1);
        assert_eq!(history.runs[0].id, 42);
        assert_eq!(history.runs[0].source_revision, 3);
        assert_eq!(
            history.runs[0].telemetry.available_for_distribution,
            30000.0
        );
        assert_eq!(history.runs[0].telemetry.contribution_pool, 10500.0);
        assert_eq!(history.runs[0].telemetry.allocation_count, 1);

        let request = server.join().unwrap();

        assert!(
            request.starts_with("GET /api/drafts/operator-proof/runs HTTP/1.1"),
            "unexpected request: {request}"
        );
    }

    #[test]
    fn client_reads_authoritative_immutable_run_detail_over_http() {
        let body = r#"{
            "id": 42,
            "source_draft_id": "operator-proof",
            "source_revision": 3,
            "draft_snapshot": {
                "id": "operator-proof",
                "revision": 3,
                "money": {
                    "revenue": 125000.0
                }
            },
            "preview_snapshot": {
                "available_for_distribution": 35000.0
            },
            "protocol_version": "PP-0.1-draft",
            "engine_version": "distribution-machine/0.1.0",
            "manual_version": "manual/0.1-draft",
            "finalized_at": "2026-09-19 16:00:00+00"
        }"#;

        let (base_url, server) = serve_once("200 OK", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let run = client.get_finalized_run(42).unwrap();

        assert_eq!(run.id, 42);
        assert_eq!(run.source_draft_id, "operator-proof");
        assert_eq!(run.source_revision, 3);
        assert_eq!(run.draft_snapshot["revision"], 3);
        assert_eq!(run.draft_snapshot["money"]["revenue"], 125000.0);
        assert_eq!(run.preview_snapshot["available_for_distribution"], 35000.0);

        let request = server.join().unwrap();

        assert!(
            request.starts_with("GET /api/runs/42 HTTP/1.1"),
            "unexpected request: {request}"
        );
    }

    #[test]
    fn client_reads_authoritative_allocation_explanation_over_http() {
        let body = r#"{
            "run_id": 54,
            "source_draft_id": "operator-proof",
            "source_revision": 0,
            "draft_snapshot_schema_version": "distribution-draft-snapshot/1",
            "preview_snapshot_schema_version": "draft-preview-snapshot/1",
            "contributor_id": "backend",
            "contribution_id": "backend-engineering",
            "base_work_value": 1.0,
            "weights": {
                "responsibility": 0.20,
                "criticality": 0.30,
                "deal_impact": 0.25,
                "dependency_reach": 0.25
            },
            "factors": {
                "responsibility": 1.0,
                "criticality": 1.0,
                "deal_impact": 1.0,
                "dependency_reach": 1.0,
                "evidence_confidence": 1.0
            },
            "factor_terms": {
                "responsibility": 0.20,
                "criticality": 0.30,
                "deal_impact": 0.25,
                "dependency_reach": 0.25
            },
            "weighted_factors": 1.0,
            "contribution_value": 1.0,
            "total_contribution_value": 1.0,
            "normalized_share": 1.0,
            "contribution_pool": 10500.0,
            "allocated_amount": 10500.0,
            "protocol_version": "PP-0.1-draft",
            "engine_version": "distribution-machine/0.1.0",
            "manual_version": "manual/0.1-draft",
            "finalized_at": "2026-09-19 16:00:00+00"
        }"#;

        let (base_url, server) = serve_once("200 OK", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let explanation = client
            .get_allocation_explanation(54, "backend-engineering")
            .unwrap();

        assert_eq!(explanation.run_id, 54);
        assert_eq!(explanation.source_draft_id, "operator-proof");
        assert_eq!(
            explanation.draft_snapshot_schema_version,
            "distribution-draft-snapshot/1"
        );
        assert_eq!(
            explanation.preview_snapshot_schema_version,
            "draft-preview-snapshot/1"
        );
        assert_eq!(explanation.contributor_id, "backend");
        assert_eq!(explanation.contribution_id, "backend-engineering");
        assert_eq!(explanation.base_work_value, 1.0);
        assert_eq!(explanation.weights.criticality, 0.30);
        assert_eq!(explanation.factors.evidence_confidence, 1.0);
        assert_eq!(explanation.factor_terms.responsibility, 0.20);
        assert_eq!(explanation.weighted_factors, 1.0);
        assert_eq!(explanation.contribution_value, 1.0);
        assert_eq!(explanation.total_contribution_value, 1.0);
        assert_eq!(explanation.normalized_share, 1.0);
        assert_eq!(explanation.contribution_pool, 10500.0);
        assert_eq!(explanation.allocated_amount, 10500.0);

        let request = server.join().unwrap();

        assert!(
            request.starts_with(
                "GET /api/runs/54/contributions/backend-engineering/explanation HTTP/1.1"
            ),
            "unexpected request: {request}"
        );
    }

    #[test]
    fn run_contributions_url_targets_the_selector_path() {
        let client = OperatorApiClient::new("http://127.0.0.1:18080").unwrap();

        let url = client.run_contributions_url(54).unwrap();

        assert_eq!(
            url.as_str(),
            "http://127.0.0.1:18080/api/runs/54/contributions"
        );
    }

    #[test]
    fn client_reads_authoritative_run_contributions_over_http() {
        let body = r#"{
            "run_id": 54,
            "source_draft_id": "operator-proof",
            "source_revision": 0,
            "contributions": [
                {
                    "contributor_id": "operations",
                    "contribution_id": "operations-delivery",
                    "normalized_share": 0.2839,
                    "allocated_amount": 2980.95
                },
                {
                    "contributor_id": "backend",
                    "contribution_id": "backend-engineering",
                    "normalized_share": 0.3843,
                    "allocated_amount": 4034.83
                }
            ]
        }"#;

        let (base_url, server) = serve_once("200 OK", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let contributions = client.list_run_contributions(54).unwrap();

        assert_eq!(contributions.run_id, 54);
        assert_eq!(contributions.source_draft_id, "operator-proof");
        assert_eq!(contributions.source_revision, 0);
        assert_eq!(contributions.contributions.len(), 2);

        assert_eq!(contributions.contributions[0].contributor_id, "operations");
        assert_eq!(
            contributions.contributions[0].contribution_id,
            "operations-delivery"
        );
        assert_eq!(contributions.contributions[0].normalized_share, 0.2839);
        assert_eq!(contributions.contributions[0].allocated_amount, 2980.95);

        assert_eq!(contributions.contributions[1].contributor_id, "backend");
        assert_eq!(
            contributions.contributions[1].contribution_id,
            "backend-engineering"
        );
        assert_eq!(contributions.contributions[1].normalized_share, 0.3843);
        assert_eq!(contributions.contributions[1].allocated_amount, 4034.83);

        let request = server.join().unwrap();

        assert!(
            request.starts_with("GET /api/runs/54/contributions HTTP/1.1"),
            "unexpected request: {request}"
        );
    }

    #[test]
    fn run_detail_preserves_api_error_status_and_message() {
        let body = r#"{"error":"distribution run not found: 9223372036854775807"}"#;

        let (base_url, server) = serve_once("404 Not Found", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let error = client.get_finalized_run(i64::MAX).unwrap_err();

        match error {
            OperatorApiError::Http { status, error } => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(error, "distribution run not found: 9223372036854775807");
            }
            other => panic!("unexpected operator API error: {other}"),
        }

        let request = server.join().unwrap();

        assert!(
            request.starts_with("GET /api/runs/9223372036854775807 HTTP/1.1"),
            "unexpected request: {request}"
        );
    }

    #[test]
    fn client_preserves_api_error_status_and_message() {
        let body = r#"{"error":"draft not found: missing-proof"}"#;

        let (base_url, server) = serve_once("404 Not Found", body);
        let client = OperatorApiClient::new(&base_url).unwrap();

        let error = client
            .list_finalized_runs_for_draft("missing-proof")
            .unwrap_err();

        match error {
            OperatorApiError::Http { status, error } => {
                assert_eq!(status, StatusCode::NOT_FOUND);
                assert_eq!(error, "draft not found: missing-proof");
            }
            other => panic!("unexpected operator API error: {other}"),
        }

        let request = server.join().unwrap();

        assert!(
            request.starts_with("GET /api/drafts/missing-proof/runs HTTP/1.1"),
            "unexpected request: {request}"
        );
    }
}
