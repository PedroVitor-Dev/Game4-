# ADR 0011: Evidence-based file/platform metadata and exact sizes

Date: November 4, 2026

Status: Accepted initial detector

## Context

Preservation users need useful platform/format hints and readable sizes without conflating filename guesses, declared editorial metadata, integrity, or authenticity. Identical content can appear under different names; archives/disc images are often platform-ambiguous. Existing manifests and backups must remain usable.

## Decision

Capture at most 64 KiB during the existing streaming SHA-256 pass. Store checked content-bound header observations in schema v4; derive extension hints from relative paths. Prefer matching headers and suppress contradictory filename platform guesses. Preserve declared platform separately. Present evidence and unknown states explicitly, alongside per-item/per-file exact bytes and IEC summaries using integer arithmetic.

Add a paginated, validated file-details IPC capability. Keep manifest v1 core unchanged and export ignorable namespaced identification metadata. Extend private backups to v2/schema v4 while continuing to read v1/schema v3 without fabricated signature claims. This extends [ADR 0010](0010-library-backup-restore.md)'s format compatibility policy.

## Consequences and alternatives

No second disk scan, execution, archive unpacking, or network lookup is needed. Old catalogs receive filename-only suggestions until explicit reimport. Signatures identify a header pattern, not full format validity or authenticity. Case-insensitive extension rules are intentionally weaker. Automatically overwriting declared metadata, inferring PlayStation from every ISO/BIN, guessing Mega Drive from `.md`, and storing filename guesses on content identities were rejected.

Current scope and validation evidence are recorded in [file identification](../file-identification.md). Broader format parsing, manual metadata editing, independent integrity rechecks, and identification refresh workflows remain separate work.
