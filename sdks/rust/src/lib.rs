//! SXAPI v1 player facade with header-only credentials and bounded responses.
#![forbid(unsafe_code)]

mod models;
pub use models::*;
mod identity;
pub use identity::*;
mod connecting;
pub use connecting::*;

use chrono::{DateTime, Datelike, SecondsFormat, Utc};
use reqwest::{header::{self, HeaderValue}, Method, RequestBuilder, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, sync::Arc, time::{Duration, SystemTime}};
use uuid::Uuid;
use zeroize::Zeroizing;

const JSON_CAP: usize = 2 * 1024 * 1024;
const ERROR_CAP: usize = 16 * 1024;
const IMAGE_CAP: usize = 16 * 1024 * 1024;
const RANGE_CAP: u64 = 16 * 1024 * 1024;
const SUBTITLE_CAP: usize = 2 * 1024 * 1024;
const VIDEO_HEADER: &str = "X-SXAPI-Stream-Ticket";
const SUBTITLE_HEADER: &str = "X-SXAPI-Subtitle-Ticket";

/// An owned secret with redacted Debug and best-effort zeroizing storage.
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Take ownership of a private value without exposing it through Debug.
    pub fn new(value: String) -> Self { Self(Zeroizing::new(value)) }
    /// Read the value only to move it into protected storage, such as a refresh token.
    ///
    /// Never log, display or put it in a URL.
    pub fn expose_secret(&self) -> &str { self.text() }
    fn text(&self) -> &str { self.0.as_str() }
    fn header(&self) -> Result<HeaderValue, Error> {
        let mut value = HeaderValue::from_str(self.text()).map_err(|_| Error::InvalidCredential)?;
        value.set_sensitive(true);
        Ok(value)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("Secret([REDACTED])") }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

/// Safe client errors: never contain raw URLs, headers, server bodies or tokens.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidOrigin, InvalidPath, InvalidCredential, UnsupportedHeaderContract,
    UnsupportedFeature, InvalidInput, InvalidResponse, ResponseTooLarge,
    Transport, RepresentationChanged, RangeMismatch, SessionEnded,
    CleanupFailed, Http { status: u16, code: Option<String>, retry_after: Option<RetryAfter> },
    /// `GET /media/removed` answered 410 `removed_history_unavailable`.
    ///
    /// `since` is before the complete history, which starts at `tracked_since` (`None` before
    /// the server's first sweep). Reconcile with `GET /media/ids`, then continue from that
    /// response's time.
    RemovedHistoryUnavailable { tracked_since: Option<DateTime<Utc>> },
}

/// Validated server backoff metadata, without retaining the raw header text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryAfter {
    /// Nonnegative whole seconds; no automatic retry or clamping is performed.
    DelaySeconds(u64),
    /// Parsed HTTP date; the caller compares this deadline with its clock.
    HttpDate(SystemTime),
}

impl Error {
    /// Inspect a valid, unambiguous Retry-After value without replaying a request.
    pub fn retry_after(&self) -> Option<&RetryAfter> {
        match self { Self::Http { retry_after, .. } => retry_after.as_ref(), _ => None }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { status, code, .. } => write!(f, "SXAPI HTTP {status} ({})", code.as_deref().unwrap_or("unclassified")),
            other => write!(f, "StreamixKit {other:?}"),
        }
    }
}
impl std::error::Error for Error {}

/// Transport pinned to one approved origin; HTTPS default, explicit local HTTP.
#[derive(Clone)]
pub struct Client { inner: Arc<Transport> }
type TraceObserver = Arc<dyn Fn(&RequestTrace) + Send + Sync>;
struct Transport { origin: Url, http: reqwest::Client, identity: RequestIdentity, observer: Option<TraceObserver> }

impl Client {
    /// Construct an origin-only client; API prefixes, userinfo and query are invalid.
    pub fn new(origin: &str, identity: RequestIdentity) -> Result<Self, Error> { Self::build(origin, identity, false) }

    /// Apply an explicit protocol choice and standard ports without any fallback.
    pub fn with_address(address: &str, identity: RequestIdentity, mode: ConnectionMode) -> Result<Self, Error> {
        let parsed = ServerAddress::parse(address,mode)?;
        Self::build(parsed.origin(),identity,matches!(mode,ConnectionMode::LocalHttp))
    }

