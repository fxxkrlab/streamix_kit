# Preload grants and downloads (server 0.9.0-rc.25 and later)

From 0.9.0-rc.25 the server decides background preloads, whole-file disk caches and downloads one version at a time: the client asks first, then sends the returned grant with the stream reads. Normal playback, the buffering that goes with it and a disk cache that keeps only the parts already watched need no grant and work as before.

## 1. Check the capabilities

Read `GET /sxapi/v1/capabilities`:

- `features.prefetch_grants=true`: `POST /prefetch/grants` and the `STREAMIX-Grant` stream header are honoured. Use the flow below.
- `features.download_permissions=true` (always true from 0.9.0-rc.25): whole-file caches and downloads follow the server's download permission, which the administrator sets in three tiers (the site, each library, each user).
- `limits.prefetch_max_bytes_ceiling` (10737418240, 10 GiB): the largest preload budget a server can grant; a client setting above it gains nothing.

`prefetch_grants` is false when playback is unavailable, and in Nimbus private mode until the private runtime checks grants. In that mode private stream reads neither check `STREAMIX-Grant`, nor count preload bytes, nor re-check the download permission, while `download_permissions` stays true: still ask before a whole-file cache or download and follow the decision (it is made when the grant is issued or renewed), and preload under the earlier rules in [client request headers](client-request-headers.md#request-purpose). A server before 0.9.0-rc.25 has neither capability nor the grant operations; keep the earlier preload rules there.

## 2. Ask for a grant

Before each preload, whole-file cache or download of one version, send `POST /sxapi/v1/prefetch/grants` with the login's Bearer:

```json
{"media_id": "11111111-1111-4111-8111-111111111111", "version_id": "22222222-2222-4222-8222-222222222222", "category": "next_episode", "network": "lan"}
```

- `media_id`: the work (movie or series) or the episode the version belongs to.
- `version_id`: a current file-backed version of it. Prepare the playback session for this same version.
- `category`: `detail` (preload from a detail or episode page), `next_episode` (preload of the next episode), `cache` (whole-file disk cache while playing) or `download` (download for offline viewing).
- `network`: `lan` or `wan`, as the client believes; it is recorded and does not change the decision yet.

Other members are rejected (400). Send this request as an ordinary request, with no purpose product in the User-Agent and no `Sec-Purpose: prefetch`. A request marked as a preload meets the preload switch and the preload slots first and gets 403 `prefetch_disabled` or 429 `prefetch_busy` instead of a decision.

Approval and refusal are both 200; `granted` tells them apart. Approved:

```json
{"granted": true, "grant_id": "33333333-3333-4333-8333-333333333333", "max_bytes": 268435456, "max_seconds": null, "expires_at": "2026-11-23T08:10:00Z"}
```

- `grant_id`: send it as `STREAMIX-Grant` on the stream reads.
- `max_bytes`: for `detail` and `next_episode`, the bytes this grant may read (the user's setting, else the site setting, 256 MiB by default, between 1 MiB and 10 GiB). Preload the smaller of this and the client's own setting, and when the server's value is the smaller one, say so ("limited by the server to X"). Null for `cache` and `download`, which are not counted.
- `max_seconds`: always null; there is no duration limit.
- `expires_at`: the first stream read must start before it (10 minutes after issue). A grant that has started keeps its budget afterwards.
- `download` only: `license_expires_at`, the end of the offline licence (30 days after issue or renewal by default), and `renew_after`, from when to renew while online (half the licence).

Refused:

```json
{"granted": false, "reason": "rate_limited", "retry_after_seconds": 12}
```

| `reason` | Category | Meaning |
| --- | --- | --- |
| `disabled_globally` | `cache`, `download` | Downloads are off for everyone on this server, administrators included. |
| `not_opted_in` | `cache`, `download` | Downloads are allowed only where an administrator allowed them, and neither this user nor a library holding the item is allowed. |
| `library_not_allowed` | `cache`, `download` | A library holding the item denies downloads. |
| `user_not_allowed` | `cache`, `download` | Downloads are denied for this user. |
| `prefetch_disabled` | `detail`, `next_episode` | The administrator has turned preloads off. |
| `server_busy` | `detail`, `next_episode` | The server's preload capacity is full; comes with `retry_after_seconds`. |
| `rate_limited` | any | More than 30 grants for this user within a minute; comes with `retry_after_seconds`. |

After a refusal, do not preload, cache or download, and do not ask again, except after `retry_after_seconds` for `server_busy` and `rate_limited`. Explain an unavailable download by its reason. The download decision runs in this order: a site set to disabled refuses everyone; otherwise a deny on the user, or on any library the user can see that holds the item, refuses; otherwise an allow on either allows; otherwise the site default decides (`everyone` allows, `opted_in` refuses).

Errors are not refusals and use the usual SXAPI error body: 400 for an invalid body, 401 for the login, 404 `not_found` when the item is not visible to the account or the version is not a current file of it, and 503 when a dependency is unavailable.

## 3. Read with the grant

Send the grant on `GET` or `HEAD` of the session's `stream_path`, or of a disc file read, next to the ticket; it never replaces `X-SXAPI-Stream-Ticket`:

```text
X-SXAPI-Stream-Ticket: …
STREAMIX-Grant: 33333333-3333-4333-8333-333333333333
Range: bytes=0-
```

Send exactly one value, never repeated or comma-combined. The grant belongs to the user, not to one login, and must match the session's user and version (the files of a folder disc share the folder version). A header that is present is always checked; an invalid one is never treated as ordinary playback.

`detail` and `next_episode` grants count bytes:

- Each read reserves what it serves from `max_bytes`. A Range read longer than what is left, an open `bytes=N-` included, is cut to the rest and answered with a shorter 206: take the actual range from `Content-Range` and `Content-Length`. A read without Range is served (200) only when the whole file fits. HEAD reads nothing and reserves nothing.
- These reads take the preload lease outside the concurrent-stream limit, whatever purpose the User-Agent names, so a next-episode preload sent with `Streamix-Play` under a `next_episode` grant leaves the playing slot free. Later reads of the same session without the grant are normal playback and take a normal slot. Monthly byte quotas still apply.

`cache` and `download` grants are not counted, but the download permission runs again on every read, so an administrator's change stops the next read. These reads still count toward the concurrent-stream limit like normal playback, so a running download can hold the account's stream slot. End the User-Agent of whole-file cache and download requests with the purpose product `Streamix-Download/1` and send no `Sec-Purpose` (see [client request headers](client-request-headers.md#request-purpose)); a read with that product re-runs the download permission even without the grant header.

| Response | Cause | Client action |
| --- | --- | --- |
| 206 shorter than asked | The `detail` / `next_episode` budget ran out inside this range | Keep the bytes `Content-Range` names; the preload is complete. |
| 403 `grant_invalid` | Malformed, unknown, another user's or another version's grant | Stop sending it; ask for a new grant for this version. |
| 403 `grant_expired` | First read after `expires_at`, or the download licence has passed | Ask for a new grant, or renew the download licence. |
| 403 `grant_exhausted` | Nothing is left of the budget, or a read without Range does not fit; no `Content-Range` | Stop preloading. It says nothing about the file length. |
| 403 `prefetch_disabled` | Preloads were turned off after the grant was issued | Stop preloading; normal playback is unaffected. |
| 403 `disabled_globally`, `not_opted_in`, `user_not_allowed`, `library_not_allowed` | The download permission now refuses this `cache` / `download` read | Stop caching or downloading and show the reason. |
| 416 | The range lies outside the file (`Content-Range: bytes */size`); never a budget | Recheck the representation and range as before. |

## 4. Renew an offline licence

`POST /sxapi/v1/prefetch/grants/{grant_id}/renew`, with the Bearer and no body, renews a `download` grant of the same user. Renew while online once `renew_after` has passed.

- The download permission is decided again. An approval is the same 200 body with a new `license_expires_at` (now plus the licence days) and `renew_after`; `grant_id` and `expires_at` stay the same.
- A refusal is a 200 refusal with its reason; the copy keeps its current licence.
- When `license_expires_at` passes without renewal, stop offline playback of that copy and tell the user why. The server never deletes a copy remotely.
- 404 for another user's or an unknown grant, or an item that is no longer visible; 409 `grant_not_renewable` for a `detail`, `next_episode` or `cache` grant.

The Rust SDK does not wrap these operations and never sends `STREAMIX-Grant`; its range helper also rejects a 206 that stops before both the requested end and the end of the file. Make grant-bound reads over HTTP with the ticket and Range rules of the [playback guide](playback.md).
