# Game4∞ JSON manifest v1

The public format is specified in [manifest v1](../../docs/manifest-v1.md).

- [JSON Schema 2020-12](v1.schema.json), identified by `urn:game4:manifest:1`.
- [Synthetic example](examples/v1.example.json): an `abc` file and an empty file.
- [Invalid cases](invalid-cases.json): shared by Rust semantic validation and offline Ajv schema tests.

Run `npm.cmd run manifests:test` to check the schema and examples offline after installing dependencies. These checks also run in the root `check` command and Windows CI. Rust tests round-trip the same example, validate the same rejection corpus, and test actual catalog export and safe file creation.

This folder is a public format specification, not an npm application package. The Rust serializer/parser lives in `apps/desktop/src-tauri/src/manifests/`. JSON Schema validates structure and local field syntax; the Rust validator additionally enforces u64 bounds, unique paths, ancestor conflicts, duplicate typed fields, and the 32 MiB serialized-document limit. `schemaValid` in the invalid corpus explicitly records this distinction.

No schema or referenced content is fetched over the network. The schema and example ship in this repository; publication on GitHub awaits the owner’s push. The URN identifies the format and is not a fetchable URL.