    fn build(origin: &str, identity: RequestIdentity, explicitly_allow_http: bool) -> Result<Self, Error> {
        let origin = Url::parse(origin).map_err(|_| Error::InvalidOrigin)?;
        let chosen_http = explicitly_allow_http && origin.scheme() == "http";
        if (origin.scheme() != "https" && !chosen_http) || origin.host_str().is_none()
            || !origin.username().is_empty() || origin.password().is_some()
            || origin.query().is_some() || origin.fragment().is_some() || origin.path() != "/" {
            return Err(Error::InvalidOrigin);
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none()).no_proxy()
            .connect_timeout(Duration::from_secs(10)).read_timeout(Duration::from_secs(30))
            .build().map_err(|_| Error::Transport)?;
        Ok(Self { inner: Arc::new(Transport { origin, http, identity, observer:None }) })
    }

    /// Observe method/purpose/trace only; no automatic logging or request mutation.
    pub fn with_request_observer(&self, observer: impl Fn(&RequestTrace) + Send + Sync + 'static) -> Self {
        Self { inner:Arc::new(Transport { origin:self.inner.origin.clone(),http:self.inner.http.clone(),identity:self.inner.identity.clone(),observer:Some(Arc::new(observer)) }) }
    }

    /// Read current discovery; it is not a user authorization grant.
    pub async fn capabilities(&self) -> Result<Capabilities, Error> {
        self.inner.json(self.inner.api(Method::GET, "/capabilities")?).await
    }

    /// Login using the password body only after explicit safe-transport discovery.
    ///
    /// Since server 0.9.0-rc.27 (`features.session_refresh`) the login also carries its
    /// expiry and a rotating refresh token; see `AuthenticatedClient::renewal`.
    pub async fn login(&self, username: &str, password: Secret) -> Result<AuthenticatedClient, Error> {
        let capabilities = self.capabilities().await?;
        require_contract(&capabilities)?;
        #[derive(Serialize)] struct Login<'a> { username: &'a str, password: &'a str }
        let request = self.inner.api(Method::POST, "/sessions")?.json(&Login { username, password: password.text() });
        let login: LoginWire = self.inner.json(request).await?;
        check_access(&login.access_token, &login.token_type)?;
        let renewal = pair_renewal(login.refresh_token, login.refresh_expires_at)?;
        Ok(AuthenticatedClient { client: self.clone(), token: login.access_token, user: login.user, features:capabilities.features, expires_at: login.expires_at, renewal })
    }

    /// Exchange a refresh secret for a new login (`POST /sessions/refresh`) without using it yet.
    ///
    /// A personal credential secret (`stx_sxrt_`) is not rotated. A login refresh token
    /// (`stx_sxls_` from `login` or an earlier refresh, server 0.9.0-rc.27
    /// `features.session_refresh`) is spent: the result's `renewal` holds the next token of
    /// its chain. Store that token before `RefreshedLogin::activate` uses the new access token,
    /// and refresh one request at a time. Malformed secrets are rejected before any request.
    pub async fn refresh(&self, refresh_secret: Secret) -> Result<RefreshedLogin, Error> {
        let kind = refresh_kind(refresh_secret.text()).ok_or(Error::InvalidCredential)?;
        let capabilities = self.capabilities().await?;
        require_contract(&capabilities)?;
        if kind == RefreshKind::Login && capabilities.features.get("session_refresh") != Some(&true) { return Err(Error::UnsupportedFeature); }
        #[derive(Serialize)] struct Refresh<'a> { refresh_token: &'a str }
        let body = Refresh { refresh_token: refresh_secret.text() };
        let wire: RefreshWire = self.inner.json(self.inner.api(Method::POST, "/sessions/refresh")?.json(&body)).await?;
        check_access(&wire.access_token, &wire.token_type)?;
        // The 200 body is one of two closed shapes: SessionRefreshed carries the next refresh
        // token, OwnCredentialRefreshed the credential ID. It must answer the kind sent.
        let renewal = pair_renewal(wire.refresh_token, wire.refresh_expires_at)?;
        let answered = match kind {
            RefreshKind::Personal => renewal.is_none(),
            RefreshKind::Login => renewal.is_some() && wire.credential_id.is_none() && wire.expires_at.is_some(),
        };
        if !answered { return Err(Error::InvalidResponse); }
        Ok(RefreshedLogin { client: self.clone(), token: wire.access_token, expires_at: wire.expires_at, renewal, features: capabilities.features })
    }
}

#[derive(Deserialize)]
struct LoginWire {
    access_token: Secret, token_type: String, user: User,
    // Server 0.9.0-rc.27 and later; absent from earlier servers.
    #[serde(default)] expires_at: Option<DateTime<Utc>>,
    #[serde(default)] refresh_token: Option<Secret>,
    #[serde(default)] refresh_expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct RefreshWire {
    access_token: Secret, token_type: String,
    #[serde(default)] expires_at: Option<DateTime<Utc>>,
    #[serde(default)] credential_id: Option<Uuid>,
    #[serde(default)] refresh_token: Option<Secret>,
    #[serde(default)] refresh_expires_at: Option<DateTime<Utc>>,
}

