# Desktop IPC contracts

Rust owns the DTO definitions in `apps/desktop/src-tauri/src/contracts/`. ts-rs generates `src/generated.ts`; Rust serialization generates `examples.json`. Both artifacts are committed and checked for drift. `src/validation.ts` validates active command responses and sanitizes errors at runtime.

The desktop consumes this private TypeScript-source package through the npm workspace. It is not a published package or a network service.

```powershell
npm.cmd run contracts:generate
npm.cmd run contracts:check
npm.cmd run check
npm.cmd run rust:test
```

Do not edit generated artifacts. Update Rust DTOs, regenerate, update runtime validators and both error translations where needed, and run the checks. Generated files are excluded from Prettier because the exporter owns their exact contents.

`ImplementedCommand` and the client expose `app_info`, `database_info`, `library_select_sources`, `library_import`, `library_list`, `library_item_files`, `tasks_list`, `task_get`, `library_export_manifest`, `library_save_backup`, `library_prepare_restore`, and `library_restore_backup`. Runtime decoders validate all active responses. `library_verify` starts an idempotent background recheck and `library_integrity` reads latest copy observations. `task_cancel` and `DRAFT_TASK_UPDATED_EVENT` remain drafts; they do not cancel tasks or emit events.

See [the protocol specification](../../docs/ipc-contracts.md) for versioning, types, command mapping, semantic validation, and future prerequisites.
