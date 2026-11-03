# Risk register

Initial qualitative assessment: November 3, 2026. Technical review owner: project maintenance. Budget and publication decisions: project owner.

| ID | Risk | Impact | Mitigation / review trigger |
|---|---|---|---|
| R01 | Scope exceeds capacity | High | Deliver library before integrations; review each milestone |
| R02 | Incompatible or unsuitable P2P engine | High | Real prototype and license review at M3; fallback: local release |
| R03 | Custom formats obstruct interoperability | High | Start with JSON and `.torrent`; specify and test before stabilization |
| R04 | Few peers and disappearing collections | High | Show unavailability; later guardians and restoration testing |
| R05 | Manipulated availability metrics | High | Timestamp observations; no rewards for untrusted metrics |
| R06 | Disk, path, or parser failures | High | Security review, limits, and interruption scenarios |
| R07 | Costs or dependency on paid networks | High | Optional integration, budget, and tested retrieval |
| R08 | Lost curator keys | High | Design rotation and recovery before production signatures |
| R09 | Fragile Windows setup and installer | Medium | Clean build and installation at M0 and M5 |
| R10 | Incorrect metadata or unauthorized redistribution | High | Record declared provenance; separate trust from integrity |
| R11 | Estimates lack execution history | High | Reestimate after M0/M1; active weeks and explicit contingency |
| R12 | Incomplete or inconsistent translations | Medium | Shared locale keys, parity checks, and review of both languages per delivery |

When a risk materializes, record the occurrence, specific owner, and action. Schedule and architecture changes should reference the risk prompting review.