/// The rotating refresh token of a password login (server 0.9.0-rc.27).
///
/// Offered with `features.session_refresh`. Keep it in protected storage; each
/// `Client::refresh` spends it and returns the next one. Debug never shows the token.
#[derive(Debug)]
pub struct LoginRenewal {
    /// The `stx_sxls_` token for the next `Client::refresh`.
    pub refresh_token: Secret,
    /// When the token expires unless it is used first.
    ///
    /// 30 days, renewed by each refresh, never more than 365 days after the password login.
    pub refresh_expires_at: DateTime<Utc>,
}

/// A login from `Client::refresh` whose new access token has not been used yet.
pub struct RefreshedLogin { client: Client, token: Secret, expires_at: Option<DateTime<Utc>>, renewal: Option<LoginRenewal>, features: BTreeMap<String,bool> }

impl RefreshedLogin {
    /// The next refresh token after a login refresh (`stx_sxls_`).
    ///
    /// `None` after a personal secret, which is not rotated. Store it before `activate`:
    /// the token sent is spent.
    pub fn renewal(&self) -> Option<&LoginRenewal> { self.renewal.as_ref() }
    /// When the new access token expires (always present after a login refresh).
    pub fn expires_at(&self) -> Option<DateTime<Utc>> { self.expires_at }
    /// Read `/users/me` with the new access token and return the login-scoped client.
    ///
    /// The client keeps `renewal` and `expires_at`.
    pub async fn activate(self) -> Result<AuthenticatedClient, Error> {
        let mut client = AuthenticatedClient { client: self.client, token: self.token, user: User { id: Uuid::nil(), username: String::new(), library_ids: Vec::new(), extra: Default::default() }, features: self.features, expires_at: self.expires_at, renewal: self.renewal };
        client.user = client.json(Method::GET, "/users/me").await?;
        Ok(client)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RefreshKind { Personal, Login }

/// Classify `^stx_sx(rt|ls)_[0-9a-f]{64}$`, the only refresh secrets the server accepts.
fn refresh_kind(value: &str) -> Option<RefreshKind> {
    let (kind, digest) = match value.strip_prefix("stx_sxls_") {
        Some(rest) => (RefreshKind::Login, rest),
        None => (RefreshKind::Personal, value.strip_prefix("stx_sxrt_")?),
    };
    (digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())).then_some(kind)
}

/// A rotating token and its expiry come together or not at all.
fn pair_renewal(token: Option<Secret>, expires_at: Option<DateTime<Utc>>) -> Result<Option<LoginRenewal>, Error> {
    match (token, expires_at) {
        (None, None) => Ok(None),
        (Some(token), Some(expires_at)) if refresh_kind(token.text()) == Some(RefreshKind::Login) => Ok(Some(LoginRenewal { refresh_token: token, refresh_expires_at: expires_at })),
        _ => Err(Error::InvalidResponse),
    }
}

fn check_access(token: &Secret, token_type: &str) -> Result<(), Error> {
    if token_type != "Bearer" || !token.text().starts_with("stx_sx_") || token.text().len() > 2048 { return Err(Error::InvalidResponse); }
    token.header()?;
    Ok(())
}

fn require_contract(value: &Capabilities) -> Result<(), Error> {
    if value.protocol != "SXAPI" || value.version != "1.0"
        || value.features.get("playback_ticket_header") != Some(&true)
        || value.features.get("credentials_in_urls") != Some(&false) { return Err(Error::UnsupportedHeaderContract); }
    Ok(())
}

impl Transport {
    fn relative(&self, path: &str) -> Result<Url, Error> {
        // Only resource characters used by this public contract. This also rejects
        // encoded traversal/separators, control bytes, credentials and redirects.
        if !path.starts_with('/') || path.starts_with("//") || path.contains("..")
            || !path.bytes().all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b)) {
            return Err(Error::InvalidPath);
        }
        let url = self.origin.join(path).map_err(|_| Error::InvalidPath)?;
        if url.origin() != self.origin.origin() || url.query().is_some() || url.fragment().is_some() { return Err(Error::InvalidPath); }
        Ok(url)
    }
    fn api(&self, method: Method, suffix: &str) -> Result<RequestBuilder, Error> {
        Ok(self.request(method, &format!("/sxapi/v1{suffix}"), RequestPurpose::Interaction)?.header(header::ACCEPT, "application/json"))
    }
    fn request(&self, method: Method, path: &str, purpose: RequestPurpose) -> Result<RequestBuilder, Error> {
        let url = self.relative(path)?;
        let headers = self.identity.headers(purpose);
        if let (Some(observer),Some(traceparent)) = (&self.observer,headers.get("traceparent").and_then(|value|value.to_str().ok())) {
            observer(&RequestTrace { method:method.as_str().to_owned(),purpose,traceparent:traceparent.to_owned() });
        }
        Ok(self.http.request(method, url).headers(headers))
    }
    async fn json<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T, Error> {
        let response = request.send().await.map_err(|_| Error::Transport)?;
        let response = success(response).await?;
        let raw = bounded(response, JSON_CAP).await?;
        serde_json::from_slice(&raw).map_err(|_| Error::InvalidResponse)
    }
}

