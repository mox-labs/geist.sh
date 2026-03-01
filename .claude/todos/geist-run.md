# geist-run — Implementation Plan

**What**: Agent runtime for geist.sh. Orchestrates geist execution via matrix DAG engine.
**Where**: `geist/run/` (Python package, uv managed)
**Depends on**: matrix (from `~/mox/cix/tools/matrix/`)

## Architecture

```
geist.sh (the shell)
├── geist-edge    — micro gateway (Rust). Governs, routes, logs. [DONE]
├── geist-run     — agent runtime (Python). Matrix + components + composition. [THIS PLAN]
└── geists/       — agent definitions (markdown). Capabilities, permissions, schedule.
```

geist-run is a Python package that:
1. Loads geist definitions (markdown with YAML frontmatter)
2. Builds component DAGs from geist capabilities
3. Executes via matrix Orchestrator
4. Exposes HTTP API (geist-edge proxies to it as upstream)

## Component Architecture

geist-run follows the same hex pattern as matrix/ix:

```
geist/run/
├── src/geist_run/
│   ├── domain/
│   │   ├── types.py            ← GeistDefinition, Beat, Briefing, CheckResult
│   │   ├── config.py           ← GeistRunConfig (Pydantic, subschema of full config)
│   │   ├── loader.py           ← Load geist markdown → GeistDefinition
│   │   └── ports/
│   │       └── _out/
│   │           ├── storage.py  ← Storage port (D1/SQLite abstraction)
│   │           └── notify.py   ← Notification port (Web Push abstraction)
│   ├── components/
│   │   ├── beat_checker.py     ← CheckBeat component (Haiku)
│   │   ├── briefing_gen.py     ← GenerateBriefing component (Sonnet)
│   │   ├── notifier.py         ← SendNotification component
│   │   └── conversation.py     ← HandleConversation component
│   ├── composition/
│   │   ├── __init__.py         ← create_service(), component registry
│   │   └── config.py           ← 3-tier config discovery for geist-run
│   ├── adapters/
│   │   ├── _in/
│   │   │   ├── api.py          ← HTTP API (the upstream geist-edge proxies to)
│   │   │   └── cli.py          ← CLI entry point
│   │   └── _out/
│   │       ├── storage/
│   │       │   ├── d1.py       ← Cloudflare D1 adapter
│   │       │   └── sqlite.py   ← Local SQLite adapter (dev/desktop)
│   │       └── notify/
│   │           └── webpush.py  ← Web Push adapter
│   └── __init__.py
├── tests/
├── pyproject.toml
└── geist-run.yaml              ← Default config
```

## Config Schema

geist-run owns its config subschema. Full config composes matrix + geist-run:

```yaml
# geist-run.yaml
matrix:
  runtime:
    model: "claude-haiku-4-5-20251001"  # default for beat checks
    max_tokens: 1024

geist-run:
  # Storage
  storage:
    type: "sqlite"              # or "d1"
    path: "./geist.db"          # for sqlite
    # d1_binding: "DB"          # for D1

  # Notifications
  notifications:
    enabled: true
    max_per_hour: 1
    quiet_hours: { start: 22, end: 7 }

  # Models (override per-task)
  models:
    monitor: "claude-haiku-4-5-20251001"
    briefing: "claude-sonnet-4-6-20260220"
    conversation: "claude-sonnet-4-6-20260220"

  # Geist loading
  geists_dir: "./geists"        # where markdown geist definitions live
```

Pydantic model:

```python
class StorageConfig(BaseModel, frozen=True):
    type: Literal["sqlite", "d1"] = "sqlite"
    path: str = "./geist.db"

class NotificationConfig(BaseModel, frozen=True):
    enabled: bool = True
    max_per_hour: int = 1
    quiet_hours: dict = {"start": 22, "end": 7}

class ModelConfig(BaseModel, frozen=True):
    monitor: str = "claude-haiku-4-5-20251001"
    briefing: str = "claude-sonnet-4-6-20260220"
    conversation: str = "claude-sonnet-4-6-20260220"

class GeistRunConfig(BaseModel, frozen=True):
    storage: StorageConfig = StorageConfig()
    notifications: NotificationConfig = NotificationConfig()
    models: ModelConfig = ModelConfig()
    geists_dir: str = "./geists"
```

