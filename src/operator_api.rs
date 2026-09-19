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
                    "finalized_at": "2026-09-19 16:00:00+00"
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
