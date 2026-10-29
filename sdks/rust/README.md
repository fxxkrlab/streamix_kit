# Rust SDK source candidate

Crate `streamix-kit`; Rust library `streamix_kit`. **0.1.0-rc.1 / Build 20261029.003, compilation and mock tests pending for this source.** The contract is source-bound in [SOURCE.json](../../spec/SOURCE.json); this is not SDK validation. No crates.io package is published; the manifest disables publication. See the commercial [LICENSE](../../LICENSE) before integration or binary redistribution.

This is a bounded typed facade for the common player flow, not a wrapper for every public operation. It preserves unknown JSON response fields. The exact synchronized contract generates an operation/schema inventory; it is not generated using an unverified OpenAPI 3.1 general-purpose tool.

```rust,no_run
use streamix_kit::{Client, RequestIdentity, Secret};

async fn libraries(origin: &str, identity: RequestIdentity, username: &str, password: String)
    -> Result<(), streamix_kit::Error>
{
    let client = Client::new(origin, identity)?;
    let login = client.login(username, Secret::new(password)).await?;
    let result = login.libraries().await;
    let logout = login.logout().await;
    result?;
    logout?;
    Ok(())
}
```

Capabilities must explicitly enable header tickets and forbid URL credentials. Authenticated API operations keep the Bearer internal; video and subtitle grant credentials are private fields with redacted Debug. Redirects, query-bearing grant paths, cross-origin paths, malformed grants and old transports fail closed. There is no raw URL credential accessor or generic authenticated arbitrary-URL method.

`Playback` owns the session and current grant. It renews before new Range/HEAD requests when the grant is within 15 seconds of expiry. The default bounded range limit is 16 MiB; the CLI requests 1 MiB. The helper accepts only matching 206 responses, checks the actual length, and reads no full body after a 200 range fallback. The caller must discard an existing fragment after `RepresentationChanged`.

Purpose-aware prefetch reads retain the prefetch label during an automatic ticket renewal and the subsequent GET. Explicit `renew` and normal reads remain interaction. `Error::retry_after` exposes validated seconds or a normalized HTTP date for caller-controlled backoff; the SDK never automatically sleeps or replays a failed request.

Always await `Playback::end` after stop/error/success and `AuthenticatedClient::logout` when done. Drop cannot perform async cleanup. Progress requires a real player position and increasing sequence; the SDK does not infer completion or retry mutations automatically. Credentials are held in zeroizing memory, but the application and HTTP stack may still create temporary copies; no claim of complete process-memory secrecy is made.

BuildInfo and DeviceIdentity are explicit application inputs. Use `load_or_create_installation_id` with an application-private file to persist a random per-install UUID; do not read hardware serial numbers or use account names. The SDK injects the stable UA/client/device labels and a new traceparent for every request. `media_with_purpose` and `read_range_with_purpose` mark intentional background prefetch without changing UA, authorization or limits. See [client request headers](../../docs/client-request-headers.md).

The SDK rejects redirects and disables automatic environment proxies. `Client::new` requires HTTPS; `Client::with_address` also supports an explicit trusted-local HTTP choice with default port 8970. HTTPS defaults to 443 and explicit ports win. No failed TLS request downgrades. See [connection security](../../docs/connecting.md). Proxy support requires an explicitly reviewed transport extension.
