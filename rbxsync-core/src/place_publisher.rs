use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

const ROBLOX_OPEN_CLOUD_BASE_URL: &str = "https://apis.roblox.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublishPlaceFormat {
    Rbxl,
    Rbxlx,
}

impl PublishPlaceFormat {
    pub fn from_path(path: &std::path::Path) -> Result<Self> {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("rbxl") => Ok(Self::Rbxl),
            Some("rbxlx") => Ok(Self::Rbxlx),
            _ => bail!("publish-place only supports .rbxl and .rbxlx files"),
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Rbxl => "application/octet-stream",
            Self::Rbxlx => "application/xml",
        }
    }
}

impl std::fmt::Display for PublishPlaceFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Rbxl => "rbxl",
            Self::Rbxlx => "rbxlx",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishVersionType {
    Published,
    Saved,
}

impl PublishVersionType {
    pub fn as_query_value(self) -> &'static str {
        match self {
            Self::Published => "Published",
            Self::Saved => "Saved",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublishPlaceDiagnosticKind {
    InvalidInput,
    AuthenticationFailed,
    PermissionDenied,
    NotFound,
    RateLimited,
    InvalidPlaceFile,
    NetworkError,
    UnexpectedResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishPlaceDiagnostic {
    pub kind: PublishPlaceDiagnosticKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPlaceError {
    pub diagnostic: PublishPlaceDiagnostic,
}

impl std::fmt::Display for PublishPlaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:?}: {}",
            self.diagnostic.kind, self.diagnostic.message
        )
    }
}

impl std::error::Error for PublishPlaceError {}

