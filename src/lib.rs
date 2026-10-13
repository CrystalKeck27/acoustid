use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use reqwest::{Client as HttpClient, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

const API_BASE: &str = "https://api.acoustid.org/v2";
const REQUEST_INTERVAL: Duration = Duration::from_millis(400);

static REQUEST_LIMITER: OnceLock<Arc<DefaultDirectRateLimiter>> = OnceLock::new();

fn request_limiter() -> Arc<DefaultDirectRateLimiter> {
    Arc::clone(REQUEST_LIMITER.get_or_init(|| {
        let quota = Quota::with_period(REQUEST_INTERVAL)
            .expect("request interval is non-zero")
            .allow_burst(NonZeroU32::new(1).expect("burst size is non-zero"));
        Arc::new(RateLimiter::direct(quota))
    }))
}

/// Async client for the read-only AcoustID web-service endpoints.
///
/// All client instances in this process share a governor rate limiter. Requests
/// are paced at 2.5 per second, below AcoustID's documented limit of 3 per second.
/// This limiter is process-local; coordinate separately when using multiple processes.
#[derive(Clone)]
pub struct AcoustIDClient {
    http: HttpClient,
    application_key: Option<Arc<str>>,
    limiter: Arc<DefaultDirectRateLimiter>,
}

impl AcoustIDClient {
    /// Creates a client that can call endpoints which do not require an application key.
    pub fn new() -> Result<Self, AcoustidError> {
        Self::build(None)
    }

    /// Creates a client configured with an AcoustID application API key.
    pub fn with_application_key(key: impl Into<String>) -> Result<Self, AcoustidError> {
        Self::build(Some(key.into()))
    }

    fn build(application_key: Option<String>) -> Result<Self, AcoustidError> {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self {
            http,
            application_key: application_key.map(Arc::<str>::from),
            limiter: request_limiter(),
        })
    }

    /// Looks up AcoustID tracks by a Chromaprint fingerprint and duration in seconds.
    pub async fn lookup_by_fingerprint(
        &self,
        duration: u32,
        fingerprint: &str,
        metadata: &[Metadata],
    ) -> Result<ApiResponse, AcoustidError> {
        let mut params = self.lookup_params(metadata)?;
        params.push(("duration", duration.to_string()));
        params.push(("fingerprint", fingerprint.to_owned()));
        self.get("lookup", params).await
    }

    /// Looks up an AcoustID track UUID and optional associated metadata.
    pub async fn lookup_by_track_id(
        &self,
        track_id: &str,
        metadata: &[Metadata],
    ) -> Result<ApiResponse, AcoustidError> {
        let mut params = self.lookup_params(metadata)?;
        params.push(("trackid", track_id.to_owned()));
        self.get("lookup", params).await
    }

    /// Lists AcoustID tracks associated with a MusicBrainz recording UUID.
    /// This endpoint does not require an application API key.
    pub async fn list_by_mbid(&self, mbid: &str) -> Result<ApiResponse, AcoustidError> {
        self.get("track/list_by_mbid", vec![("mbid", mbid.to_owned())])
            .await
    }

    fn lookup_params(
        &self,
        metadata: &[Metadata],
    ) -> Result<Vec<(&'static str, String)>, AcoustidError> {
        let application_key = self
            .application_key
            .as_deref()
            .ok_or(AcoustidError::MissingApplicationKey)?;
        let mut params = vec![("client", application_key.to_owned())];
        if !metadata.is_empty() {
            params.push((
                "meta",
                metadata
                    .iter()
                    .map(|item| item.as_str())
                    .collect::<Vec<_>>()
                    .join("+"),
            ));
        }
        Ok(params)
    }

    async fn get(
        &self,
        endpoint: &str,
        mut params: Vec<(&'static str, String)>,
    ) -> Result<ApiResponse, AcoustidError> {
        params.push(("format", "json".to_owned()));
        self.limiter.until_ready().await;

        let response = self
            .http
            .get(format!("{API_BASE}/{endpoint}"))
            .query(&params)
            .send()
            .await?;
        let status = response.status();
        let body = response.bytes().await?;
        if !status.is_success() {
            return Err(AcoustidError::HttpStatus {
                status,
                body: String::from_utf8_lossy(&body).into_owned(),
            });
        }

        let result: ApiResponse = serde_json::from_slice(&body)?;
        if result.status != "ok" {
            return Err(AcoustidError::Api {
                status: result.status.clone(),
                error: result.error.clone(),
            });
        }
        Ok(result)
    }
}

impl Default for AcoustIDClient {
    fn default() -> Self {
        Self::new().expect("create AcoustID HTTP client")
    }
}

/// Metadata fields supported by the AcoustID lookup endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Metadata {
    Recordings,
    RecordingIds,
    Releases,
    ReleaseIds,
    ReleaseGroups,
    ReleaseGroupIds,
    Tracks,
    Compress,
    UserMeta,
    Sources,
    Isrcs,
}