/// Original-login scoped API client; Debug never exposes its Bearer credential.
pub struct AuthenticatedClient { client: Client, token: Secret, user: User, features:BTreeMap<String,bool>, expires_at: Option<DateTime<Utc>>, renewal: Option<LoginRenewal> }

impl AuthenticatedClient {
    /// Inspect the non-secret identity returned by login.
    pub fn user(&self) -> &User { &self.user }
    /// When this login's access token expires, when the server says (0.9.0-rc.27 and later).
    pub fn expires_at(&self) -> Option<DateTime<Utc>> { self.expires_at }
    /// The rotating refresh token that came with this login, if the server offers one.
    ///
    /// Offered with `features.session_refresh`. Store it; `Client::refresh` exchanges it
    /// for a new login.
    pub fn renewal(&self) -> Option<&LoginRenewal> { self.renewal.as_ref() }
    fn request(&self, method: Method, suffix: &str) -> Result<RequestBuilder, Error> {
        self.request_for(method,suffix,RequestPurpose::Interaction)
    }
    fn request_for(&self, method: Method, suffix: &str, purpose: RequestPurpose) -> Result<RequestBuilder, Error> {
        Ok(self.client.inner.request(method,&format!("/sxapi/v1{suffix}"),purpose)?.header(header::ACCEPT,"application/json").header(header::AUTHORIZATION,self.bearer()?))
    }
    fn bearer(&self) -> Result<HeaderValue, Error> {
        Secret::new(format!("Bearer {}", self.token.text())).header()
    }
    async fn json<T: DeserializeOwned>(&self, method: Method, suffix: &str) -> Result<T, Error> {
        self.client.inner.json(self.request(method, suffix)?).await
    }
    /// List only libraries visible to this current account.
    pub async fn libraries(&self) -> Result<Vec<Library>, Error> { self.json(Method::GET, "/libraries").await }

    /// Fetch one media page without copying or interpreting its opaque cursor.
    pub async fn media(&self, library: Uuid, search: Option<&str>, cursor: Option<&str>, limit: u16) -> Result<Page<Media>, Error> {
        self.media_with_purpose(library,search,cursor,limit,RequestPurpose::Interaction).await
    }

    /// Explicit background browsing changes only Sec-Purpose, never authorization.
    pub async fn media_with_purpose(&self, library: Uuid, search: Option<&str>, cursor: Option<&str>, limit: u16, purpose: RequestPurpose) -> Result<Page<Media>, Error> {
        if !(1..=100).contains(&limit) { return Err(Error::InvalidInput); }
        let mut query = vec![("library_id", library.to_string()), ("limit", limit.to_string())];
        if let Some(value) = search { query.push(("search", value.to_owned())); }
        if let Some(value) = cursor { query.push(("cursor", value.to_owned())); }
        self.client.inner.json(self.request_for(Method::GET, "/media",purpose)?.query(&query)).await
    }

    /// One page of works removed at or after `since`, oldest first (`GET /media/removed`).
    ///
    /// Requires `features.media_removed` (server 0.9.0-rc.27). Pass `next_cursor` back
    /// unchanged with the same `since`, and ignore IDs you do not hold. A `since` before the
    /// complete history is `Error::RemovedHistoryUnavailable`.
    pub async fn removed_media(&self, since: DateTime<Utc>, cursor: Option<&str>, limit: u16) -> Result<MediaRemovalPage, Error> {
        if self.features.get("media_removed") != Some(&true) { return Err(Error::UnsupportedFeature); }
        if !(1..=5000).contains(&limit) || !(1..=9999).contains(&since.year())
            || cursor.is_some_and(|value| value.is_empty() || value.len() > 4096) { return Err(Error::InvalidInput); }
        // The server compares microseconds; truncating keeps the inclusive bound inclusive.
        let mut query = vec![("since", since.to_rfc3339_opts(SecondsFormat::Micros, true)), ("limit", limit.to_string())];
        if let Some(value) = cursor { query.push(("cursor", value.to_owned())); }
        let response = self.request(Method::GET, "/media/removed")?.query(&query).send().await.map_err(|_| Error::Transport)?;
        if response.status() == StatusCode::GONE { return Err(removed_history_error(response).await); }
        let raw = bounded(success(response).await?, JSON_CAP).await?;
        serde_json::from_slice(&raw).map_err(|_| Error::InvalidResponse)
    }

