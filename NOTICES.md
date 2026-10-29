# Third-party notices

StreamixKit is commercially licensed. Dependencies retain their own licenses; this repository does not relicense them.

The candidate Rust manifests use reqwest (MIT OR Apache-2.0), serde and serde_json (MIT OR Apache-2.0), Tokio (MIT), uuid (MIT OR Apache-2.0), chrono (MIT OR Apache-2.0), and zeroize (Apache-2.0 OR MIT). Server CI must resolve and freeze the actual dependency graph and retain dependency license texts before a distributable build. This is not a completed binary license inventory.

Python tools use the standard library. The synchronized specifications remain subject to the upstream Streamix commercial copyright.

API signatures checked against [reqwest 0.12.24 ClientBuilder](https://docs.rs/reqwest/0.12.24/reqwest/struct.ClientBuilder.html) and [Response](https://docs.rs/reqwest/0.12.24/reqwest/struct.Response.html). No implementation text is copied from those pages.
