# External subtitles

Use `GET /sxapi/v1/media/{id}/subtitles` with the selected version (and episode where needed) only when `features.subtitles=true`. Reading the ticket additionally requires `features.subtitle_ticket_header=true`; missing/false is unsupported, regardless of protocol version. Use the opaque subtitle identifier from that inventory; never accept arbitrary paths or provider addresses.

Request `POST /playback/sessions/{id}/subtitles/{subtitle_id}/tickets`. Keep the returned `ticket` private and resolve its credential-free `read_path` against the trusted origin. Send **both** the original `Authorization: Bearer` and `X-SXAPI-Subtitle-Ticket` on the read request. `header_name` must equal that subtitle header, including private transport. Do not fall back to a path-embedded ticket.

The response can be HEAD or GET and is bound to the selected source, original login, session, actual expiry, permissions and quotas. The current contract caps external subtitles at 2 MiB; the SDK enforces that response cap. A ticket does not promise that the player can decode a subtitle format or authorize burned-in subtitles.

Do not log text from subtitles as diagnostic data. A missing subtitle can be shown as unavailable without suppressing the valid video source. No automatic subtitle conversion is performed by this Kit.
