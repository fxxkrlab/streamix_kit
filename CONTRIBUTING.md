# Development and validation

Changes need an exact source Issue, acceptance criteria, linked Linear task, shared project board, isolated `codex/` branch and Draft PR. Preserve existing work; never implement on the default branch. Refs [streamix_kit#1](https://github.com/fxxkrlab/streamix_kit/issues/1); related to [FXX-566](https://linear.app/fxxkrlab/issue/FXX-566).

Public source only: no server implementation, private history, production domains, configurations, addresses, credentials or logs. Use fictional examples. Sensitive reports belong in a private channel, not a public Issue.

No local Docker, product builds or tests. Use the registered dedicated server Runner coordinated by the release owner. A new repository requires its own verified registration; labels do not register it. Workflows use `self-hosted`, `Linux`, `X64`, `portfolio-ci`, `portfolio-container`. Jobs need disposable workspaces and existing Runner cleanup controls. Never add production credentials to workflows.

Before review, synchronize the exact reviewed spec pair; check provenance, subset equality, credentials and local links; compile, lint and test Rust on the designated Runner. Record actual SHA, Build and results. Unbound specs, failures and skipped checks do not count as passes.

Real-server validation is separately authorized: login, list a permitted library and movie, prepare with truthful capabilities, read at most 1 MiB, explicitly end playback and log out. Do not print credentials, ticket paths, media names or response bodies; retain sanitized stage, status and byte counts only.

General OpenAPI generator compatibility with 3.1 remains unverified. Rust wire models are typed wrappers for the documented subset; generated schema fingerprints and an explicit typed-wire shape map are checked against the synchronized client contract. A field change requires reviewing the Rust model and its expected map together. Tests also inspect the same public spec. Other-language SDKs and general generated clients are future work.

Merge, release, package publication, tags and production changes require the release owner's explicit authorization. The candidate workflow publishes no packages.
