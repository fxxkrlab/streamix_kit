//! Typed player models. Unknown response fields are retained for additive changes.

use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Additional wire response fields retained without assuming their semantics.
pub type Extra = BTreeMap<String, Value>;

/// Current server discovery; protocol version alone does not authorize transport.
#[derive(Debug, Deserialize)]
pub struct Capabilities {
    pub protocol: String,
    pub version: String,
    pub features: BTreeMap<String, bool>,
    pub server: Option<ServerIdentity>,
    #[serde(default)]
    pub limits: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Persistent public server identity. This is distinct from any private issuer ID.
#[derive(Debug, Deserialize)]
pub struct ServerIdentity {
    pub id: Uuid,
    pub name: String,
    pub product: String,
    pub version: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Current account identity, without any login credential.
#[derive(Debug, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub library_ids: Vec<Uuid>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A library visible under the current account's live grants.
#[derive(Debug, Deserialize)]
pub struct Library {
    pub id: Uuid,
    pub name: String,
    pub library_type: String,
    pub library_kind: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A file-backed media catalog item, not proof of client decoding.
#[derive(Debug, Deserialize)]
pub struct Media {
    pub id: Uuid,
    pub name: String,
    pub media_type: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A keyset page; the opaque cursor is not an authorization credential.
#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub next_cursor: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Selected actual file version and its observed technical status.
#[derive(Debug, Deserialize)]
pub struct Version {
    pub id: Uuid,
    pub media_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub technical_status: String,
    pub duration_secs: Option<f64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Truthful player constraints. Missing facts require an explicit opt-in.
#[derive(Debug, Default, Serialize)]
pub struct PlaybackProfile {
    pub containers: Vec<String>,
    pub video_codecs: Vec<String>,
    pub audio_codecs: Vec<String>,
    pub max_bitrate_bps: Option<i64>,
    pub allow_unverified: bool,
}

/// Prepare an original-file session for one movie or exact episode/version.
#[derive(Debug, Serialize)]
pub struct PlaybackRequest {
    pub media_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<Uuid>,
    pub profile: PlaybackProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

/// Bound playback session. No transport credential is exposed by this model.
#[derive(Debug, Deserialize)]
pub struct PlaybackSession {
    pub session_id: Uuid,
    pub media_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub version_id: Uuid,
    pub mode: String,
    pub compatibility: String,
    pub expires_at: DateTime<Utc>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Explicit, ordered client playback report; never infer completion from a read.
#[derive(Debug, Serialize)]
pub struct Progress {
    pub sequence: i64,
    pub position_ms: i64,
    pub completed: bool,
}

/// Actual acknowledgement of one ordered report; not a current-state snapshot.
#[derive(Debug, Deserialize)]
pub struct ProgressAck {
    pub session_id: Uuid,
    pub sequence: i64,
    pub position_ms: i64,
    pub completed: bool,
    pub state_revision: String,
    pub recorded_at: DateTime<Utc>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Selected external subtitle inventory, without provider paths or credentials.
#[derive(Debug, Deserialize)]
pub struct SubtitleInventory {
    pub media_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub version_id: Uuid,
    pub revision: String,
    pub embedded_status: String,
    pub data: Vec<Subtitle>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// An opaque selected subtitle identifier; language can truthfully be unknown.
#[derive(Debug, Deserialize)]
pub struct Subtitle {
    pub id: String,
    pub format: String,
    pub language: Option<String>,
    pub delivery: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Returned bytes from one validated range, never a decoded playback result.
#[derive(Debug)]
pub struct RangeData {
    pub bytes: Vec<u8>,
    pub start: u64,
    pub end: u64,
    pub total_length: u64,
    pub strong_etag: Option<String>,
}
