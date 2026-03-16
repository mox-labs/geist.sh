# Branch: feat/benchmark

13 commits since main. Covers P1 (core types) through P1.5 (typed extension registry + access control) plus renames and docs.

## Commits (oldest → newest)

### P1: Core Types
1. `590f831` **feat(edge): implement Phase 1 core types** — Processor trait (`&self`, `PhaseResult`), Sequence compositor, `ProcessingMode` (headers-only, full), deployment models doc. 67 tests.
2. `1882632` **refactor(edge): address guild review** — rename, invariants, docs cleanup.

### Docs
3. `bbfdcd1` **docs: capability-led connectivity model + extension registry** — `.claude/docs/` with full processing pipeline model, typed extension registry pattern, updated Rust mastery + gateway patterns skills.

### P1.5: Extension Registry
4. `4f6b9c2` **refactor: delete geist-policy** — removed 1,905 lines of monolithic policy engine, replaced by extension model.
5. `f16c460` **feat(edge): typed extension registry** — `TypedConfig`, `TypedRegistryBuilder`, `inventory`-based self-registration. 459 lines.
6. `e61f343` **fix(edge): guild review** — macro improvements, duplicate detection, docs.

### P1.5: Access Control Extension
7. `71e0c89` **feat(edge): add geist-access-control processor** — first extension crate. Policy → rumi matchers at construction. 608 lines.
8. `ed4e520` **fix(edge): guild review** — JSON injection fix, visibility, dead code.

### P1.5: Composition Root
9. `62cfdfa` **feat(edge): wire composition root** — `geist-sh` binary links extensions, builds pipeline from `TypedConfig` list.
10. `dc62266` **docs: mark P1.5 as done**

### Renames
11. `2de6c83` **refactor: rename geist-access-control → geist-acl** — shorter crate name.
12. `69a1095` **refactor: rename binary geist → geist-sh** — avoid clash with crate name.

### Experience
13. `bcccded` **docs: add gestalt.mox.nexus experience vision** — `.claude/envisioning/gestalt-experience.md`.

## Architecture Summary

```
geist-edge (core)
├── Processor trait (&self, Arc-shareable)
├── Sequence compositor (deny-first, phase-based)
├── PhaseResult (Continue/Deny/Error + mutations)
├── ProcessingMode (headers-only, full)
├── Typed extension registry (type_url → factory)
│   └── inventory::submit! for zero-code registration
└── adapter::axum (feature-gated HTTP adapter)

geist-acl (extension)
├── AccessControl processor
├── Policy → rumi matchers at construction
└── Self-registers via inventory

geist-sh (binary)
├── Composition root
├── Builds pipeline from TypedConfig list
└── axum server on :3000
```

## Test Count
- 48 tests across workspace (all pass)
- Core: sequence compositor, phase types, registry
- ACL: policy deserialization, evaluator logic, processor integration
- Adapter: axum routing, deny/allow/error responses

## What's NOT Here (Next Phases)
- P2: `http::` types refactor, axum adapter improvements, OTel, Docker (done on feat/cli-and-docs branch)
- P3: Composer (intent → capability selection)
- P4: CLI demo
- P5: geist-shell (Tauri)
