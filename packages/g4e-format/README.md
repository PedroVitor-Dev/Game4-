# Game4 Ever Manifest (.g4e)

The [public v1 specification](spec-v1.md) defines a portable JSON descriptor for P2P discovery and sharing. It references ordinary original files directly or an explicitly created `.g4`; packaging and payload duplication are optional. There is no mandatory encryption, DRM, account or central server.

- [Public JSON Schema](v1.schema.json), using the existing [manifest schema](../manifest-schema/v1.schema.json)
- [Original-file example](examples/original-files.g4e)
- [Optional-package example](examples/package.g4e)
- [Invalid conformance corpus](invalid-cases.json)
- [Original preservation policy](../../docs/original-preservation.md)

```powershell
npm.cmd run g4e:validate -- "path/to/collection.g4e"
npm.cmd run g4e:test
```

Validation is offline: it checks the descriptor and magnet/reference consistency without reading any referenced torrent or game file and without opening a network connection. A valid descriptor does not prove source availability or payload integrity. BitTorrent v1 identity uses SHA-1 of exact info-dictionary bytes; original-file integrity uses separate SHA-256 hashes. Native creation/reading and explicit offline byte verification are implemented; in-app network sessions/discovery remain pending. See [the workflow and CLI guide](../../docs/ever-manifests.md).

The former conceptual `.g4t` name is superseded by `.g4e`, not treated as an extension alias. `.g4` remains the optional ZIP/ZIP64 payload container. No existing package version or SQLite/backup schema changes.
