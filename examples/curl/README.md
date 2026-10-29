# Header-only curl flow

`player_flow.py` drives the installed curl executable with a private stdin configuration, not credential-bearing command arguments. It demonstrates discovery, login, one library/catalog page, movie preparation, a bounded first 1 MiB, end and logout. It never writes credentials or media to files and prints only safe stage/status/byte markers.

Set `STREAMIX_ORIGIN`, `STREAMIX_USERNAME`, `STREAMIX_PASSWORD`, `STREAMIX_MEDIA_ID`, and `STREAMIX_PROFILE_JSON` privately in the authorized test environment. Use a permitted movie UUID and a truthful SXAPI profile object. Optional `STREAMIX_VERSION_ID` selects one actual version. Do not paste a production configuration, enable shell tracing or echo the variables. Run:

Also set `STREAMIX_DEVICE_ID_FILE` to a file in an existing app-private directory, `STREAMIX_DEVICE_NAME` to a non-sensitive display label and `STREAMIX_SYSTEM_VERSION` to the OS version. The script persists one random installation UUID, uses repository version/Build metadata and adds client/device/protocol headers plus a fresh traceparent on every curl request. `STREAMIX_ACCEPT_LANGUAGE` defaults to `en`. No hostname, username or hardware serial is used as a label. The flow is user interaction and omits Sec-Purpose; explicit background use can pass `prefetch=True` without changing UA. See [request headers](../../docs/client-request-headers.md).

```sh
python3 examples/curl/player_flow.py
```

The script requires curl and Python 3.11+. Redirects and environment proxies are disabled. HTTPS is the default; `STREAMIX_ALLOW_LOCAL_HTTP=1` explicitly opts into trusted-local cleartext HTTP. Bare hosts receive HTTP 8970 or HTTPS 443; explicit ports win. No TLS failure downgrades. Credentials remain in exchange JSON or their defined headers. A 200 video Range fallback fails instead of appending it.

The script is a source candidate awaiting designated-server validation and the final upstream binding. Success means a limited HTTP flow, not playback decoding. The Rust example offers stronger Content-Range/body checks. Environment variables may be visible to other processes under the same user; use a private environment and protected application storage for a real player.
