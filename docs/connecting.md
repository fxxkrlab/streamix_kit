# Connecting to a server

The RC.9 source candidate targets persistent public server identity, conventional ports and LAN discovery. Final implementation, server CI and deployment are still pending; [SOURCE.json](../spec/SOURCE.json) provides the exact source binding. Kit includes a parser and discovery guide, not a UDP networking client.

Choose the trusted address and protocol explicitly. A bare host with HTTP selected receives port **8970**; HTTPS receives **443**. An explicit port always wins, including HTTP port 80. HTTPS normally terminates at a reverse proxy; no alternate direct HTTPS port is implied. Rust `ServerAddress` and `Client::with_address` implement this convention. `Client::new` remains HTTPS-origin-only.

Cleartext local HTTP requires `ConnectionMode::LocalHttp`, or `STREAMIX_ALLOW_LOCAL_HTTP=1` in the examples, for an origin the user approves on a trusted local network. Header credentials remain visible on cleartext HTTP; public/untrusted-network access should use HTTPS. There is no automatic TLS downgrade, ignore-certificate option or alternate-origin fallback.

API paths begin `/sxapi/v1`; returned stream paths can begin `/nimbus-play/v1`. Resolve them against the trusted origin. Input must have no userinfo, query, fragment or API-prefix path. Use standard URL brackets for IPv6.

Before login, capabilities can return optional `server` with persistent UUID `id`, `name`, `product` (`Streamix Server`) and software `version`. Do not confuse `server.version` with protocol `version=1.0`, or the public UUID with an internal streaming issuer identifier. Use `server.id` to group approved aliases for the same server. It survives upgrades/restarts; restoring a full backup can duplicate it.

The ID is **not proof of origin authenticity**. Never forward credentials to a newly discovered address merely because it presents a matching ID. Have the user approve that origin and re-establish its security and login state. Older servers can omit the object; keep them explicitly unidentified rather than deriving IDs from hostnames, accounts or devices. Current grants and feature discovery remain independent.

See [LAN discovery](discovery.md) for the candidate protocol. It supplies untrusted connection hints, not authentication or network administration.
