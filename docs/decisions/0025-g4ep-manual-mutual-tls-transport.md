# 0025: Manual private Guardian transport with mutual TLS

Date: 2026-11-14. Status: accepted for G4EP step 5 private MVP.

Reuse Tokio, Rustls, Guardian storage/permissions, aggregate rate gates and operation cancellation. Keep normal BitTorrent sessions separate: they do not alone authenticate enrolled Guardian identities or enforce local per-fragment admission.

Implement direct TCP with mutual TLS 1.3, exact enrolled certificate fingerprints, node identity and inbound IP checks. Explicitly exchange public profiles; keep private TLS keys outside fragment stores. Start no listener or discovery by default. Offer only request-scoped availability for known descriptors, never a full inventory broadcast. Bound peers, framing, fragment sizes, connections, attempts, deadlines and buffers.

Use existing atomic verified StorageNode commits for receive; retain current permission and quota checks on both ends. Stop and participation disable interrupt operations. Keep publisher signature trust separate from transport identity. There is no central transfer server or public DHT bootstrap; later discovery may propose endpoints but cannot enroll trusted peers.

See [protocol and two-computer procedure](../g4ep/direct-network.md). Real loopback and independent-process evidence must not be represented as completed physical two-computer acceptance.
