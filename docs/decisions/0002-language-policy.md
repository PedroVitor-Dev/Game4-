# ADR 0002 — English repository and bilingual application

Date: November 3, 2026. Status: accepted.

## Context

The project owner requested English and Portuguese support while keeping GitHub content entirely in English initially. The repository contains documentation, not application code.

## Alternatives

1. Maintain parallel English and Portuguese documentation immediately.
2. Use English for both repository and application only.
3. Use English repository content and plan a bilingual application from its foundation.

## Decision

Adopt option 3. Translate current repository documentation into English. Plan English (`en`) and Brazilian Portuguese (`pt-BR`) application resources, with English as default and fallback, a persistent selector, and locale coverage as a delivery criterion. Keep public technical records and future GitHub templates in English.

## Consequences

The repository has one documentation source of truth initially. UI work includes translation and review in both locales. M0 must establish localization infrastructure; its impact on estimates will be reviewed when implementation starts. No running application gains language support through this documentation-only change.

## Evidence

Explicit project-owner request on November 3, 2026. Brazilian Portuguese was selected as the initial Portuguese locale based on project context. Runtime behavior will be verified during implementation.
