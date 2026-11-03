# ADR 0001 — Local foundation and progressive decentralization

Date: November 3, 2026. Status: accepted initial direction.

## Context

The repository starts with a README and license. The vision includes file preservation, P2P, and decentralized technologies. Implementing every integration immediately would increase the risk of never delivering a usable tool.

## Alternatives

1. Build a financial platform with a token and incentives first.
2. Depend on a central service for catalog, identity, and files.
3. Deliver a local library and evolve decentralization through modules.

## Decision

Adopt option 3. Follow the README's stack: Tauri 2, Rust, React, TypeScript, Tailwind CSS, and SQLite. Start with BitTorrent, selecting an engine after experimentation. Keep formats open, support offline exports, and require explicit sharing.

IPFS, timestamp proofs, donations, and paid storage are optional later integrations. No native token, custody, or automatic rewards in the foundation. The user controls push; work is recorded through Conventional Commits.

## Consequences

The first version is smaller than the full vision. Import, integrity, and restoration can be validated before external networks become dependencies. The core needs clear boundaries for future adapters. Engine, format, and key-management details remain open.

## Evidence

Direction based on the README and planning alignment with the project owner. No prototype, benchmark, or operational stack validation exists in this repository yet.
