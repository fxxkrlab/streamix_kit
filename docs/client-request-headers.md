# Client request headers

These are optional, self-reported descriptive labels. They never authenticate a user, grant a library, bypass a quota or establish trusted player capabilities. Credentials remain in the distinct headers described in [credentials-in-headers](credentials-in-headers.md). Server recognition depends on the matching server candidate; source availability does not prove deployment or UI acceptance.

## User-Agent

Use RFC 9110 product/version tokens and a parenthesized comment. First-party Streamix uses this exact ordering:

```text
Streamix/1.2.0 (Windows 10.0.26100; x64; build 812; beta) StreamixCore/0.9.0
```

Version, Build, channel (`test`, `beta`, `release`) and Core version come from build metadata. The shared Core builds the string once; requests reuse it. Third-party players use their own product name with the Kit suffix, for example:

```text
ExamplePlayer/1.0.0 (ExampleOS 1.0; x64; build 42; test) StreamixKit/0.1.0-rc.1
```

Do not concatenate product, platform and version into an unparseable token. Do not put request purpose, account name, origin, IP, token or ticket in the UA. Rust `BuildInfo` validates the fields and builds the common UA; it does not inspect the host name, login name or hardware serial number.

## Identity, language and tracing

```text
X-Streamix-Client: ExamplePlayer/1.0.0
X-Streamix-Device-Id: 55555555-5555-4555-8555-555555555555
X-Streamix-Device-Name: %E5%AE%A2%E5%8E%85%E7%94%B5%E8%A7%86
X-Streamix-Protocol: SXAPI/1.0
Accept-Language: zh-CN, zh;q=0.9, en;q=0.8
traceparent: 00-11111111111141118111111111111111-2222222222224222-01
```

The assembled `X-Streamix-Client` product/version is at most 128 ASCII bytes, including the separating slash. The UUID and trace above are fictional examples. Generate a random lowercase UUID once per installation, persist it locally and reuse it across logins. A reinstall/reset can deliberately create a new ID. Do not derive it from a hardware serial, MAC, account, server address or advertising identifier. `load_or_create_installation_id` uses an explicit app-private local file; the application chooses and protects its directory. The ID is a label, not a secret.

The user can edit the device display name. Its UTF-8 encoding is at most **128 decoded bytes**, has no control characters, and is percent-encoded before entering the ASCII HTTP header. This is not a 128-character limit, and percent-encoded length can be up to 384 bytes.

`Accept-Language` is a preference; it does not promise localized data exists. Stable `error.code` values are not translated. `X-Streamix-Protocol` declares support; an unsupported declaration can return 400 `client_version_unsupported`. Feature availability still comes from capabilities.

Generate a fresh valid W3C `traceparent` per request: version 00, nonzero 32-hex trace ID, nonzero 16-hex parent ID, flags 01. A trace is safe to show in a client diagnostic view. It must not contain usernames, addresses, IPs or credentials. The SDK's optional `with_request_observer` reports only method, purpose and the outgoing traceparent, enabling a client diagnostic view without exposing URLs, headers or bodies. It cannot modify the request and performs no automatic logging.

## Request purpose

User interaction or an explicit play click omits purpose. Intentional background prefetch sends:

```text
Sec-Purpose: prefetch
```

Keep UA and device identity unchanged between these requests. The server may lower priority or return 429/503 with Retry-After; honor it with bounded backoff. A label never relaxes authentication or guarantees a prioritization policy. Rust exposes explicit purpose-aware browsing and Range methods; defaults remain interaction. Server-side scheduling changes are outside this Kit.

When a prefetch Range needs a fresh ticket, the automatic ticket POST and subsequent GET both retain prefetch purpose. Explicit `Playback::renew` remains an interaction request by default. The observer reports the same purpose that is sent; it does not turn a tag into authority.

Emby/Jellyfin identity uses its own MediaBrowser-style convention and belongs to a separate adapter. SXAPI must not send its login secrets in those legacy headers. `X-SXAPI-*` holds SXAPI credentials/control; `X-Streamix-*` labels the client.