#[derive(Debug, Clone)]
pub struct PublishPlaceOptions {
    pub input_path: PathBuf,
    pub universe_id: u64,
    pub place_id: u64,
    pub api_key: String,
    pub version_type: PublishVersionType,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct PublishPlaceSummary {
    pub input_path: PathBuf,
    pub universe_id: u64,
    pub place_id: u64,
    pub format: PublishPlaceFormat,
    pub content_type: String,
    pub bytes: u64,
    pub version_type: PublishVersionType,
    pub dry_run: bool,
    pub version_number: Option<u64>,
    pub diagnostics: Vec<PublishPlaceDiagnostic>,
}

#[derive(Debug, Clone)]
pub struct PublishPlaceHttpRequest {
    pub url: String,
    pub api_key: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PublishPlaceHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

pub type PublishTransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PublishPlaceHttpResponse>> + Send + 'a>>;

pub trait PublishPlaceTransport: Send + Sync {
    fn post<'a>(&'a self, request: PublishPlaceHttpRequest) -> PublishTransportFuture<'a>;
}

#[derive(Debug, Clone, Default)]
pub struct ReqwestPublishPlaceTransport;

impl PublishPlaceTransport for ReqwestPublishPlaceTransport {
    fn post<'a>(&'a self, request: PublishPlaceHttpRequest) -> PublishTransportFuture<'a> {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(120))
                .user_agent(format!("rbxsync/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .context("Failed to initialize Roblox place publish HTTP client")?;
            let response = client
                .post(&request.url)
                .header("x-api-key", request.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .header(reqwest::header::CONTENT_TYPE, request.content_type)
                .body(request.body)
                .send()
                .await
                .context("Failed to send Roblox place publish request")?;
            let status = response.status().as_u16();
            let headers = response_headers(response.headers());
            let body = response.text().await.unwrap_or_default();
            Ok(PublishPlaceHttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}

pub async fn publish_place(options: PublishPlaceOptions) -> Result<PublishPlaceSummary> {
    publish_place_with_transport(options, &ReqwestPublishPlaceTransport::default()).await
}

pub async fn publish_place_with_transport(
    options: PublishPlaceOptions,
    transport: &dyn PublishPlaceTransport,
) -> Result<PublishPlaceSummary> {
    let (summary, request) = prepare_publish_request(&options)?;

    if options.dry_run {
        return Ok(summary);
    }

    let response = transport.post(request).await.map_err(|error| {
        publish_place_error(PublishPlaceDiagnosticKind::NetworkError, error.to_string())
    })?;
    let version_number = parse_publish_response(&response)?;

    Ok(PublishPlaceSummary {
        version_number: Some(version_number),
        ..summary
    })
}

fn prepare_publish_request(
    options: &PublishPlaceOptions,
) -> Result<(PublishPlaceSummary, PublishPlaceHttpRequest)> {
    if options.universe_id == 0 {
        bail!(
            "{}",
            publish_place_error(
                PublishPlaceDiagnosticKind::InvalidInput,
                "Universe ID must be greater than zero",
            )
        );
    }
    if options.place_id == 0 {
        bail!(
            "{}",
            publish_place_error(
                PublishPlaceDiagnosticKind::InvalidInput,
                "Place ID must be greater than zero",
            )
        );
    }
    if options.api_key.trim().is_empty() {
        bail!(
            "{}",
            publish_place_error(
                PublishPlaceDiagnosticKind::AuthenticationFailed,
                "Open Cloud API key is required",
            )
        );
    }

    let input_path = options.input_path.canonicalize().with_context(|| {
        format!(
            "Failed to resolve input path {}",
            options.input_path.display()
        )
    })?;
    let format = PublishPlaceFormat::from_path(&input_path)?;
    let body = std::fs::read(&input_path)
        .with_context(|| format!("Failed to read {}", input_path.display()))?;
    let bytes = body.len() as u64;
    let content_type = format.content_type().to_string();
    let url = publish_place_url(
        ROBLOX_OPEN_CLOUD_BASE_URL,
        options.universe_id,
        options.place_id,
        options.version_type,
    );

    let summary = PublishPlaceSummary {
        input_path,
        universe_id: options.universe_id,
        place_id: options.place_id,
        format,
        content_type: content_type.clone(),
        bytes,
        version_type: options.version_type,
        dry_run: options.dry_run,
        version_number: None,
        diagnostics: Vec::new(),
    };
    let request = PublishPlaceHttpRequest {
        url,
        api_key: options.api_key.clone(),
        content_type,
        body,
    };

    Ok((summary, request))
}

pub fn publish_place_url(
    base_url: &str,
    universe_id: u64,
    place_id: u64,
    version_type: PublishVersionType,
) -> String {
    format!(
        "{}/universes/v1/{}/places/{}/versions?versionType={}",
        base_url.trim_end_matches('/'),
        universe_id,
        place_id,
        version_type.as_query_value()
    )
}

fn parse_publish_response(response: &PublishPlaceHttpResponse) -> Result<u64> {
    if !(200..300).contains(&response.status) {
        let kind = diagnostic_kind_for_status(response.status);
        bail!(
            "{}",
            publish_place_error(kind, response_error_message(response))
        );
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct PublishSuccess {
        version_number: u64,
    }

    let success: PublishSuccess = serde_json::from_str(&response.body).map_err(|error| {
        anyhow::anyhow!(
            "{}",
            publish_place_error(
                PublishPlaceDiagnosticKind::UnexpectedResponse,
                format!("Roblox returned invalid publish JSON: {error}")
            )
        )
    })?;
    Ok(success.version_number)
}

fn diagnostic_kind_for_status(status: u16) -> PublishPlaceDiagnosticKind {
    match status {
        400 => PublishPlaceDiagnosticKind::InvalidInput,
        401 => PublishPlaceDiagnosticKind::AuthenticationFailed,
        403 => PublishPlaceDiagnosticKind::PermissionDenied,
        404 => PublishPlaceDiagnosticKind::NotFound,
        413 | 415 | 422 => PublishPlaceDiagnosticKind::InvalidPlaceFile,
        429 => PublishPlaceDiagnosticKind::RateLimited,
        _ => PublishPlaceDiagnosticKind::UnexpectedResponse,
    }
}

fn response_error_message(response: &PublishPlaceHttpResponse) -> String {
    let retry_after = response
        .headers
        .get("retry-after")
        .map(|value| format!(" Retry after: {value}."))
        .unwrap_or_default();

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&response.body) {
        if let Some(message) = first_json_message(&value) {
            return format!(
                "Roblox publish request failed with status {}: {message}{retry_after}",
                response.status,
            );
        }
    }

    let base = if response.body.trim().is_empty() {
        format!(
            "Roblox publish request failed with status {} and no response body",
            response.status
        )
    } else {
        format!(
            "Roblox publish request failed with status {}: {}",
            response.status,
            response.body.trim()
        )
    };

    format!("{base}{retry_after}")
}

fn first_json_message(value: &serde_json::Value) -> Option<String> {
    if let Some(message) = value.get("message").and_then(|message| message.as_str()) {
        return Some(message.to_string());
    }
    if let Some(message) = value
        .get("error")
        .and_then(|error| error.get("message").or_else(|| error.get("errorMessage")))
        .and_then(|message| message.as_str())
    {
        return Some(message.to_string());
    }
    if let Some(message) = value
        .get("errorMessage")
        .and_then(|message| message.as_str())
    {
        return Some(message.to_string());
    }
    if let Some(errors) = value.get("errors").and_then(|errors| errors.as_array()) {
        let messages = errors
            .iter()
            .filter_map(|error| error.get("message").and_then(|message| message.as_str()))
            .collect::<Vec<_>>();
        if !messages.is_empty() {
            return Some(messages.join("; "));
        }
    }

    None
}

fn response_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_ascii_lowercase(), value.to_string()))
        })
        .collect()
}

