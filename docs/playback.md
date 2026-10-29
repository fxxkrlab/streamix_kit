# Original-file playback

Current SXAPI playback is `direct_play`: no remuxing, audio conversion or video transcoding. Future modes are not part of this SDK candidate. A client profile describes what that player can consume; do not copy another player's profile or declare unsupported codecs to obtain a stream.

1. Prepare `/sxapi/v1/playback/sessions` with media ID, optional episode/version IDs and `profile`.
2. Retain the returned `session_id`. The session is bounded by the original login and server policy.
3. Validate the video grant: exact header name, credential-free root-relative `stream_path`, nonempty private ticket and actual `expires_at`. Tickets last at most 120 seconds and can be shorter. They permit new request admission; the current output authorization still governs an open body.
4. Send video GET or HEAD to that path on the trusted origin with `X-SXAPI-Stream-Ticket`. For a seek, request one valid byte range. Never replace credentials with a query string.
5. Before a new Range/HEAD when the ticket is near expiry, request `/playback/sessions/{id}/tickets` with the original Bearer and atomically replace the current header credential. Do not replay a failed POST automatically; the SDK performs no blind mutation retries.
6. Report progress through PUT `/playback/sessions/{id}/progress` using a monotonically increasing `sequence` and actual `position_ms`. `completed=true` means a real explicit completion report, not a duration estimate.
7. End via DELETE `/playback/sessions/{id}` on success, stop, cancellation and error. Rust Drop cannot perform async cleanup; the caller must await `end_playback`. Then log out when appropriate. A cleanup failure remains a reported failure, not a successful close.

The SDK's bounded range helper requires HTTP 206 and validates the starting offset, Content-Range and actual body length. If `If-Range` is stale, a server may send 200 with a whole representation: the SDK returns `RepresentationChanged` and reads no body. **Discard/reset the partial output; never append a 200 body to an old fragment.** A 416 requires rechecking the current representation and range, not retrying indefinitely.

The CLI reads at most 1 MiB and does not render video. A successful CLI read proves neither hardware decoding nor a visible first frame. Real-player acceptance is a separate result.