    /// Fetch exact actual versions for one movie or episode.
    pub async fn versions(&self, media: Uuid, episode: Option<Uuid>, cursor: Option<&str>, limit: u16) -> Result<Page<Version>, Error> {
        if !(1..=100).contains(&limit) { return Err(Error::InvalidInput); }
        let mut query = vec![("limit", limit.to_string())];
        if let Some(value) = episode { query.push(("episode_id", value.to_string())); }
        if let Some(value) = cursor { query.push(("cursor", value.to_owned())); }
        self.client.inner.json(self.request(Method::GET, &format!("/media/{media}/versions"))?.query(&query)).await
    }

    /// List the exact selected version's external subtitles without source paths.
    pub async fn subtitles(&self, media: Uuid, episode: Option<Uuid>, version: Uuid) -> Result<SubtitleInventory, Error> {
        let capabilities = self.client.capabilities().await?;
        require_contract(&capabilities)?;
        if capabilities.features.get("subtitles") != Some(&true) { return Err(Error::UnsupportedFeature); }
        let mut query = vec![("version_id", version.to_string())];
        if let Some(value) = episode { query.push(("episode_id", value.to_string())); }
        self.client.inner.json(self.request(Method::GET, &format!("/media/{media}/subtitles"))?.query(&query)).await
    }

    /// Prepare a typed playback handle. A rejected grant is ended if its ID is known.
    pub async fn prepare(&self, input: &PlaybackRequest) -> Result<Playback<'_>, Error> {
        let capabilities = self.client.capabilities().await?;
        require_contract(&capabilities)?;
        if capabilities.features.get("playback") != Some(&true) { return Err(Error::UnsupportedFeature); }
        #[derive(Deserialize)] struct Prepared { session: PlaybackSession, #[serde(default)] ticket: Value }
        let wire: Prepared = self.client.inner.json(self.request(Method::POST, "/playback/sessions")?.json(input)).await?;
        let grant = serde_json::from_value::<VideoGrant>(wire.ticket).map_err(|_| Error::InvalidResponse)
            .and_then(|grant| { grant.validate(&wire.session)?; Ok(grant) });
        match grant {
            Ok(grant) if wire.session.mode == "direct_play" => Ok(Playback { login: self, session: wire.session, grant, ended: false }),
            result => { self.end_id(wire.session.session_id).await.map_err(|_| Error::CleanupFailed)?; Err(result.err().unwrap_or(Error::InvalidResponse)) }
        }
    }

    async fn end_id(&self, id: Uuid) -> Result<(), Error> {
        let response = self.request(Method::DELETE, &format!("/playback/sessions/{id}"))?.send().await.map_err(|_| Error::Transport)?;
        require_no_content(response).await
    }

    /// Fetch a protected SXAPI image; never accept arbitrary provider origins.
    pub async fn image(&self, path: &str, cap: usize) -> Result<Vec<u8>, Error> {
        // Compatibility discovery from login avoids one extra round trip per
        // cover. The image endpoint independently rechecks live authorization.
        if self.features.get("images") != Some(&true) { return Err(Error::UnsupportedFeature); }
        let parts: Vec<_> = path.split('/').collect();
        let person = parts.len() == 7 && parts[..4] == ["", "sxapi", "v1", "persons"]
            && Uuid::parse_str(parts[4]).is_ok() && parts[5] == "images" && parts[6] == "profile";
        let media_or_library = (path.starts_with("/sxapi/v1/media/") || path.starts_with("/sxapi/v1/libraries/")) && path.contains("/images/");
        if !(person || media_or_library) || cap == 0 || cap > IMAGE_CAP { return Err(Error::InvalidPath); }
        self.image_bytes(path, &[], cap).await
    }

    /// Fetch a work's logo, optionally in a chosen language.
    ///
    /// `language` picks one of `Media.logos` by its `lang`, or takes any BCP 47 tag such as
    /// `zh-Hant`, on servers with `features.logo_languages` (0.9.0-rc.27); `None` lets the
    /// request's Accept-Language decide.
    pub async fn logo(&self, media: Uuid, language: Option<&str>, cap: usize) -> Result<Vec<u8>, Error> {
        if self.features.get("images") != Some(&true) { return Err(Error::UnsupportedFeature); }
        let mut query = Vec::new();
        if let Some(value) = language {
            if self.features.get("logo_languages") != Some(&true) { return Err(Error::UnsupportedFeature); }
            if !(1..=35).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') { return Err(Error::InvalidInput); }
            query.push(("language", value));
        }
        if cap == 0 || cap > IMAGE_CAP { return Err(Error::InvalidInput); }
        self.image_bytes(&format!("/sxapi/v1/media/{media}/images/logo"), &query, cap).await
    }

    async fn image_bytes(&self, path: &str, query: &[(&str, &str)], cap: usize) -> Result<Vec<u8>, Error> {
        let mut request = self.client.inner.request(Method::GET,path,RequestPurpose::Interaction)?.header(header::AUTHORIZATION, self.bearer()?);
        if !query.is_empty() { request = request.query(query); }
        let response = success(request.send().await.map_err(|_| Error::Transport)?).await?;
        let content_type = response.headers().get(header::CONTENT_TYPE).and_then(|x| x.to_str().ok()).unwrap_or("");
        if !["image/jpeg", "image/png", "image/webp", "image/avif"].contains(&content_type.split(';').next().unwrap_or("")) { return Err(Error::InvalidResponse); }
        bounded(response, cap).await
    }

    /// Revoke the login used here. End owned playback handles first.
    pub async fn logout(&self) -> Result<(), Error> {
        let response = self.request(Method::DELETE, "/sessions/current")?.send().await.map_err(|_| Error::Transport)?;
        require_no_content(response).await
    }
}

