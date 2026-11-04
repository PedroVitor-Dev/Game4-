# Decentralization and funding

## Direction

Essential features do not require an account, wallet, token, or payment. BitTorrent is the initial transport. Curator identity, file integrity, availability, and funding are separate concerns.

## Guardian Mode and rescue

Plan explicit per-collection participation, disk quotas, upload/download limits, schedules, and voluntary withdrawal. Never delete personal files to free quota. Explain what remains shared when a collection is paused or removed.

Show observed sources, last observation, and recent verification. Assess risk from those signals without claiming a global copy count. Prioritize content with few sources and avoid all guardians choosing only popular items. Resistance to fake identities and false reports requires research before economic incentives.

## Signed catalogs

Signatures verify integrity and association with a key. Plan key export, rotation, recovery, and revocation. Trust is local: users choose curators and communities, without a mandatory global editorial authority.

## Integrations

| Technology               | Proposed use                                    | Adoption gate                                                       |
| ------------------------ | ----------------------------------------------- | ------------------------------------------------------------------- |
| IPFS                     | Complementary manifest and content distribution | Test pinning, restoration, and resource usage                       |
| OpenTimestamps / Bitcoin | Manifest timestamp proofs                       | Preserve and verify proofs with manifests                           |
| Bitcoin / Lightning      | Voluntary project or collection donations       | Prefer external flows; no app custody of keys                       |
| Filecoin                 | Funded additional copy                          | Experiment covering agreement, duration, cost, and actual retrieval |

IPFS requires nodes to retain data; content addressing does not guarantee persistence. OpenTimestamps neither stores games nor certifies authorship. Filecoin requires storage and retrieval planning; do not promise permanence.

## Collection sponsorship

Future hypothesis: publish a goal, budget, period, responsible parties, closure policy, and verification reports. Received funding does not automatically prove storage. Start with transparent processes and external contributions before considering smart contracts.

## Initial exclusions

No native token, NFTs, device mining, custody, financial yield, or automatic traffic payments. These mechanisms would increase fraud exposure and distract from verifiable restoration.

## Technical references

- [IPFS persistence](https://docs.ipfs.tech/concepts/persistence/)
- [OpenTimestamps](https://opentimestamps.org/)
- [Filecoin specification](https://spec.filecoin.io/)
- [BTCPay Server](https://docs.btcpayserver.org/FAQ/General/)

References consulted for planning on November 3, 2026. Revalidate APIs, costs, licenses, and terms when implementing an integration.
