# Browsing and paging

Read `/sxapi/v1/libraries` for the current user's visible libraries. Then query `/media` with a selected library and a bounded `limit` (maximum 100, default 30 in the current contract). Catalog data is not sent as a single full-library response. Load a small initial page, then fetch more when the user scrolls.

Preserve an opaque `next_cursor` unchanged and keep the filters and sort consistent across pages. A cursor is not a credential, does not grant access and is bound to the original login. Restart from the first page after a new login, changed filters, cursor rejection or authorization change. Counts are observations and may change between requests.

For films use `/media/{id}/versions`; for series use the series/season/episode operations in the [synchronized client specification](../spec/README.md), then select versions for the chosen episode. A media ID, episode ID and version ID are distinct. Do not align episodes by title text or manufacture file-backed episodes from metadata-only entries.

Missing technical data does not prove that a file is absent. `technical_status=unknown` requires an explicit player decision on trying an unverified original file. Known unsupported codecs remain a mismatch. The SDK's profile defaults to `allow_unverified=false`; applications must opt in deliberately.

Request additional details, tracks, subtitles and artwork when the page needs them. Never infer adult or hidden-library access from a media ID, another user's result, a cached page or a previously issued cursor. Each API call rechecks current authorization.