3-tier discovery: defaults → `~/.geist-run/config.yaml` → `./geist-run.yaml`

## Components (Matrix DAG Nodes)

### Beat Monitoring DAG

```
CheckBeat → [if relevant] → GenerateBriefing → StoreBriefing → SendNotification
```

| Component | consumes | produces | Model |
|-----------|----------|----------|-------|
| `CheckBeat` | `{}` (root) | `geist-run.v1/beat.check` | Haiku |
| `GenerateBriefing` | `{geist-run.v1/beat.check}` | `geist-run.v1/briefing` | Sonnet |
| `StoreBriefing` | `{geist-run.v1/briefing}` | `geist-run.v1/briefing.stored` | none |
| `SendNotification` | `{geist-run.v1/briefing.stored}` | `geist-run.v1/notification.sent` | none |

### Conversation DAG

```
ParseIntent → RouteAction → [beat_create | beat_refine | query | feedback]
```

| Component | consumes | produces |
|-----------|----------|----------|
| `ParseIntent` | `{}` | `geist-run.v1/intent` |
| `CreateBeat` | `{geist-run.v1/intent}` | `geist-run.v1/action.result` |
| `RefineBeat` | `{geist-run.v1/intent}` | `geist-run.v1/action.result` |
| `AnswerQuery` | `{geist-run.v1/intent}` | `geist-run.v1/action.result` |

## Geist Definition Format

```markdown
---
name: journex
version: 0.1.0
description: Personal radar — monitors your beats and surfaces what matters.
schedule:
  check_interval: 1h
capabilities:
  - web_search
  - web_fetch
permissions:
  notifications: true
  storage: read_write
models:
  monitor: claude-haiku-4-5-20251001
  briefing: claude-sonnet-4-6-20260220
---

# journex

You are a personal monitoring agent. Users tell you what to track ("beats"),
and you watch, filter, and surface what matters — as push notifications with
interactive briefings.

## Monitoring

When checking a beat, search for recent developments. Focus on:
- New releases, announcements, RFCs
- Significant community discussions
- Breaking changes or migrations
- Policy/regulatory changes

Only flag as relevant if something genuinely new happened since the last check.

## Briefings

Structure every briefing as:
1. What happened (factual, 2-3 sentences)
2. Why it matters to the user (personalized based on their beat description)
3. Sources (linked, verifiable)

## Conversation

When the user messages you:
- Creating a beat: confirm understanding, ask clarifying questions
- Refining: acknowledge the refinement, explain what changes
- Feedback: adapt your monitoring approach
```

---

## Milestones

### M0: Scaffold + Config (Day 1)
- [ ] Create `geist/run/` package structure (hex arch, matching matrix/ix pattern)
- [ ] `pyproject.toml` with matrix dependency (path dep to `~/mox/cix/tools/matrix`)
- [ ] `GeistRunConfig` Pydantic model
- [ ] 3-tier config discovery (`discover_sources("geist-run")`)
- [ ] `load_config(GeistRunConfig, "geist-run")` wired up
- [ ] Verify: `uv run python -c "from geist_run.domain.config import GeistRunConfig; print('ok')"`

### M1: Domain Types + Geist Loader (Day 1-2)
- [ ] `Beat`, `Briefing`, `CheckResult` domain types (Pydantic, frozen)
- [ ] `GeistDefinition` type (parsed from markdown frontmatter + body)
- [ ] `loader.py` — parse markdown geist files → `GeistDefinition`
- [ ] Storage port (`Storage` protocol: get_beats, create_beat, store_briefing, etc.)
- [ ] Notification port (`Notifier` protocol: send)
- [ ] First geist definition: `geists/journex.md`
- [ ] Tests: loader parses markdown, types validate, ports satisfy protocol

