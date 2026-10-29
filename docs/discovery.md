# LAN discovery integration guide

This describes the RC.9 target server contract, awaiting final source binding and designated-server acceptance. StreamixKit does not implement a UDP client in this phase. A player can integrate the protocol into its existing LAN-discovery component.

Send UTF-8 `who is StreamixServer?` to UDP **7359** on the user's selected local network. The server trims surrounding whitespace and matches the whole question case-insensitively; other product questions are ignored. Requests are at most 64 bytes. Replies are unicast UTF-8 JSON of at most 512 bytes.

Replies contain `Address`, `Id`, `Name`, `EndpointAddress:null`, `Product` and `Version`. `Product` is `StreamixServer`; `Id` is the UUID from capabilities `server.id`. `Address` contains the reachable configured origin/port, normally HTTP 8970. Never assume a container-private address is reachable. Discovery can be disabled, suppressed, rate-limited or require an advertised address.

Treat every reply as an **untrusted hint**. Bound packet length/parsing, require the expected product and a valid UUID, reject credentials in the address and show the proposal to the user. After explicit origin/protocol approval, fetch public capabilities and compare identity. A matching ID does not permit moving an existing Bearer; use the normal trusted-origin login flow.

Discovery belongs to private/link-local networks, not a public directory. Malformed/oversized questions, public-origin packets and excess requests receive no useful reply. Avoid tight retries. Silence may mean disabled discovery, a firewall or container networking, not a broken server. Manual address entry remains available using [connection conventions](connecting.md).

Server network binding and container deployment are separate configuration. Kit does not change host ports, start listeners, manage interfaces or add mDNS/DNS-SD/IPv6 multicast.
