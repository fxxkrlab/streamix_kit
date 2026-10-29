//! Explicit, non-authorizing client labels and per-installation UUID storage.

use crate::Error;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT_LANGUAGE};
use std::{fs::{self, OpenOptions}, io::{Read, Write}, path::Path};
use uuid::Uuid;

/// Values supplied by the application's build pipeline, never inferred from login.
#[derive(Clone, Debug)]
pub struct BuildInfo {
    pub product: String,
    pub version: String,
    pub system: String,
    pub system_version: String,
    pub model_or_arch: String,
    pub build: String,
    /// Exactly test, beta or release.
    pub channel: String,
    /// Required only for the first-party Streamix product.
    pub streamix_core_version: Option<String>,
}

/// Descriptive identity. Neither this UUID nor its name grants any permission.
#[derive(Clone, Debug)]
pub struct DeviceIdentity {
    pub installation_id: Uuid,
    /// User-editable UTF-8 display name, maximum 128 decoded bytes.
    pub display_name: String,
}

/// Explicit purpose, independent from and never embedded in User-Agent.
#[derive(Clone, Copy, Debug, Default)]
pub enum RequestPurpose {
    #[default]
    Interaction,
    Prefetch,
}

/// Optional request diagnostic context, deliberately excluding URL and credentials.
#[derive(Clone, Debug)]
pub struct RequestTrace {
    pub method: String,
    pub purpose: RequestPurpose,
    pub traceparent: String,
}

/// Validated self-reported request metadata; never an authentication credential.
#[derive(Clone)]
pub struct RequestIdentity {
    user_agent: HeaderValue,
    client: HeaderValue,
    device_id: HeaderValue,
    device_name: HeaderValue,
    language: Option<HeaderValue>,
}

fn label(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
        && !value.contains("://") && !value.to_ascii_lowercase().contains("stx_")
}

fn token(value: &str) -> bool {
    label(value, 64) && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn comment(value: &str) -> bool {
    label(value, 128) && value.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        && !value.bytes().any(|b| b"();\\".contains(&b))
}

fn value(text: &str) -> Result<HeaderValue, Error> {
    HeaderValue::from_str(text).map_err(|_| Error::InvalidInput)
}

impl RequestIdentity {
    /// Build one stable UA and identity set from explicit application data.
    pub fn new(build: BuildInfo, device: DeviceIdentity, language: Option<&str>) -> Result<Self, Error> {
        if !token(&build.product) || !token(&build.version) || !token(&build.build)
            || !comment(&build.system) || !comment(&build.system_version) || !comment(&build.model_or_arch)
            || !["test", "beta", "release"].contains(&build.channel.as_str())
            || device.installation_id.is_nil() || device.installation_id.get_version_num() != 4
            || !label(&device.display_name, 128) { return Err(Error::InvalidInput); }
        let suffix = if build.product == "Streamix" {
            let core = build.streamix_core_version.as_deref().ok_or(Error::InvalidInput)?;
            if !token(core) { return Err(Error::InvalidInput); }
            format!("StreamixCore/{core}")
        } else {
            format!("StreamixKit/{}", env!("CARGO_PKG_VERSION"))
        };
        let client = format!("{}/{}",build.product,build.version);
        if client.len() > 128 { return Err(Error::InvalidInput); }
        let ua = format!("{}/{} ({} {}; {}; build {}; {}) {}", build.product, build.version, build.system, build.system_version, build.model_or_arch, build.build, build.channel, suffix);
        let encoded: String = device.display_name.bytes().map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) { (byte as char).to_string() }
            else { format!("%{byte:02X}") }
        }).collect();
        let language = language.map(|text| {
            if !label(text, 256) || !text.bytes().all(|b| b.is_ascii_alphanumeric() || b"-,;= .".contains(&b)) { return Err(Error::InvalidInput); }
            value(text)
        }).transpose()?;
        Ok(Self { user_agent:value(&ua)?,client:value(&client)?,device_id:value(&device.installation_id.hyphenated().to_string())?,device_name:value(&encoded)?,language })
    }

    pub(crate) fn headers(&self, purpose: RequestPurpose) -> HeaderMap {
        let mut result = HeaderMap::new();
        result.insert(USER_AGENT,self.user_agent.clone());
        result.insert("x-streamix-client",self.client.clone());
        result.insert("x-streamix-device-id",self.device_id.clone());
        result.insert("x-streamix-device-name",self.device_name.clone());
        result.insert("x-streamix-protocol",HeaderValue::from_static("SXAPI/1.0"));
        if let Some(language) = &self.language { result.insert(ACCEPT_LANGUAGE,language.clone()); }
        // Random per request, unrelated to accounts, IPs, credentials or hardware.
        let trace = Uuid::new_v4().simple().to_string();
        let span = Uuid::new_v4().simple().to_string();
        let traceparent = format!("00-{trace}-{}-01",&span[..16]);
        // The format above consists only of fixed ASCII and UUID hexadecimal.
        if let Ok(header) = HeaderValue::from_str(&traceparent) { result.insert("traceparent",header); }
        if matches!(purpose,RequestPurpose::Prefetch) { result.insert("sec-purpose",HeaderValue::from_static("prefetch")); }
        result
    }
}

/// Load or create one random installation UUID in an explicitly chosen private file.
///
/// The application owns its private parent directory and chooses when an install
/// is reset. Existing invalid files and symlinks are rejected, never overwritten.
/// This UUID is a descriptive label, not a secret or authorization token.
pub fn load_or_create_installation_id(path: &Path) -> Result<Uuid, Error> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 { return Err(Error::InvalidInput); }
        let mut raw = String::new();
        fs::File::open(path).map_err(|_| Error::InvalidInput)?.take(64).read_to_string(&mut raw).map_err(|_| Error::InvalidInput)?;
        let id = Uuid::parse_str(raw.trim()).map_err(|_| Error::InvalidInput)?;
        if id.is_nil() || id.get_version_num() != 4 { return Err(Error::InvalidInput); }
        return Ok(id);
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| Error::InvalidInput)?;
    let id = Uuid::new_v4();
    file.write_all(format!("{}\n",id.hyphenated()).as_bytes()).map_err(|_| Error::InvalidInput)?;
    file.sync_all().map_err(|_| Error::InvalidInput)?;
    Ok(id)
}
