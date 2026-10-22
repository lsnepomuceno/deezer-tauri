# 0001: Spec-driven development

- Status: accepted
- Date: 2026-10-22

## Context

Deezer Tauri is small, but most of what it does happens against a web page it does not control and on three operating systems, two of which its author does not run day to day. Behavior has to be written down before it is built, and verified by tests that run on all three platforms, or regressions will only be found by users. AI coding agents are part of the workflow.

## Decision

Specs are the source of truth. Every feature starts with `requirements.md`, `design.md` and `tasks.md` in `specs/features/NNN-name/`. Every acceptance criterion has an ID and at least one test that names it. Code that needs behavior a spec does not describe updates the spec in the same change set.

## Consequences

- More writing up front, less guessing about what the web player and each engine actually do.
- `make check` and CI fail when a test fails on any platform; there is no "works on Linux" merge.
- Specs, ADRs, identifiers, docs and commit messages are in English.