#[derive(Debug, Deserialize)]
struct VideoGrant { stream_path: String, ticket: Secret, header_name: String, expires_at: DateTime<Utc> }
impl VideoGrant {
    fn validate(&self, session: &PlaybackSession) -> Result<(), Error> {
        let local = format!("/sxapi/v1/playback/sessions/{}/stream", session.session_id);
        if self.header_name != VIDEO_HEADER || !(self.stream_path == local || self.stream_path == "/nimbus-play/v1/sxapi")
            || self.ticket.text().is_empty() || self.ticket.text().len() > 4096
            || self.expires_at <= Utc::now() || self.expires_at > session.expires_at { return Err(Error::InvalidResponse); }
        self.ticket.header()?;
        Ok(())
    }
}

/// Owned protocol session and current admission ticket; explicitly await `end`.
pub struct Playback<'a> { login: &'a AuthenticatedClient, session: PlaybackSession, grant: VideoGrant, ended: bool }

impl Playback<'_> {
    /// Inspect the non-secret selected session and original-file mode.
    pub fn session(&self) -> &PlaybackSession { &self.session }
    /// Refresh the grant through the original login; no automatic mutation retry.
    pub async fn renew(&mut self) -> Result<(), Error> {
        self.renew_for(RequestPurpose::Interaction).await
    }
    async fn renew_for(&mut self, purpose: RequestPurpose) -> Result<(), Error> {
        if self.ended { return Err(Error::SessionEnded); }
        let request = self.login.request_for(Method::POST, &format!("/playback/sessions/{}/tickets", self.session.session_id),purpose)?;
        let grant: VideoGrant = self.login.client.inner.json(request).await?;
        grant.validate(&self.session)?;
        self.grant = grant;
        Ok(())
    }
    async fn current_grant(&mut self, purpose: RequestPurpose) -> Result<(), Error> {
        if self.ended { return Err(Error::SessionEnded); }
        if self.grant.expires_at <= Utc::now() + chrono::Duration::seconds(15) { self.renew_for(purpose).await?; }
        Ok(())
    }
    /// Read exactly one bounded range. A 200 fallback is never appended to fragments.
    pub async fn read_range(&mut self, start: u64, end: u64, if_range: Option<&str>) -> Result<RangeData, Error> {
        self.read_range_with_purpose(start,end,if_range,RequestPurpose::Interaction).await
    }

    /// Mark an explicitly requested prefetch without changing UA, grants or quota.
    pub async fn read_range_with_purpose(&mut self, start: u64, end: u64, if_range: Option<&str>, purpose: RequestPurpose) -> Result<RangeData, Error> {
        let length = end.checked_sub(start).and_then(|n| n.checked_add(1)).ok_or(Error::InvalidInput)?;
        if length > RANGE_CAP { return Err(Error::InvalidInput); }
        self.current_grant(purpose).await?;
        let inner = &self.login.client.inner;
        let mut request = inner.request(Method::GET,&self.grant.stream_path,purpose)?
            .header(VIDEO_HEADER, self.grant.ticket.header()?).header(header::RANGE, format!("bytes={start}-{end}"))
            .header(header::ACCEPT_ENCODING, "identity");
        if let Some(value) = if_range {
            if !strong_etag(value) { return Err(Error::InvalidInput); }
            request = request.header(header::IF_RANGE, value);
        }
        let response = request.send().await.map_err(|_| Error::Transport)?;
        if response.status() == StatusCode::OK { return Err(Error::RepresentationChanged); }
        let response = success(response).await?;
        if response.status() != StatusCode::PARTIAL_CONTENT { return Err(Error::RangeMismatch); }
        if response.headers().get(header::CONTENT_ENCODING).is_some_and(|encoding|encoding != "identity") { return Err(Error::InvalidResponse); }
        let raw_range = response.headers().get(header::CONTENT_RANGE).and_then(|value| value.to_str().ok()).ok_or(Error::RangeMismatch)?;
        let (actual_start, actual_end, total) = parse_range(raw_range)?;
        if actual_start != start || actual_end > end || actual_end >= total || actual_start > actual_end { return Err(Error::RangeMismatch); }
        // Short final ranges are valid only when they end at the actual EOF.
        if actual_end != end && actual_end + 1 != total { return Err(Error::RangeMismatch); }
        let etag = response.headers().get(header::ETAG).and_then(|value| value.to_str().ok()).filter(|value| strong_etag(value)).map(str::to_owned);
        let bytes = bounded(response, length as usize).await?;
        if bytes.len() as u64 != actual_end - actual_start + 1 { return Err(Error::RangeMismatch); }
        Ok(RangeData { bytes, start, end: actual_end, total_length: total, strong_etag: etag })
    }
    /// Recheck admission headers without downloading a video body.
    pub async fn head(&mut self) -> Result<(), Error> {
        self.current_grant(RequestPurpose::Interaction).await?;
        let inner = &self.login.client.inner;
        let response = inner.request(Method::HEAD,&self.grant.stream_path,RequestPurpose::Interaction)?.header(VIDEO_HEADER, self.grant.ticket.header()?).send().await.map_err(|_| Error::Transport)?;
        let response = success(response).await?;
        if [StatusCode::OK,StatusCode::PARTIAL_CONTENT].contains(&response.status()) { Ok(()) } else { Err(Error::InvalidResponse) }
    }
    /// Report an actual player position with an explicit increasing sequence.
    pub async fn progress(&self, report: &Progress) -> Result<ProgressAck, Error> {
        if self.ended { return Err(Error::SessionEnded); }
        if report.sequence < 1 || !(0..=31_536_000_000).contains(&report.position_ms) { return Err(Error::InvalidInput); }
        self.login.client.inner.json(self.login.request(Method::PUT, &format!("/playback/sessions/{}/progress", self.session.session_id))?.json(report)).await
    }
    /// Read a selected external subtitle using both original Bearer and ticket.
    pub async fn subtitle(&self, subtitle_id: &str) -> Result<Vec<u8>, Error> {
        if self.ended { return Err(Error::SessionEnded); }
        if subtitle_id.len() != 64 || !subtitle_id.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) { return Err(Error::InvalidInput); }
        let capabilities = self.login.client.capabilities().await?;
        require_contract(&capabilities)?;
        if capabilities.features.get("subtitles") != Some(&true)
            || capabilities.features.get("subtitle_ticket_header") != Some(&true) { return Err(Error::UnsupportedFeature); }
        #[derive(Deserialize)] struct SubtitleGrant { ticket: Secret, expires_at: DateTime<Utc>, read_path: String, header_name: String }
        let grant: SubtitleGrant = self.login.json(Method::POST, &format!("/playback/sessions/{}/subtitles/{subtitle_id}/tickets", self.session.session_id)).await?;
        let local = format!("/sxapi/v1/playback/sessions/{}/subtitles/{subtitle_id}", self.session.session_id);
        if grant.header_name != SUBTITLE_HEADER || !(grant.read_path == local || grant.read_path == "/nimbus-play/v1/sxapi-subtitle")
            || grant.ticket.text().is_empty() || grant.ticket.text().len() > 1024
            || grant.expires_at <= Utc::now() || grant.expires_at > self.session.expires_at { return Err(Error::InvalidResponse); }
        let inner = &self.login.client.inner;
        let response = inner.request(Method::GET,&grant.read_path,RequestPurpose::Interaction)?.header(SUBTITLE_HEADER, grant.ticket.header()?)
            .header(header::AUTHORIZATION, self.login.bearer()?).send().await.map_err(|_| Error::Transport)?;
        bounded(success(response).await?, SUBTITLE_CAP).await
    }
    /// End on every path; local ended state changes only after actual HTTP 204.
    pub async fn end(&mut self) -> Result<(), Error> {
        if !self.ended { self.login.end_id(self.session.session_id).await?; self.ended = true; }
        Ok(())
    }
}

