# Game4 package format

The [public .g4 v1 specification](spec-v1.md) defines a stored ZIP/ZIP64 containing a strict package descriptor, Game4 manifest v1 and exact original payload bytes. Readers verify all SHA-256 hashes and sizes without extracting or executing files.

- [Descriptor schema](v1.schema.json)
- [Synthetic fixtures](fixtures/manifest.json), independently generated with Python
- [Manifest v1 schema](../manifest-schema/v1.schema.json)
- Rust implementation: `apps/desktop/src-tauri/src/packages/`

```powershell
npm.cmd run g4:test
npm.cmd run g4 -- create --manifest "C:\Collection\manifest.json" --root "C:\Collection" --output "C:\Exports\collection.g4"
npm.cmd run g4 -- read "C:\Exports\collection.g4"
```

The CLI operates offline and does not access the library database. `create` expects original files under the root at their exact manifest-relative paths. It refuses existing destinations and fails if source bytes differ from the manifest. `read` checks the entire package and outputs the embedded manifest and exact total size as JSON. A nonzero exit code indicates failure. Error codes and supported limits are documented in the specification.

In the desktop Library, **Create .g4** exports an item using verified source bytes; **Open .g4** performs a full check and displays a summary. Neither operation automatically shares content. Package files are ignored by Git except for the public synthetic fixtures.
