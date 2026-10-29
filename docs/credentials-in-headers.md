# Credentials stay out of addresses

| Request | Credential location |
| --- | --- |
| Password login | JSON body over the approved transport (HTTPS by default) |
| Personal credential exchange | JSON body over the approved transport (HTTPS by default) |
| Authenticated SXAPI JSON and images | `Authorization: Bearer …` |
| Video GET / HEAD | `X-SXAPI-Stream-Ticket: …` |
| External subtitle GET / HEAD | `X-SXAPI-Subtitle-Ticket: …` **and** original Bearer |

The video grant contains `stream_path`, `ticket`, `header_name` and `expires_at`. The only accepted video header name is `X-SXAPI-Stream-Ticket`. The deprecated `stream_url` is a credential-free alias of `stream_path`; the SDK does not use it as a fallback. The subtitle grant contains `read_path` with the fixed subtitle header name. Missing or inconsistent fields are a contract error.

Paths must be root-relative, same-origin and free of query strings, fragments and embedded credentials. Private transport can return a fixed `/nimbus-play/v1/sxapi` or `/nimbus-play/v1/sxapi-subtitle` path. Do not append an API prefix or extract a token from a path. UUIDs and the non-secret subtitle identifier are resource identifiers, not authorization.

Do not put access tokens, refresh secrets, tickets or API keys in query parameters, path segments, cookies or referrers. Query-credential requests and duplicate ticket headers are rejected. A login from one origin must never authorize a redirect to another origin. This SDK rejects all redirects; an application must establish a new trusted origin explicitly.

Trusted-local HTTP is a separate explicit user choice under [connecting](connecting.md); credentials remain cleartext on that transport. Keeping them in headers does not encrypt HTTP and never permits an automatic TLS downgrade.

`check_credentials.py` rejects credential-bearing URL patterns and query/cookie security definitions in published content. It is a static safeguard, not a replacement for runtime authorization. Do not print raw HTTP errors, response bodies, token-bearing DTOs, curl verbose traces, environment dumps or session headers.
