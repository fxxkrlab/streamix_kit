# Authentication

`POST /sxapi/v1/sessions` accepts exactly `username` and `password`, and returns a Bearer `access_token` with the current user. The SDK keeps the token in memory and omits it from Debug and error output. An application needing persistent sign-in should use its platform's protected credential store, not a plain preferences file, crash report or URL.

`DELETE /sxapi/v1/sessions/current` revokes the token used on that request. Playback sessions must be ended separately as part of normal cleanup. Revoking the login also fences its children; it is not a substitute for reporting accurate progress.

`POST /sxapi/v1/sessions/refresh` is an optional **personal credential exchange**, not a generic password-session refresh endpoint. Its JSON accepts the user's separately created `refresh_token`; it returns a child login. Do not send the personal secret as Bearer and do not assume refresh-secret rotation. This candidate SDK supports the exchange but does not create or manage personal credentials.

On 401 stop using the failed credential and request sign-in. Do not repeatedly retry with the same secret. On 403 keep authentication separate from policy denial; changing a transport flag must not bypass current library, adult-content, concurrency or quota restrictions. A 503 is an infrastructure failure and must not be shown as a wrong password.

Video tickets authorize only their bound video stream. Subtitle tickets require the original Bearer **and** the subtitle header. A video credential is never accepted as general API authentication. See [credential headers](credentials-in-headers.md).
