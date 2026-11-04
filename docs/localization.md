# Language and localization policy

Status: accepted policy. The desktop foundation implements English and Brazilian Portuguese using i18next and react-i18next; future features must maintain coverage.

## Repository and GitHub

Initially use English for README files, documentation, decision records, public project updates, issue and pull request templates when created, code identifiers, comments, and commit messages. Keep official names and quoted material in their original form when appropriate.

Do not maintain duplicate Portuguese documentation yet. A future translated documentation set should have an explicit synchronization process. The GPL license text remains unchanged. This policy does not require rewriting existing Git history or changing private conversation language.

## Application

Initial supported locales:

- English: `en`, default and fallback.
- Brazilian Portuguese: `pt-BR`.

Expose a language selector in settings and persist the choice locally. Changing the interface language must not modify collection data. An explicit saved choice wins over any future automatic language detection.

## Implementation requirements

- Store user-facing strings in locale resources using stable semantic keys.
- Avoid hardcoded UI copy and sentence concatenation; support interpolation and plural forms.
- Translate navigation, empty states, dialogs, accessibility labels, task states, and user-facing errors.
- Use stable Rust error codes with parameters; translate their presentation in the interface.
- Use locale-aware date and number formatting; serialize machine data in stable formats.
- Preserve original filenames, titles, user metadata, hashes, schema keys, and technical identifiers.
- Missing translations fall back to English. Check missing keys during development so fallback does not mask incomplete coverage.
- Use i18next with react-i18next for locale resources, fallback, interpolation, and pluralization support.

## Acceptance

Both locales must cover each delivered feature. Verify key parity, switching and persistence, fallback behavior, interpolated values, plural forms where used, and layout with longer text. Security and sharing explanations must have equivalent meaning in both languages.

Localization belongs to M0 and continues with every feature; it is not a final-release translation task.
