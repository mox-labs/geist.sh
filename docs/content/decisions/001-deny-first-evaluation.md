---
title: "ADR-001: Deny-First Evaluation"
date: "2026-02-19"
status: accepted
---

## Context

geist-edge needs an evaluation strategy for access control decisions. When a request arrives and policy evaluation occurs, the system must decide how to handle:

- Requests that match an explicit ALLOW rule
- Requests that match an explicit DENY rule
- Requests that match no rule at all

This is a foundational security decision that affects every processor that makes authorization decisions.

## Decision

**Deny-first evaluation.** If no rule explicitly permits an action, it is denied.

The evaluation order is:

1. Check DENY rules first — any match immediately denies
2. Check ALLOW rules — a match permits
3. No match → **deny** (the default)

## Rationale

This decision has broad consensus across the industry:

- **Cedar** (AWS authorization policy language) — default deny, explicit allow required
- **Envoy RBAC** — deny by default when no policy matches
- **OWASP** — "deny by default" is a core secure design principle
- **Zero Trust architectures** — never trust, always verify

The alternative — allow-first / default-allow — requires enumerating everything that should be blocked. This is inherently fragile: any unenumerated case is permitted.

## Consequences

**Positive:**
- Unknown states fail closed — the safest possible default for agent governance
- Policy authors must explicitly grant access — no accidental over-permission
- Consistent with zero-trust principles appropriate for autonomous agents

**Negative:**
- Requires explicit ALLOW rules for every permitted action — more configuration upfront
- New capabilities are unreachable until policy is written — deliberate friction

**Accepted tradeoff:** The friction of requiring explicit permission is a feature, not a bug. For a system governing autonomous agents with shell access, filesystem reach, and network capability, implicit permission is the wrong default.
