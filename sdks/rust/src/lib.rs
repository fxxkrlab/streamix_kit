//! SXAPI v1 player facade with header-only credentials and bounded responses.
#![forbid(unsafe_code)]

mod models;
pub use models::*;
mod identity;
pub use identity::*;
mod connecting;
pub use connecting::*;

use chrono::{DateTime, Utc};
use reqwest::{header::{self, HeaderValue}, Method, RequestBuilder, Response, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, sync::Arc, time::Duration};
use uuid::Uuid;
use zeroize::Zeroizing;

const JSON_CAP: usize = 2 * 1024 * 1024;
const RANGE_CAP: u64 = 16 * 1024 * 1024;
const SUBTITLE_CAP: usize = 2 * 1024 * 1024;
const VIDEO_HEADER: &str = "X-SXAPI-Stream-Ticket";
const SUBTITLE_HEADER: &str = "X-SXAPI-Subtitle-Ticket";

/// An owned secret with redacted Debug and best-effort zeroizing storage.
pub struct Secret(Zeroizing<String>);

impl Secret {
    /// Take ownership of a private value without exposing it through Debug.
    pub fn new(value: String) -> Self { Self(Zeroizing::new(value)) }
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
    CleanupFailed, Http { status: u16, code: Option<String> },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { status, code } => write!(f, "SXAPI HTTP {status} ({})", code.as_deref().unwrap_or("unclassified")),
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
    pub async fn login(&self, username: &str, password: Secret) -> Result<AuthenticatedClient, Error> {
        let capabilities = self.capabilities().await?;
        require_contract(&capabilities)?;
        #[derive(Serialize)] struct Login<'a> { username: &'a str, password: &'a str }
        let request = self.inner.api(Method::POST, "/sessions")?.json(&Login { username, password: password.text() });
        let login: LoginWire = self.inner.json(request).await?;
        if login.token_type != "Bearer" || !login.access_token.text().starts_with("stx_sx_") || login.access_token.text().len() > 2048 { return Err(Error::InvalidResponse); }
        login.access_token.header()?;
        Ok(AuthenticatedClient { client: self.clone(), token: login.access_token, user: login.user, features:capabilities.features })
    }

    /// Exchange a separately created personal secret; this does not rotate it.
    pub async fn refresh(&self, refresh_secret: Secret) -> Result<AuthenticatedClient, Error> {
        let capabilities = self.capabilities().await?;
        require_contract(&capabilities)?;
        #[derive(Serialize)] struct Refresh<'a> { refresh_token: &'a str }
        let body = Refresh { refresh_token: refresh_secret.text() };
        let login: RefreshWire = self.inner.json(self.inner.api(Method::POST, "/sessions/refresh")?.json(&body)).await?;
        if login.token_type != "Bearer" || !login.access_token.text().starts_with("stx_sx_") || login.access_token.text().len() > 2048 { return Err(Error::InvalidResponse); }
        login.access_token.header()?;
        let mut client = AuthenticatedClient { client: self.clone(), token: login.access_token, user: User { id: Uuid::nil(), username: String::new(), library_ids: Vec::new(), extra: Default::default() }, features:capabilities.features };
        client.user = client.json(Method::GET, "/users/me").await?;
        Ok(client)
    }
}

#[derive(Deserialize)] struct LoginWire { access_token: Secret, token_type: String, user: User }
#[derive(Deserialize)] struct RefreshWire { access_token: Secret, token_type: String }

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
pub struct AuthenticatedClient { client: Client, token: Secret, user: User, features:std::collections::BTreeMap<String,bool> }

impl AuthenticatedClient {
    /// Inspect the non-secret identity returned by login.
    pub fn user(&self) -> &User { &self.user }
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
        if !(person || media_or_library) || cap == 0 || cap > 16 * 1024 * 1024 { return Err(Error::InvalidPath); }
        let request = self.client.inner.request(Method::GET,path,RequestPurpose::Interaction)?.header(header::AUTHORIZATION, self.bearer()?);
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
        if self.ended { return Err(Error::SessionEnded); }
        let grant: VideoGrant = self.login.json(Method::POST, &format!("/playback/sessions/{}/tickets", self.session.session_id)).await?;
        grant.validate(&self.session)?;
        self.grant = grant;
        Ok(())
    }
    async fn current_grant(&mut self) -> Result<(), Error> {
        if self.ended { return Err(Error::SessionEnded); }
        if self.grant.expires_at <= Utc::now() + chrono::Duration::seconds(15) { self.renew().await?; }
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
        self.current_grant().await?;
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
        self.current_grant().await?;
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
    let code = bounded(response, 16 * 1024).await.ok().and_then(|raw| serde_json::from_slice::<Value>(&raw).ok())
        .and_then(|value| value.get("error")?.get("code")?.as_str().map(str::to_owned))
        .filter(|value| !value.is_empty() && value.len() <= 96 && !value.starts_with("stx_")
            && !(value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b)));
    Err(Error::Http { status, code })
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
