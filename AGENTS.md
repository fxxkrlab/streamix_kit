# StreamixKit contributor rules

StreamixKit is a public client-integration repository under its commercial proprietary LICENSE.

- Reconcile actual Git root, remote, branch, dirty state and applicable instructions before changes.
- Track the exact GitHub Issue, Project 6 item and Linear task; do not create duplicate mirrors. Initial task: streamix_kit#1 / FXX-566.
- Use an isolated `codex/` branch and Draft PR with exact source, evidence and remaining delivery steps. Never implement on default branch.
- Public content only: no server implementation, private history, production configuration, credentials, private endpoints or logs. Use fictional origins.
- Specs come only from the reviewed server-generated full/client pair; pin source, minimum version and SHA256 in spec/SOURCE.json. Do not report unbound provenance as passing.
- Bearer, video and subtitle credentials stay in defined headers. No credential-in-URL fallback; deny credential-bearing redirects.
- Keep docs, CHANGELOG, VERSION and BUILD consistent. Source, CI, integration, merge, deployment and real-client acceptance are distinct.
- No local Docker, product builds or tests. Only the dedicated registered server Runner coordinated by the release owner; no cross-job copies or caches.
- No merge, tag, release, package publication or production change without specific authorization.