### M2: Components — Beat Monitoring DAG (Day 2-3)
- [ ] `CheckBeat` component — uses ClaudeAgent (Haiku) to scan for beat activity
- [ ] `GenerateBriefing` component — uses ClaudeAgent (Sonnet) for full briefing
- [ ] `StoreBriefing` component — writes to storage port
- [ ] `SendNotification` component — sends via notifier port (respects quiet hours, rate limit)
- [ ] Wire DAG: `[CheckBeat, GenerateBriefing, StoreBriefing, SendNotification]`
- [ ] Test with MockAgent: full DAG executes, construct has all 4 artifacts
- [ ] Test with live Claude: single beat check end-to-end

### M3: Components — Conversation DAG (Day 3-4)
- [ ] `ParseIntent` component — classify user message (create_beat, refine, query, feedback)
- [ ] `CreateBeat` component — create new beat from user description
- [ ] `RefineBeat` component — update existing beat
- [ ] `AnswerQuery` component — answer questions about beats/briefings
- [ ] Wire conversation DAG with intent routing
- [ ] Tests: each intent type routes to correct component

### M4: Adapters — Storage + Notifications (Day 4-5)
- [ ] SQLite storage adapter (local dev / Tauri desktop)
- [ ] D1 storage adapter (Cloudflare production)
- [ ] Web Push notification adapter
- [ ] Tests: SQLite adapter CRUD, notification formatting

### M5: HTTP API + Composition Root (Day 5-6)
- [ ] HTTP API adapter (the upstream that geist-edge proxies to)
  - `POST /api/check-beat` — trigger beat check DAG
  - `POST /api/conversation` — trigger conversation DAG
  - `GET /api/beats` — list beats
  - `GET /api/briefings` — list briefings
  - `GET /api/briefings/:id` — full briefing
- [ ] Composition root: config → registry → adapters → service
- [ ] CLI: `geist-run serve` starts HTTP server
- [ ] CLI: `geist-run check` runs beat monitoring once (cron-friendly)

### M6: Integration — geist-edge + geist-run (Day 6-7)
- [ ] geist-edge config: upstream = geist-run HTTP API
- [ ] Full flow: client → geist-edge (auth, logging) → geist-run (matrix DAG) → Claude → response
- [ ] Docker Compose: geist-edge + geist-run + SQLite
- [ ] Cron: periodic `geist-run check` via scheduler

### M7: Geist Loading + Multi-Geist (Day 7+)
- [ ] Load geist definitions from `geists/` directory
- [ ] Geist-specific config overrides (models, schedule, capabilities)
- [ ] Multiple geists running simultaneously
- [ ] Geist lifecycle: load, activate, pause, archive

---

## Iteration Strategy

**Iteration 1 (M0-M2)**: Beat monitoring works end-to-end with mock + live agent.
Prove: matrix orchestrates journex components, construct ledger captures full trace.

**Iteration 2 (M3-M4)**: Conversation + storage + notifications.
Prove: user can create beats conversationally, get notified when something surfaces.

**Iteration 3 (M5-M6)**: HTTP API + geist-edge integration.
Prove: full governed flow — client through micro gateway to agent runtime and back.

**Iteration 4 (M7)**: Multi-geist, dynamic loading.
Prove: geist definitions are markdown files, new geists can be added without code changes.

## Dependencies

| Dependency | Source | How |
|------------|--------|-----|
| matrix | `~/mox/cix/tools/matrix` | Path dep in pyproject.toml |
| anthropic | PyPI | `anthropic>=0.49` |
| claude-agent-sdk | PyPI | For ClaudeAgent adapter |
| pydantic | PyPI | Config + domain types |
| uvicorn + starlette | PyPI | HTTP API adapter |
| python-frontmatter | PyPI | Geist markdown parsing |
| click + rich | PyPI | CLI (mox convention) |

## Verification

After each milestone:
1. `uv run pytest` — all tests pass
2. `uv run geist-run check --mock` — beat monitoring DAG executes with MockAgent
3. `uv run geist-run serve` — HTTP API starts, responds to requests
4. `cargo test --workspace` — geist-edge still passes (57 tests)
5. Full flow: `geist-sh` (edge) → `geist-run serve` (runtime) → mock/live agent
