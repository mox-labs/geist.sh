# Matcher Engine & Data Model Patterns

> Synthesized from ripgrep (BurntSushi school) and serde (Tolnay school) mastery extracts.
> For x.uma/rumi matcher and policy serialization judgment.

---

## Table of Contents

1. [Matcher Engine Architecture](#1-matcher-engine-architecture)
2. [Two-Phase Matching: Candidate and Confirm](#2-two-phase-matching-candidate-and-confirm)
3. [Finite Automata Trade-offs](#3-finite-automata-trade-offs)
4. [Data Model Design](#4-data-model-design)
5. [Serialization Strategy](#5-serialization-strategy)
6. [Performance Patterns](#6-performance-patterns)
7. [Exhaustive Testing Methodology](#7-exhaustive-testing-methodology)

---

## 1. Matcher Engine Architecture

**The question for x.uma:** How should rumi structure its policy matching pipeline?

### Internal Iteration (Push, Not Pull)

ripgrep's `Matcher` trait uses push-based (internal) iteration. BurntSushi chose this because (1) some search implementations inherently require internal iteration and converting to external is "practically impossible," (2) Rust's type system isn't expressive enough for external iteration without sacrificing ease or performance. The `Sink` trait mirrors this: callbacks return `bool` -- `false` halts immediately for early termination with zero allocation overhead. **[ripgrep: Matcher trait, crates/matcher/src/lib.rs]**

**What Claude gets wrong:** Claude defaults to iterator-based (pull) APIs because they feel idiomatic. For a matcher engine, push is correct. Pull forces the engine to suspend and resume state, which is slower and harder to optimize with pre-filters.

### Decompose by Concern, Not Feature

ripgrep's 10 crates decompose by concern boundary: `matcher` (abstract interface), `searcher` (buffer management + strategy), `printer` (output formatting), `regex` (one of N backends), `globset` (standalone pattern library). A facade crate re-exports everything. **[ripgrep: Insight 7]**

### Application to x.uma

rumi should decompose into: (1) abstract matcher trait (policy-agnostic), (2) policy-specific matcher impls (ACL, rate-limit, content), (3) evaluation driver (walks requests, pushes matches), (4) result formatting (audit, deny response, metrics). The matcher trait should use push-based iteration with early-termination booleans and must not leak regex or glob assumptions.

---

## 2. Two-Phase Matching: Candidate and Confirm

**The question for x.uma:** How should rumi structure fast-path policy checks?

### The LineMatchKind Pattern

```rust
pub enum LineMatchKind {
    Confirmed(usize),  // Definitely matches
    Candidate(usize),  // Might match -- needs verification
}
```

`find_candidate_line` allows a fast pre-filter (vectorized `memmem`) to produce candidates. The full regex confirms. **False positives are allowed; false negatives are forbidden.** **[ripgrep: Insight 3]**

This pattern recurs at every layer:

| Layer | Candidate | Confirm |
|-------|-----------|---------|
| Inner literals | `memmem` finds substring | Full regex validates |
| Word boundary | `(^|\W)(regex)(\W|$)` fast path | Capture engine extracts match |
| Ngram indexing (RFC) | Index says "file MAY contain" | Full search confirms |
| serde tagged enums | Peek at tag via `Content` buffer | Full struct deserialization |

**What Claude gets wrong:** Claude suggests a single matching phase. For policy matching at scale, two-phase leaves 10x+ performance on the table. Most policy rules have literal prefixes (path prefixes, header names, method strings) that can be checked with vectorized search before the full matcher.

### Protocol Obligation

The contract: pre-filters MUST NOT produce false negatives. Missing a deny-rule match is a security vulnerability. `non_matching_bytes` has the same one-directional correctness: always err toward the safe side. **[ripgrep: Cross-Codebase Pattern]**

### Application to x.uma

rumi should have two-phase evaluation: (1) fast candidate filter on literal policy prefixes using optimized substring/set matching, (2) full policy evaluation on candidates only. For deny-first policy: a missed deny candidate is a security bug. For allow rules: a missed candidate means unnecessary denial (fail-closed -- safe but noisy).

---

## 3. Finite Automata Trade-offs

**The question for x.uma:** When should rumi use DFA vs NFA vs literal matching?

### The Optimization Hierarchy

ripgrep's pipeline of speculative optimizations, each with fallback:

1. **Literal extraction** -- `memmem` for constant substrings
2. **DFA** -- O(n) matching, no backtracking
3. **Lazy DFA** -- compile states on demand, avoids state explosion
4. **NFA** -- bounded-memory fallback
5. **Capture engine** -- slowest, only when capture groups needed

Each layer bails out to the next. **[ripgrep: Executive Summary, Insight 3]**

**What Claude gets wrong:** Claude recommends "just use regex" or "just use a trie." The right answer depends on pattern complexity:

| Approach | Wins | Loses |
|----------|------|-------|
| Literal set (Aho-Corasick) | Many exact strings | Wildcards or regex |
| DFA | Predictable patterns, O(n) | State explosion on Unicode |
| Lazy DFA | Large pattern sets | Worst-case degrades |
| NFA | Bounded memory, complex patterns | Slower per-character |

BurntSushi: if the regex engine is already "accelerated," skip custom literal extraction -- unless Unicode word boundaries force a slower path. Also: the inner literal extractor was "far more important in the old days" before the regex crate grew its own optimization. Keep the infrastructure -- you'll need it when upstream changes. **[ripgrep: Insight 3, commit ca740d9]**

### Application to x.uma

rumi's policy patterns have distinct tiers: **exact match** (method = GET): hash set; **prefix/glob** (path starts with /api/v2/): trie; **regex** (path `^/users/[0-9]+/.*$`): lazy DFA with NFA fallback. Build the pipeline: exact first, then glob, then regex. Document the performance contract per tier.

---

## 4. Data Model Design

**The question for x.uma:** How should rumi define its policy schema?

### Serde's 29-Type Ontology

Serde defines exactly 29 types. Why not fewer: collapsing types (merging `struct` and `map`) loses information binary formats need. Why not more: every new type is a method every format implementor must handle -- adding type #30 breaks every existing format. **[serde: Core Abstractions]**

**What Claude gets wrong:** Claude suggests either open-ended schemas ("just use JSON") or overly abstract trait systems. Open-ended schemas defer validation to evaluation time -- in a security context, this means policies silently fail. Overly abstract trait systems make every new policy dimension a breaking change. Serde's lesson: define a bounded type system. Enumerate policy element types explicitly.

### The Data Model Constrains Everything

Three serde feature categories have been open 7+ years because they violate the data model: `flatten` demotes structs to maps, tagged enums + `other` requires `Content` buffering, `serialize_with` inside containers needs stateful deserialization. Features within the model are trivial; features that "see around" it stall indefinitely. **[serde: Issues #912, #941, #723, #1346]**

### Backward-Compatible Evolution

New data model types via default methods that return errors. Existing implementors compile unchanged. New capability is opt-in. **[serde: Issue #1136]**

### Application to x.uma

Define rumi's policy data model as a bounded enumeration: primitives (string, integer, bool, IP, CIDR, port range), containers (list, map, set), policy-specific (glob, regex, time window, identity reference). Every matcher backend handles all types. New types use default-error methods. Validate at load time, not evaluation time. Features that violate the model (dynamic cross-field validation, context-dependent types) belong outside the core matcher.

---

## 5. Serialization Strategy

**The question for x.uma:** When should rumi derive vs write custom Serialize/Deserialize impls?

### Decision Framework

| Situation | Strategy |
|-----------|----------|
| Fields map 1:1 to wire format | `#[derive(Serialize, Deserialize)]` |
| Field names differ | `#[serde(rename = "...")]` |
| Post-deserialization validation | `#[serde(try_from = "RawType")]` + TryFrom |
| Internal repr differs from wire | Custom impl |
| Enum representation matters | `#[serde(tag = "type")]` -- but beware flatten |

**What Claude gets wrong:**

1. **Over-recommends `flatten`.** Flatten is serde's most complex feature. It demotes structs to maps, was reverted with `deserialize_in_place`, requires double buffering with tagged enums, and breaks non-self-describing formats (RON, Postcard, MessagePack). **[serde: Insight 4, Issue #1346]**

2. **Under-recommends `try_from`.** Serde deliberately stops at "produce a Rust value from data." Post-deserialization validation via `try_from` is the intended escape hatch, not a workaround. **[serde: Issues #642/#939/#1118]**

### Zero-Copy via Lifetime

`Deserialize<'de>` enables borrowing from input bytes. A single missing `#[inline]` on `DeserializeSeed` caused 2-5% regression -- zero-cost abstractions live on a knife edge. **[serde: Issue #650]**

### Application to x.uma

Use `#[derive(Deserialize)]` for policy files. Use `try_from` for policy validation (e.g., "deny rules must have at least one condition"). Never use `flatten` for policy composition -- compose at the application layer. Consider zero-copy deserialization from mmap'd policy files for hot-path matching. Prefer self-describing formats (TOML, JSON) so `deserialize_any` works.

---

## 6. Performance Patterns

**The question for x.uma:** How should rumi achieve reliable, measurable performance?

### Measurement Over Convention

BurntSushi includes `hyperfine` benchmarks inline in commit messages. Key findings:

| Convention | Reality | Evidence |
|-----------|---------|---------|
| mmap is faster for large files | Usually worse for directory traversal | Default `MmapChoice::Never` |
| Micro-benchmarks predict production | bytecount's superiority was "an illusion" on 13GB data | Commit 6cdb99e |
| Algorithm optimization is primary | Buffer 8KB->64KB = 14.6% speedup | Commit 46fb77c |
| BFS is fine for parallel traversal | BFS 1GB vs DFS 50MB for wide dirs | Issue #1550 |

**What Claude gets wrong:** Claude optimizes the matching algorithm first. Check I/O granularity first -- a buffer size change (one constant) yielded 14.6% that no algorithm benchmark would predict. Claude also suggests mmap for "zero-copy performance." mmap is the right default almost never. ripgrep auto-enables mmap only for <=10 files, never on macOS. **[ripgrep: Insights 1, 4]**

### Optimization-Mode Conflict

A throughput fix for `--after-context` (loop `read()` to fill buffer) broke `--line-buffered` mode (must NOT wait for full buffer). Performance optimizations can conflict with behavioral contracts. **[ripgrep: Insight 6]**

### Compile-Time Discipline (Serde)

When ubiquitous, compile overhead is everyone's problem. Custom `tri!` macro over `?`: 9% compile-time improvement. `serde_core` split for parallel compilation. **[serde: Insight 1, RFC 3058]**

### Application to x.uma

(1) Benchmark with `hyperfine` on real policy files, not synthetic patterns. (2) Measure I/O granularity before optimizing the matcher. (3) Never default to mmap. (4) Test every optimization against every mode (batch load, hot reload, single check). (5) If rumi becomes foundational, apply compile-time discipline: minimize dependencies, consider crate splitting.

---

## 7. Exhaustive Testing Methodology

**The question for x.uma:** How should rumi achieve security-grade reliability?

### BurntSushi's Debugging Method

From live debugging in Issue #1335: (1) reproducible corpus, (2) strace syscall tables, (3) user vs system time to distinguish I/O from computation, (4) `perf record --call-graph`, (5) binary search on corpus to isolate, (6) minimal fix. The fix was one range change -- two components that worked independently interacted perversely. **[ripgrep: Issue #1335]**

### Error Message Testing (Serde)

118 `.stderr` snapshots lock down compiler error output. When `serde_core` was extracted, `diagnostic::on_unimplemented` was immediately added to prevent internal paths in error messages. **[serde: Insight 3, PR bf9ebea3]**

**What Claude gets wrong:** Claude tests happy path + edge cases. For a security matcher:

1. **Contract testing**: property-based tests asserting pre-filters are strict supersets of full matchers. Generate random inputs, assert fast path never misses a match.
2. **Interaction testing**: test policy combinations, not individual rules. The ripgrep buffer regression (oversized buffer penalizing subsequent files) was invisible in unit tests.
3. **Error message testing**: deny messages ARE the user interface. Snapshot-test them.
4. **Mode interaction testing**: hot-reload and batch-processing may have conflicting I/O assumptions.

### Principled Feature Resistance

BurntSushi resisted multiline search for 7 years (Issue #176, 103 comments). When implemented: behind a flag, never default. "There HAS to be a point at which 'write code for your specialized search' becomes a valid thing to say." Tolnay: "mature projects naturally become increasingly selective about feature work." **[ripgrep: Insight 8; serde: Issue #1723]**

### Application to x.uma

(1) Property-based tests: candidate filters are supersets of full matchers. (2) Interaction tests: deny + allow + rate-limit on same path. (3) Snapshot tests for deny messages. (4) Mode interaction tests. (5) Documented live debugging protocol (strace, perf, corpus binary search). Feature scope: rumi matches policies, not interprets them. Evaluation logic belongs in x.uma's evaluation layer. Resist adding evaluation to the matcher crate.

---

## Source Traceability

| Concern | ripgrep | serde |
|---------|---------|-------|
| Push-based iteration | Matcher + Sink traits | Serialize/Serializer dispatch |
| Two-phase matching | LineMatchKind, Insight 3 | Content buffering for tagged enums |
| Crate decomposition | 10 crates by concern | serde_core extraction, PR f3869307 |
| Data model design | -- | 29-type ontology, Issues #912/#941/#1346 |
| Performance measurement | Insights 1, 4; commits 46fb77c, 6cdb99e | tri! macro, serde_core split |
| Error message craft | -- | 118 .stderr snapshots, diagnostic attrs |
| Feature resistance | Issue #176 (7 years), #152, #196 | Issue #1723, #642/#939/#1118 |
| Backward-compatible evolution | -- | Default methods returning errors |
| Flatten hazards | -- | Insight 4, Issue #1346, revert c23be3f8 |
| Optimization conflicts | Insight 6, commits 8c6595c/d47663b | Flatten + in-place revert |

**Full extracts**: `~/oss/research/ripgrep-mastery.md`, `~/oss/research/serde-mastery.md`
