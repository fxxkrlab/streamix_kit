//! Explicit protocol/port selection; discovery hints never choose authentication.

use crate::Error;
use reqwest::Url;

/// The user explicitly chooses a protocol; no failed HTTPS request downgrades.
#[derive(Clone, Copy, Debug, Default)]
pub enum ConnectionMode {
    #[default]
    Https,
    /// Opt in to cleartext HTTP on a user-approved local network.
    LocalHttp,
}

/// A normalized, credential-free origin with the documented default port.
#[derive(Clone, Debug)]
pub struct ServerAddress(Url);

impl ServerAddress {
    /// Parse a bare host or matching full origin. Explicit ports always win.
    pub fn parse(input: &str, mode: ConnectionMode) -> Result<Self, Error> {
        let scheme = match mode { ConnectionMode::Https => "https", ConnectionMode::LocalHttp => "http" };
        let input = input.trim();
        if input.is_empty() || input.chars().any(char::is_control) { return Err(Error::InvalidOrigin); }
        let address = if input.contains("://") { input.to_owned() } else { format!("{scheme}://{input}") };
        let authority = address.split_once("://").ok_or(Error::InvalidOrigin)?.1.split('/').next().ok_or(Error::InvalidOrigin)?;
        // URL normalizes an explicitly supplied :80 away, so preserve that choice
        // before parsing rather than confusing it with an omitted HTTP port.
        let port_text = if authority.starts_with('[') {
            authority.split_once(']').and_then(|(_,tail)|tail.strip_prefix(':'))
        } else { authority.rsplit_once(':').map(|(_,port)|port) };
        let explicit_port = port_text.is_some();
        if port_text.is_some_and(|port|port.is_empty() || !port.bytes().all(|b|b.is_ascii_digit()) || port.parse::<u16>().ok().filter(|port|*port > 0).is_none()) { return Err(Error::InvalidOrigin); }
        let mut url = Url::parse(&address).map_err(|_| Error::InvalidOrigin)?;
        if url.scheme() != scheme || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some()
            || url.query().is_some() || url.fragment().is_some() || url.path() != "/" { return Err(Error::InvalidOrigin); }
        if matches!(mode,ConnectionMode::LocalHttp) && !explicit_port { url.set_port(Some(8970)).map_err(|_|Error::InvalidOrigin)?; }
        Ok(Self(url))
    }

    /// Inspect the normalized non-secret connection origin, never a grant URL.
    pub fn origin(&self) -> &str { self.0.as_str() }
}