fn publish_place_error(
    kind: PublishPlaceDiagnosticKind,
    message: impl Into<String>,
) -> PublishPlaceError {
    PublishPlaceError {
        diagnostic: PublishPlaceDiagnostic {
            kind,
            message: message.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct FakeTransport {
        response: Result<PublishPlaceHttpResponse, String>,
        requests: Arc<Mutex<Vec<PublishPlaceHttpRequest>>>,
    }

    impl FakeTransport {
        fn success(version_number: u64) -> Self {
            Self {
                response: Ok(PublishPlaceHttpResponse {
                    status: 200,
                    headers: BTreeMap::new(),
                    body: format!(r#"{{"versionNumber":{version_number}}}"#),
                }),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn failure(status: u16, body: &str) -> Self {
            Self {
                response: Ok(PublishPlaceHttpResponse {
                    status,
                    headers: BTreeMap::new(),
                    body: body.to_string(),
                }),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn network_error(message: &str) -> Self {
            Self {
                response: Err(message.to_string()),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn rate_limited(body: &str, retry_after: &str) -> Self {
            let mut headers = BTreeMap::new();
            headers.insert("retry-after".to_string(), retry_after.to_string());
            Self {
                response: Ok(PublishPlaceHttpResponse {
                    status: 429,
                    headers,
                    body: body.to_string(),
                }),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn last_request(&self) -> PublishPlaceHttpRequest {
            self.requests.lock().unwrap().last().unwrap().clone()
        }
    }

    impl PublishPlaceTransport for FakeTransport {
        fn post<'a>(&'a self, request: PublishPlaceHttpRequest) -> PublishTransportFuture<'a> {
            Box::pin(async move {
                self.requests.lock().unwrap().push(request);
                self.response
                    .clone()
                    .map_err(|message| anyhow::anyhow!(message))
            })
        }
    }

    fn options(input_path: PathBuf) -> PublishPlaceOptions {
        PublishPlaceOptions {
            input_path,
            universe_id: 123,
            place_id: 456,
            api_key: "test-key".to_string(),
            version_type: PublishVersionType::Published,
            dry_run: false,
        }
    }

    #[test]
    fn detects_format_and_content_type_from_extension() {
        assert_eq!(
            PublishPlaceFormat::from_path(std::path::Path::new("game.rbxl")).unwrap(),
            PublishPlaceFormat::Rbxl
        );
        assert_eq!(
            PublishPlaceFormat::from_path(std::path::Path::new("game.rbxlx")).unwrap(),
            PublishPlaceFormat::Rbxlx
        );
        assert_eq!(
            PublishPlaceFormat::Rbxl.content_type(),
            "application/octet-stream"
        );
        assert_eq!(PublishPlaceFormat::Rbxlx.content_type(), "application/xml");
        assert!(PublishPlaceFormat::from_path(std::path::Path::new("game.txt")).is_err());
    }

    #[test]
    fn builds_publish_endpoint() {
        assert_eq!(
            publish_place_url(
                "https://apis.roblox.com/",
                123,
                456,
                PublishVersionType::Published
            ),
            "https://apis.roblox.com/universes/v1/123/places/456/versions?versionType=Published"
        );
        assert_eq!(
            publish_place_url("https://example.test", 1, 2, PublishVersionType::Saved),
            "https://example.test/universes/v1/1/places/2/versions?versionType=Saved"
        );
    }

    #[tokio::test]
    async fn dry_run_summarizes_without_uploading() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let mut options = options(input);
        options.dry_run = true;
        let transport = FakeTransport::success(7);

        let summary = publish_place_with_transport(options, &transport)
            .await
            .unwrap();

        assert_eq!(transport.request_count(), 0);
        assert_eq!(summary.format, PublishPlaceFormat::Rbxl);
        assert_eq!(summary.content_type, "application/octet-stream");
        assert_eq!(summary.bytes, 11);
        assert!(summary.dry_run);
        assert_eq!(summary.version_number, None);
        assert!(summary.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn uploads_place_file_and_parses_version_number() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxlx");
        std::fs::write(&input, b"<roblox />").unwrap();
        let transport = FakeTransport::success(42);

        let summary = publish_place_with_transport(options(input), &transport)
            .await
            .unwrap();
        let request = transport.last_request();

        assert_eq!(summary.version_number, Some(42));
        assert_eq!(summary.format, PublishPlaceFormat::Rbxlx);
        assert_eq!(request.content_type, "application/xml");
        assert_eq!(request.api_key, "test-key");
        assert_eq!(request.body, b"<roblox />");
        assert_eq!(
            request.url,
            "https://apis.roblox.com/universes/v1/123/places/456/versions?versionType=Published"
        );
    }

    #[tokio::test]
    async fn maps_http_error_response() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let transport = FakeTransport::failure(403, r#"{"message":"API key cannot publish"}"#);

        let error = publish_place_with_transport(options(input), &transport)
            .await
            .expect_err("publish should fail");

        assert!(error.to_string().contains("PermissionDenied"));
        assert!(error.to_string().contains("API key cannot publish"));
    }

    #[tokio::test]
    async fn maps_rate_limit_with_retry_after_header() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let transport =
            FakeTransport::rate_limited(r#"{"error":{"message":"Too many requests"}}"#, "30");

        let error = publish_place_with_transport(options(input), &transport)
            .await
            .expect_err("publish should fail");

        assert!(error.to_string().contains("RateLimited"));
        assert!(error.to_string().contains("Too many requests"));
        assert!(error.to_string().contains("Retry after: 30"));
    }

    #[tokio::test]
    async fn maps_bad_request_as_invalid_input() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let transport = FakeTransport::failure(
            400,
            r#"{"errors":[{"message":"versionType is invalid"},{"message":"placeId is invalid"}]}"#,
        );

        let error = publish_place_with_transport(options(input), &transport)
            .await
            .expect_err("publish should fail");

        assert!(error.to_string().contains("InvalidInput"));
        assert!(error.to_string().contains("versionType is invalid"));
        assert!(error.to_string().contains("placeId is invalid"));
    }

    #[tokio::test]
    async fn rejects_success_response_without_version_number() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let transport = FakeTransport::failure(200, r#"{"ok":true}"#);

        let error = publish_place_with_transport(options(input), &transport)
            .await
            .expect_err("publish should fail");

        assert!(error.to_string().contains("UnexpectedResponse"));
        assert!(error.to_string().contains("invalid publish JSON"));
    }

    #[tokio::test]
    async fn maps_network_error_without_leaking_api_key() {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("game.rbxl");
        std::fs::write(&input, b"placeholder").unwrap();
        let transport = FakeTransport::network_error("connection refused");

        let error = publish_place_with_transport(options(input), &transport)
            .await
            .expect_err("publish should fail");

        assert!(error.to_string().contains("NetworkError"));
        assert!(error.to_string().contains("connection refused"));
        assert!(!error.to_string().contains("test-key"));
    }
}