impl Metadata {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Recordings => "recordings",
            Self::RecordingIds => "recordingids",
            Self::Releases => "releases",
            Self::ReleaseIds => "releaseids",
            Self::ReleaseGroups => "releasegroups",
            Self::ReleaseGroupIds => "releasegroupids",
            Self::Tracks => "tracks",
            Self::Compress => "compress",
            Self::UserMeta => "usermeta",
            Self::Sources => "sources",
            Self::Isrcs => "isrcs",
        }
    }
}

/// A successfully parsed AcoustID API response.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ApiResponse {
    pub status: String,
    #[serde(default)]
    pub results: Vec<LookupResult>,
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub mbid: Option<String>,
    #[serde(default)]
    pub error: Option<ApiErrorPayload>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// An AcoustID track lookup match.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LookupResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub recordings: Vec<Recording>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A MusicBrainz recording associated with a lookup result.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Recording {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub duration: Option<u32>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub releasegroups: Vec<ReleaseGroup>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A MusicBrainz artist.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Artist {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A MusicBrainz release group.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ReleaseGroup {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(rename = "type", default)]
    pub release_type: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// A track returned by `list_by_mbid`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Error payload returned by the AcoustID API.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ApiErrorPayload {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub code: Option<Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Errors returned by the client.
#[derive(Debug, thiserror::Error)]
pub enum AcoustidError {
    #[error("this operation requires an AcoustID application key")]
    MissingApplicationKey,
    #[error("AcoustID returned HTTP status {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
    #[error("AcoustID API returned status '{status}': {error:?}")]
    Api {
        status: String,
        error: Option<ApiErrorPayload>,
    },
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid JSON returned by AcoustID: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_without_key_fails_before_networking() {
        let client = AcoustIDClient::new().unwrap();
        let error = tokio_test_block_on(client.lookup_by_track_id("track", &[])).unwrap_err();
        assert!(matches!(error, AcoustidError::MissingApplicationKey));
    }

    #[test]
    fn parses_lookup_and_mbid_response_shapes() {
        let lookup: ApiResponse = serde_json::from_str(
            r#"{"status":"ok","results":[{"id":"track","score":0.95,"recordings":[{"id":"recording","artists":[{"name":"Artist"}]}]}]}"#,
        )
        .unwrap();
        assert_eq!(lookup.results[0].id.as_deref(), Some("track"));
        assert_eq!(
            lookup.results[0].recordings[0].artists[0].name.as_deref(),
            Some("Artist")
        );

        let mbid: ApiResponse =
            serde_json::from_str(r#"{"status":"ok","tracks":[{"id":"track"}],"mbid":"recording"}"#)
                .unwrap();
        assert_eq!(mbid.tracks[0].id, "track");
        assert_eq!(mbid.mbid.as_deref(), Some("recording"));
    }

    #[test]
    fn metadata_values_follow_api_names() {
        assert_eq!(Metadata::RecordingIds.as_str(), "recordingids");
        assert_eq!(Metadata::ReleaseGroupIds.as_str(), "releasegroupids");
        assert_eq!(Metadata::UserMeta.as_str(), "usermeta");
    }

    #[test]
    fn api_error_response_is_parsed() {
        let response: ApiResponse = serde_json::from_str(
            r#"{"status":"error","error":{"message":"invalid request","code":4}}"#,
        )
        .unwrap();
        assert_eq!(
            response.error.unwrap().message.as_deref(),
            Some("invalid request")
        );
    }

    fn tokio_test_block_on<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }
}
