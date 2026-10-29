# Bounded Rust CLI example

Source candidate; not executed against any real server yet. Requires the final header-ticket server contract and explicit test-server authorization. See [versioning](../../docs/versioning.md).

Set `STREAMIX_ORIGIN`, `STREAMIX_USERNAME`, `STREAMIX_PASSWORD`, `STREAMIX_MEDIA_ID`, and `STREAMIX_PROFILE_JSON` privately in the environment. Use a permitted **movie** ID and a JSON object containing the actual player's `containers`, `video_codecs`, `audio_codecs`, `max_bitrate_bps`, and `allow_unverified` values. The CLI refuses a missing profile, rather than claiming that it can decode everything. Optional `STREAMIX_VERSION_ID` selects an exact existing file.

Also set `STREAMIX_DEVICE_ID_FILE` to an explicit file within your application-private local directory, `STREAMIX_DEVICE_NAME` to a non-sensitive display label, and `STREAMIX_SYSTEM_VERSION` to the OS version. The parent directory must already exist and be private to your user. The UUID is generated once and reused; the script neither reads hardware serials nor uses the login name as a device label. Optional `STREAMIX_ACCEPT_LANGUAGE` supplies a UI language preference. See [request headers](../../docs/client-request-headers.md).

HTTPS is the default. Only for an explicitly approved trusted-local HTTP origin, set `STREAMIX_ALLOW_LOCAL_HTTP=1`; bare hosts receive HTTP 8970 or HTTPS 443, and explicit ports win. Credentials over HTTP are cleartext. Discovery never opts the user into HTTP automatically.

Run the `streamix-kit-example` binary only in the authorized environment. It logs in, reads one library and one catalog page, prepares the chosen movie, reads bytes 0–1048575 (up to 1 MiB), ends the session and logs out. It never saves the video or prints media names, IDs, user records, credentials or raw HTTP output. It does not send watched/completed progress because reading a fragment is not actual playback.

Output is restricted to safe stage markers and the byte count. An end/logout failure causes a nonzero result. This CLI proves a bounded HTTP flow only; it cannot prove a first frame or decoder behavior. Environment variables can be inspected by other processes under the same user; use a private test environment and avoid shell tracing or environment dumps.
