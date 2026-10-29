use std::env;
use streamix_kit::{load_or_create_installation_id,BuildInfo,Client,ConnectionMode,DeviceIdentity,Error,PlaybackProfile,PlaybackRequest,RequestIdentity,Secret};
use uuid::Uuid;

fn required(name: &str) -> Result<String, Error> {
    env::var(name).map_err(|_| Error::InvalidInput)
}

fn profile() -> Result<PlaybackProfile, Error> {
    let value: serde_json::Value = serde_json::from_str(&required("STREAMIX_PROFILE_JSON")?).map_err(|_| Error::InvalidInput)?;
    let strings = |name: &str| -> Result<Vec<String>, Error> {
        value.get(name).and_then(serde_json::Value::as_array).ok_or(Error::InvalidInput)?.iter()
            .map(|x| x.as_str().map(str::to_owned).ok_or(Error::InvalidInput)).collect()
    };
    let max = match value.get("max_bitrate_bps") {
        Some(serde_json::Value::Null) => None,
        Some(number) => Some(number.as_i64().filter(|n| *n > 0).ok_or(Error::InvalidInput)?),
        None => return Err(Error::InvalidInput),
    };
    Ok(PlaybackProfile { containers:strings("containers")?,video_codecs:strings("video_codecs")?,audio_codecs:strings("audio_codecs")?,max_bitrate_bps:max,allow_unverified:value.get("allow_unverified").and_then(serde_json::Value::as_bool).ok_or(Error::InvalidInput)? })
}

async fn run() -> Result<(), Error> {
    // Validate every input before login, so local configuration errors create no session.
    let installation_id = load_or_create_installation_id(std::path::Path::new(&required("STREAMIX_DEVICE_ID_FILE")?))?;
    let identity = RequestIdentity::new(BuildInfo { product:"StreamixKitExample".into(),version:env!("CARGO_PKG_VERSION").into(),system:env::consts::OS.into(),system_version:required("STREAMIX_SYSTEM_VERSION")?,model_or_arch:env::consts::ARCH.into(),build:include_str!("../../../BUILD").trim().into(),channel:"test".into(),streamix_core_version:None },DeviceIdentity { installation_id,display_name:required("STREAMIX_DEVICE_NAME")? },env::var("STREAMIX_ACCEPT_LANGUAGE").ok().as_deref())?;
    let mode = if env::var("STREAMIX_ALLOW_LOCAL_HTTP").ok().as_deref() == Some("1") { ConnectionMode::LocalHttp } else { ConnectionMode::Https };
    let client = Client::with_address(&required("STREAMIX_ORIGIN")?,identity,mode)?;
    let username = required("STREAMIX_USERNAME")?;
    let password = Secret::new(required("STREAMIX_PASSWORD")?);
    let media_id = Uuid::parse_str(&required("STREAMIX_MEDIA_ID")?).map_err(|_| Error::InvalidInput)?;
    let version_id = env::var("STREAMIX_VERSION_ID").ok().map(|v| Uuid::parse_str(&v).map_err(|_| Error::InvalidInput)).transpose()?;
    let request = PlaybackRequest { media_id,episode_id:None,version_id,profile:profile()?,device_id:Some("StreamixKit bounded example".into()) };
    let login = client.login(&username,password).await?;
    println!("LOGIN_OK");
    let flow = async {
        let libraries = login.libraries().await?;
        let library = libraries.first().ok_or(Error::InvalidInput)?;
        let _page = login.media(library.id,None,None,1).await?;
        println!("BROWSE_OK");
        let mut playback = login.prepare(&request).await?;
        println!("PREPARE_OK");
        let read = playback.read_range(0,1_048_575,None).await;
        let end = playback.end().await;
        if end.is_err() { return Err(Error::CleanupFailed); }
        println!("END_OK");
        let bytes = read?;
        println!("RANGE_OK bytes={}",bytes.bytes.len());
        Ok(())
    }.await;
    let logout = login.logout().await;
    if logout.is_err() { return Err(Error::CleanupFailed); }
    println!("LOGOUT_OK");
    flow
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("FLOW_FAILED: {error}");
        std::process::exit(1);
    }
}
