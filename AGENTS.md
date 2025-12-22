# Rad Roots - Code Directives

## Rust Code Directives
- Toolchain: Rust 1.86, edition 2024; use workspace versions from the root Cargo.toml.
- Portability: preserve no_std patterns; gate std usage with cfg(feature = "std") and use alloc when needed.
- Safety: avoid unsafe; prefer safe, explicit APIs. Add #![forbid(unsafe_code)] on new crates/modules.
- Public API: keep Radroots* prefix; avoid hidden panics; return Result/Option for fallible ops; use precise error enums (thiserror where appropriate).
- Features: keep serde/typeshare/ts-rs derives behind existing feature gates and in the current style; ensure feature combinations compile (no_std, std, wasm).
- Generated outputs: treat */bindings/ts/src/types.ts as generated; do not hand-edit.
- Performance: borrow over clone, avoid intermediate allocations, preallocate when sizes are known, and prefer iterators over indexing loops.
- DRY: consolidate shared logic into core/types/events-codec or dedicated helpers.
- Parity: maintain feature parity across native/wasm layers when adding SQL or Tangle APIs.
- Module layout: keep lib.rs as a module manifest and re-export surface; avoid heavy logic in lib.rs.
- Testing: add or update unit tests for new behavior and edge cases, especially around parsing, invariants, conversions, and rounding.

## TypeScript Code Directives
- Types: use strict TypeScript; do not use any, unknown casts, or weaken types; prefer explicit interfaces and types; use enums or literal unions when clear; use named exports and avoid default exports.
- Naming: variables and functions use snake_case; types, interfaces, and enums use PascalCase; prefer layered prefixes to namespace meaning: domain_object_action, e.g. nostr_keys_gen(), NostrKeysGen; do not enforce naming conventions in _env*.ts files; constant data structures must use ALL_CAPS_SNAKE_CASE.
- Source code: keep code deterministic and reproducible; do not add source code comments; single-line if statements must not use braces; anchor comments are allowed only when they start with @ (e.g. // @todo); /* */ blocks are allowed only to disable features during development and must not include descriptive text; <!-- --> HTML blocks are allowed only to disable features during development or compiler/lint suppression (e.g. <!-- svelte-ignore ... -->).
- Modularity: do not duplicate logic; put shared or generalizable code in packages/; apps should rely on packages/ for shared utilities; treat @radroots/*-bindings as generated from .rs crates and do not edit or format their .ts outputs, change upstream .rs or report errors instead; every class must implement a same-name interface prefixed with I, all public methods must be declared on the interface, and method return types are required for all class methods.
- Architecture: prefer pure functions; prefer composition over inheritance; avoid side effects unless required; avoid global mutable state.
- Change policy: apply breaking changes when needed; do not add legacy or shim fixes.