fn strong_etag(value: &str) -> bool {
    value.len() >= 2 && value.len() <= 512 && value.starts_with('"') && value.ends_with('"')
        && value[1..value.len()-1].bytes().all(|b| (0x21..=0x7e).contains(&b) && b != b'"')
}

fn parse_range(value: &str) -> Result<(u64, u64, u64), Error> {
    let value = value.strip_prefix("bytes ").ok_or(Error::RangeMismatch)?;
    let (range, total) = value.split_once('/').ok_or(Error::RangeMismatch)?;
    let (start, end) = range.split_once('-').ok_or(Error::RangeMismatch)?;
    Ok((start.parse().map_err(|_| Error::RangeMismatch)?, end.parse().map_err(|_| Error::RangeMismatch)?, total.parse().map_err(|_| Error::RangeMismatch)?))
}

async fn require_no_content(response: Response) -> Result<(), Error> {
    let response = success(response).await?;
    if response.status() == StatusCode::NO_CONTENT { Ok(()) } else { Err(Error::InvalidResponse) }
}

async fn success(response: Response) -> Result<Response, Error> {
    if response.status().is_success() { return Ok(response); }
    let status = response.status().as_u16();
    let retry_after = parse_retry_after(response.headers());
    let raw = bounded(response, ERROR_CAP).await.ok();
    Err(http_error(status, retry_after, raw.as_deref()))
}

