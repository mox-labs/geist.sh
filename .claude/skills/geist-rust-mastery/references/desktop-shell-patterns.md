# Desktop Shell Patterns Reference for geist.sh HUD

> Synthesized from: tauri (IPC, plugins, events, webview management, resource lifecycle)
> Purpose: Non-security desktop architecture judgment for geist.sh's Tauri shell + HUD
> Scope: IPC routing, plugin composition, event system, webview management, resource lifecycle
> Complements: [security-architecture.md](security-architecture.md) covers deny-first ACL, trust boundaries, crypto

---

## Table of Contents

1. [IPC Architecture](#1-ipc-architecture)
2. [Plugin Composition Model](#2-plugin-composition-model)
3. [Event System](#3-event-system)
4. [Webview Management](#4-webview-management)
5. [Resource Lifecycle Across Process Boundary](#5-resource-lifecycle-across-process-boundary)
6. [State Management](#6-state-management)
7. [Main-Thread Obligation](#7-main-thread-obligation)
8. [Runtime Abstraction and Platform Escape](#8-runtime-abstraction-and-platform-escape)

---

## 1. IPC Architecture

**What Claude gets wrong:** Treating IPC as a single transport. Tauri proved that payload size determines the optimal transport, and the thresholds are platform-specific, not guessable.

### Size-Based Transport Routing

Tauri's `Channel<TSend>` routes payloads by measured thresholds, not heuristics:

| Payload | Platform | Transport | Measured Gain |
|---------|----------|-----------|---------------|
| JSON < 8KB | WebView2 v135 | Direct JS execution | 2x faster than fetch |
| Raw < 1KB | macOS WKWebView | Direct JS execution | 30% faster than fetch |
| Large payloads | All platforms | Fetch-based IPC queue | Avoids string overhead |

The constants `MAX_JSON_DIRECT_EXECUTE_THRESHOLD` (8192) and `MAX_RAW_DIRECT_EXECUTE_THRESHOLD` (1024) were benchmarked per-platform. The routing decision is: small payloads serialize into a JS string and execute via direct invocation; large payloads go through a fetch-based queue that avoids the cost of encoding large data as a JS string literal. (tauri insight #13, channel.rs)

### Channel Lifecycle Is Two-Sided

A Channel is not just a data pipe. It has a lifecycle protocol:
1. **Creation**: Rust creates `Channel<TSend>`, assigns a numeric ID
2. **Registration**: JavaScript registers a callback by channel ID
3. **Data flow**: Rust sends typed messages, JS receives via callback
4. **Termination**: Rust drops the channel, JS receives `{ end: true }` (see [Section 5](#5-resource-lifecycle-across-process-boundary))

The channel ID is the coordination point. If the JS side receives data for an unknown ID, it must buffer or discard. If the Rust side sends after drop, the behavior is undefined. (tauri insight #14, channel.rs)

### Application to geist.sh shell/

The x.uma microgateway talks to the HUD webview over IPC. Key decisions:

- **Measure thresholds for the specific webview engine** used in geist.sh's Tauri build (likely WKWebView on macOS). Do not reuse Tauri's constants -- they were measured for Tauri's JS bridge, not geist.sh's.
- **Policy evaluation results are small** (< 1KB typically). These should go through the direct JS execution path.
- **Streaming agent output** (LLM tokens, log tails) may exceed 8KB in aggregate but individual messages are small. Use channel-per-stream, not batch-and-send.
- **Binary data** (eBPF program bytes, screenshots) should always go through the fetch path. Do not base64-encode into JS strings.

---

## 2. Plugin Composition Model

**What Claude gets wrong:** Treating plugins as isolated units. Tauri's plugin model is a composition system where plugins share an application handle, hook into lifecycle events, and declare their own command namespaces.

### Plugin Trait Structure

A Tauri plugin implements a trait with lifecycle hooks:

- `initialize(app, config)` -- called once at app startup, receives the app handle
- `on_event(app, event)` -- receives runtime events (window created, navigation, etc.)
- `on_webview_ready(app, webview)` -- per-webview initialization
- `on_drop(app)` -- cleanup when the plugin is removed

The `PluginBuilder` provides a declarative API: `.invoke_handler()` for commands, `.setup()` for initialization, `.on_event()` for lifecycle hooks. Commands registered through a plugin are automatically namespaced as `plugin:<name>|<command>`. (tauri plugin.rs, insight #11)

### Namespace Isolation Without Process Isolation

Plugins run in the same process as the application. Isolation is at the namespace level, not the process level:

- **Command namespace**: `plugin:<name>|<command>` prevents shadowing
- **Permission namespace**: each plugin declares its own permission set and default scope
- **State namespace**: plugins can register typed state via `app.manage()`, keyed by TypeId
- **No memory isolation**: plugins can access any Rust state if they have a reference

This is a trust model decision. Tauri trusts plugin code (it is compiled into the binary) but isolates permissions (a plugin should not accidentally gain another plugin's capabilities).

### Application to geist.sh shell/

geist.sh's plugin model for the shell layer (x.uma extensions, HUD widgets) should follow the same namespace-isolation-without-process-isolation pattern:

- **Namespace commands**: `plugin:<name>|<capability>` for shell extensions, consistent with the agent namespace `agent:<id>|<capability>` from security-architecture.md
- **Typed state per plugin**: use `TypeId`-keyed state maps (like Tauri's `app.manage()`) rather than string-keyed registries. TypeId prevents key collisions by construction.
- **Lifecycle hooks, not inheritance**: plugins hook into shell events (agent connected, policy updated, webview ready) rather than subclassing a base type
- **Trust boundary is the build**: plugins compiled into the shell binary are trusted code. Dynamic plugin loading (if ever needed) would require a different trust model.

---

## 3. Event System

**What Claude gets wrong:** Treating events as a flat pub/sub bus. Tauri's event system has privilege stratification and cross-boundary semantics that prevent it from being a simple EventEmitter.

### Three Dispatch Modes

| Mode | Method | Semantics |
|------|--------|-----------|
| Broadcast | `emit(event, payload)` | All listeners across all windows/webviews |
| Targeted | `emit_to(target, event, payload)` | Specific window or webview by label |
| Filtered | `emit_filter(event, payload, filter_fn)` | Callback decides per-listener |

Broadcast is the common case. Targeted delivery avoids the overhead of N listeners evaluating an event they do not care about. Filtered delivery enables patterns like "emit to all windows of type X" without maintaining a separate registry. (tauri insight #19)

### Two Privilege Levels

| Privilege | Methods | Access |
|-----------|---------|--------|
| Scoped | `listen()`, `once()` | Only events targeting this component |
| Sniffer | `listen_any()`, `once_any()` | ALL events regardless of target |

Sniffing is only available through the `Manager` trait, which is the privileged application-level API. Individual windows and webviews get scoped access. This prevents a webview from observing events intended for other webviews. (tauri insight #19)

### Cross-Boundary Semantics

Events cross the Rust-JS boundary through the IPC layer (subject to size-based routing from Section 1). Critical invariant: JS-emitted events are NOT re-broadcast to other JS contexts. This prevents amplification attacks where a compromised webview floods sibling views.

### Application to geist.sh shell/

The HUD needs an event system for: policy updates from x.uma, agent status changes, eBPF enforcement notifications, user interactions from the webview.

- **Privilege stratification is essential**: The HUD webview receives events targeted to it. It should NOT sniff agent-to-shell IPC. Sniffer access is a debug/admin capability.
- **Filtered dispatch for agent groups**: `emit_filter` enables "notify all agents in workspace X" without per-workspace event lists.
- **Payload serialization budget**: Event payloads cross the IPC boundary. Keep them small (IDs, status enums, timestamps) rather than shipping full state. The HUD queries for full state on demand.

---

## 4. Webview Management

**What Claude gets wrong:** Assuming Window = Webview. Tauri v2 split these into independent entities, and the split is the key to multiwebview architectures.

### The Window/Webview Split (v2)

v1: `Window` contained a single `Webview`. 1:1 coupling.
v2: `Window` and `Webview` are separate entities with independent lifecycle.

The split was delivered in commit `c77b40324` (PR #8280) as an incremental refactoring:
1. Split `WindowDispatch` and `WebviewDispatch` traits
2. Separate `Window` and `Webview` managers
3. `WebviewWindow` convenience type for the common 1:1 case
4. `Window::add_child()` for multiwebview (multiple webviews in one window)

The 90/10 design: 90% of apps use `WebviewWindow` (the simple path). 10% use the split for multiwebview layouts. Neither pays for the other's complexity. (tauri insight #16)

### Semantic Correctness in the Split

The refactoring renamed `WindowUrl` to `WebviewUrl` and `tauri://window-created` to `tauri://webview-created`. URLs belong to webviews, not windows. A window is a native OS container; a webview is a content renderer. Confusing them leads to bugs where window-level operations target webview-level concerns. (tauri insight #16)

### Application to geist.sh shell/

The HUD may need multiple content areas (agent output, policy status, command palette) within a single window. If panels need independent navigation and permission scopes, use multiwebview (`Window::add_child()`). If not, use `WebviewWindow` and do not pay the multiwebview cost. The decision hinges on whether panels are independent trust domains -- if so, each gets its own `Webview` with independent event subscriptions and permissions.

---

## 5. Resource Lifecycle Across Process Boundary

**What Claude gets wrong:** Assuming Rust's Drop is sufficient for cross-boundary cleanup. When resources span the Rust-JavaScript boundary, Drop must become a protocol message.

### Drop as Protocol Obligation

When a Rust `Channel` is dropped, the `Drop` impl sends `{ end: true, index: N }` to the JavaScript side:

```
impl Drop for ChannelInner {
    fn drop(&mut self) {
        if let Some(on_drop) = &self.on_drop {
            on_drop();     // sends { end: true } to JS
        }
    }
}
```

Without this notification, JavaScript event listeners would leak -- waiting indefinitely for data from a channel that no longer exists. The Drop-as-notification pattern converts RAII into a cross-boundary protocol. (tauri insight #14)

### The Invariant

If JavaScript holds a reference (by ID, callback, or listener) to a Rust resource, the Rust `Drop` must notify JavaScript to release that reference. This applies to channels, webview handles, and plugin state alike. Without notification, the JS garbage collector cannot collect callback closures bound to the resource.

### Application to geist.sh shell/

The x.uma microgateway will create Rust-side resources with HUD-side counterparts:

- **Agent session handles**: When an agent disconnects (Rust drops the session), the HUD must remove the agent's panel/status indicator.
- **Policy watch channels**: When a policy subscription ends (Rust drops the watcher), the HUD must stop expecting policy updates.
- **Stream channels for LLM output**: When the LLM response completes or errors (Rust drops the stream), the HUD must finalize the output display.

For each of these, implement Drop-as-notification. Define a standard termination message format (equivalent to Tauri's `{ end: true }`) and document it as a protocol obligation, not an optimization.

---

## 6. State Management

**What Claude gets wrong:** Defaulting to global statics for application state. Tauri's "Less statics" cleanup (PR #14668) is evidence that this pattern accumulates technical debt.

### The Static State Anti-Pattern

Tauri's CLI tool accumulated global `static Mutex<Config>` state as the codebase grew. PR #14668 ("Less statics") systematically replaced these with function parameters threaded through the call graph. The motivation:

- **Testing friction**: Static state persists across tests, causing ordering dependencies
- **Thread-safety complexity**: Every static Mutex is a potential deadlock site
- **Hidden coupling**: Functions that read statics have invisible dependencies

The fix is unglamorous: add a parameter to every function that needs the config. The call graph becomes explicit. (tauri insight #20)

### TypeId-Keyed State (Managed State)

For application-level shared state, Tauri uses `app.manage::<T>(value)` which stores state keyed by `TypeId`. This avoids string key collisions and gives compile-time type safety at access points.

The trade-off: you can only have one value per type. If you need two instances of the same type, wrap in a newtype. This is deliberate -- it forces the developer to distinguish semantically different state by type, not by name.

### Application to geist.sh shell/

- **Thread state through function parameters**, not statics. The x.uma gateway config, policy engine handle, and HUD event sender should be explicit parameters (or fields on a context struct), not global statics.
- **Use TypeId-keyed state** for plugin-managed state in the shell. Each plugin registers its state type; access is type-safe and collision-free.
- **AppHandle as context carrier**: Tauri's `AppHandle` is the threaded context that replaces statics. geist.sh's equivalent is a `ShellContext` (or similar) passed to all plugin hooks and command handlers. This is the single source of truth for shell state.

---

## 7. Main-Thread Obligation

**What Claude gets wrong:** Treating main-thread issues as one-off bugs. This is a persistent architectural force in any desktop application using native UI toolkit APIs.

### The 5-Year Pattern

Tauri has 15+ commits across 5 years fixing main-thread issues. This is not a bug category -- it is an architectural force:

- 2022: `#2668` allow window ops on main thread, `#2711` expose `run_on_main_thread`
- 2023: `#3891` create webview on main thread, `#4298` block main thread for window creation
- 2024: `#8582`, `#8999`, `#11401`, `#11583` -- wave of "ensure X runs on main thread"
- 2025: `#13422`, `#13443` -- still fixing main-thread issues

Every new platform API that touches the view layer eventually needs main-thread pinning. The pattern is architectural, not incidental. (tauri insight #7)

### UnsafeSend for Drop

When a webview or window must be dropped on the main thread (because the platform toolkit requires it), Tauri uses:

```
let inner = UnsafeSend(inner);
let _ = self.app_handle.run_on_main_thread(move || {
    drop(inner.take());
});
```

`UnsafeSend` wraps a non-Send type with a safety comment that the value was created on and will be dropped on the main thread. This is necessary because Rust's type system cannot express "this value is Send only to the main thread."

### Application to geist.sh shell/

- **Provide `run_on_main_thread` as a first-class primitive** from day one. Do not wait for the first main-thread bug to add it.
- **All webview creation and destruction must go through the main thread**. This includes HUD panel creation, webview navigation, and window decoration changes.
- **Async command handlers run on the tokio runtime, NOT the main thread**. The x.uma policy engine, agent IPC, and eBPF interactions are async and off-main-thread. Only the final "update the UI" step dispatches to main thread.
- **Budget for ongoing main-thread fixes**. This is not a one-time cost. Every new HUD feature that touches native APIs will potentially need main-thread adjustment.

---

## 8. Runtime Abstraction and Platform Escape

**What Claude gets wrong:** Building cross-platform abstractions as lowest-common-denominator. Tauri's Runtime trait provides a common API AND platform-specific escape hatches.

### The Runtime Trait

The `Runtime<T: UserEvent>` trait abstracts over the webview backend via associated types: `WindowDispatcher`, `WebviewDispatcher`, `Handle`, `EventLoopProxy`. It exists for replaceability, not purity. Today's implementation is WRY (native webviews). The `feat/cef` branch validates the abstraction: a Chromium Embedded Framework backend implements the same trait, proving the abstraction pays for itself. (tauri insight #18, `tauri-runtime/src/lib.rs`)

### Platform Escape Hatches

The trait includes `cfg()`-gated platform-specific methods (macOS: `set_activation_policy`, Android: `find_class`, Linux: `gtk_window`) plus a universal escape: `with_webview(Box<dyn Any>)` for type-erased platform access. The common API covers 80%; the escapes cover the remaining 20%. (tauri insight #17)

### CEF: Bundle Size vs Consistency

WRY (native webviews, < 5MB) trades rendering consistency for bundle size. CEF (Chromium, ~100MB) trades bundle size for consistency. The Runtime trait makes this a swap, not a rewrite. (tauri insight #18)

### Application to geist.sh shell/

- **Do not build a custom runtime abstraction.** Tauri's Runtime trait already provides the abstraction. geist.sh gets CEF swappability for free if needed later.
- **DO use `cfg(target_os = "macos")`** directly for macOS-specific HUD behavior (dock icon, activation policy). Do not abstract prematurely.
- **Use `with_webview` sparingly** for direct webview access (custom protocol handlers, JS context injection). Document every usage with rationale for why the standard API was insufficient.

---

## Source Traceability Index

| Section | Tauri Insight | Key File/Commit |
|---------|---------------|-----------------|
| IPC size routing | #13 | `ipc/channel.rs` |
| Channel lifecycle | #14 | `ipc/channel.rs`, Drop impl |
| Plugin composition | #11 | `plugin.rs`, `ipc/authority.rs` |
| Event privilege | #19 | Event system spec |
| Window/Webview split | #16 | PR #8280, commit `c77b40324` |
| Drop-as-notification | #14 | `ipc/channel.rs` |
| Less statics | #20 | PR #14668, commit `7f7d9aac2` |
| Main-thread force | #7 | PRs #2668, #2711, #3891, #4298, #8582, #13422 |
| Runtime trait | Core abstractions | `tauri-runtime/src/lib.rs` |
| Platform escapes | #17 | `tauri-runtime/src/lib.rs`, cfg-gated methods |
| CEF backend | #18 | `feat/cef` branch |