/// Map 410 `removed_history_unavailable` to its typed error; any other 410 stays an HTTP error.
async fn removed_history_error(response: Response) -> Error {
    #[derive(Deserialize)] struct Gone { error: GoneCode, tracked_since: Option<DateTime<Utc>> }
    #[derive(Deserialize)] struct GoneCode { code: String }
    let retry_after = parse_retry_after(response.headers());
    let raw = bounded(response, ERROR_CAP).await.ok();
    match raw.as_deref().and_then(|raw| serde_json::from_slice::<Gone>(raw).ok()) {
        Some(gone) if gone.error.code == "removed_history_unavailable" => Error::RemovedHistoryUnavailable { tracked_since: gone.tracked_since },
        _ => http_error(StatusCode::GONE.as_u16(), retry_after, raw.as_deref()),
    }
}

/// Keep only a bounded, credential-free `error.code`; never the body or its message.
fn http_error(status: u16, retry_after: Option<RetryAfter>, raw: Option<&[u8]>) -> Error {
    let code = raw.and_then(|raw| serde_json::from_slice::<Value>(raw).ok())
        .and_then(|value| value.get("error")?.get("code")?.as_str().map(str::to_owned))
        .filter(|value| {
            // A credential-shaped code (stx_ prefix or a 64-hex digest) is never kept.
            let secret_like = value.starts_with("stx_") || (value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()));
            (1..=96).contains(&value.len()) && !secret_like
                && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        });
    Error::Http { status, code, retry_after }
}

fn parse_retry_after(headers: &header::HeaderMap) -> Option<RetryAfter> {
    let all = headers.get_all(header::RETRY_AFTER);
    let mut values = all.iter();
    let raw = values.next()?.to_str().ok()?;
    // Reject duplicates and oversized/malformed input instead of echoing or
    // making a shorter delay from ambiguous data. HTTP dates fit within 64 bytes.
    if values.next().is_some() || raw.len() > 64 { return None; }
    let value = raw.trim_matches(|character| character == ' ' || character == '\t');
    if value.is_empty() { return None; }
    if value.bytes().all(|byte|byte.is_ascii_digit()) {
        return value.parse().ok().map(RetryAfter::DelaySeconds);
    }
    httpdate::parse_http_date(value).ok().map(RetryAfter::HttpDate)
}

async fn bounded(mut response: Response, cap: usize) -> Result<Vec<u8>, Error> {
    if response.content_length().is_some_and(|n| n > cap as u64) { return Err(Error::ResponseTooLarge); }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| Error::Transport)? {
        if chunk.len() > cap - bytes.len() { return Err(Error::ResponseTooLarge); }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
