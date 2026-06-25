# Window Architecture Follow-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the confirmed architecture issues in `surgeist-window` by making native event delivery honest, moving `winit` lowering behind adapter-owned boundaries, clarifying command normalization, and tightening `Proxy` front-door docs.

**Architecture:** Treat `EventKind` as app-facing only if the native runner can deliver it through `Handler`; otherwise tests and fake-host behavior will keep drifting from production. Keep authored/window-domain types backend-neutral, move `winit` conversions into an adapter mapping module, and make command planning either produce backend-ready normalized operations or stop duplicating `Command`.

**Tech Stack:** Rust 2024, `surgeist-window`, `winit` 0.30, optional `accessibility` feature, crate-local unit tests in `src/tests.rs`, no new dependencies.

---

## Coordinator Scope

This is a crate-local architecture plan for:

```text
/Users/codex/Development/surgeist-window
```

Do not edit sibling crates or the root `surgeist` repo while implementing this plan. If a root API report or integration change is needed, stop and report that to the top-level coordinator after crate-local commits are pushed.

Follow:

```text
/Users/codex/Development/surgeist-window/AGENTS.md
/Users/codex/Development/surgeist-window/guidance/surgeist-rust-modeling-guide.md
```

`AGENTS.md` overrides Superpowers where they conflict. In this repo, assign one implementation worker at a time on `main`, then assign a separate reviewer before committing each logical task.

## Confirmed Issues

1. **Native EventKind delivery mismatch:** `WinitRunner::deliver_event` currently only checks ids, while the fake host records the full `EventKind` stream. This makes tests imply app-facing behavior that the native runner does not honor.
2. **Backend mapping leaks into domain modules:** `WindowRequest::to_winit_attributes` and several `winit::window` conversion impls live in `src/descriptor.rs`, and `DrawScheduler::control_flow` returns `winit::event_loop::ControlFlow` from `src/scheduler.rs`.
3. **Command normalization is under-modeled:** `Command` and `HostCommand` are currently near-identical enums. Planning does useful validation, but the normalized type is not yet backend-ready enough to justify the duplicate surface area.
4. **Proxy front door is split across files:** `Proxy` has useful public methods in `src/dsl.rs`, but its type docs in `src/registry.rs` only describe a narrow wakeup handle. The API mostly exists; discoverability and documentation need to catch up.

## Non-Goals

- Do not add new window features beyond the architectural corrections below.
- Do not introduce new dependencies.
- Do not add broad lint-suppression attributes to quiet warnings.
- Do not change root workspace wiring or submodule pointers from this crate project.
- Do not remove fake-host event recording; align it with production instead.

## File Structure

- Modify `/Users/codex/Development/surgeist-window/src/handler.rs`: add a generic event callback only if `EventKind` remains public app-facing vocabulary.
- Modify `/Users/codex/Development/surgeist-window/src/dsl.rs`: add an `Event<'a>` scope type parallel to `Ready`, `Resize`, and `Input`; update `Proxy` docs/method placement only as needed.
- Modify `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`: route non-specialized events through the new handler callback; consume adapter-owned `winit` mapping helpers; keep native runner as the backend application boundary.
- Modify `/Users/codex/Development/surgeist-window/src/testing.rs`: add fake-host native-transition dispatch support for the same generic event callback and state-patch semantics.
- Modify `/Users/codex/Development/surgeist-window/src/descriptor.rs`: remove direct `winit` lowering from `WindowRequest`.
- Modify `/Users/codex/Development/surgeist-window/src/scheduler.rs`: make draw scheduling return backend-neutral deadlines and pending draw intent.
- Modify `/Users/codex/Development/surgeist-window/src/loop_.rs`: import initial native event-loop control-flow setup from the adapter mapping boundary.
- Create `/Users/codex/Development/surgeist-window/src/winit_mapping.rs`: centralize `winit` lowering for window requests, draw control flow, fullscreen, controls, levels, themes, cursors, IME, metrics, and feature-gated accessibility mapping when practical.
- Modify `/Users/codex/Development/surgeist-window/src/transition.rs`: collapse duplicated `HostCommand` variants into a validated `HostCommandPlan` around the original `Command`.
- Modify `/Users/codex/Development/surgeist-window/src/lib.rs`: update public exports for new front-door types and any mapping module privacy.
- Modify `/Users/codex/Development/surgeist-window/src/tests.rs`: add regression tests for native/fake event delivery alignment, backend-neutral domain modules, command planning shape, and proxy discoverability.
- Modify `/Users/codex/Development/surgeist-window/README.md`: document the corrected handler event surface, backend mapping boundary, command planning semantics, and proxy use from `Context::proxy()`.

## Verification Commands

Run after each task:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Run before final review and commit:

```sh
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
cargo fmt --check
```

If public exports change intentionally and the crate has source-derived API tooling available in this checkout, refresh and review the API artifact in the same commit as the source change.

## Review Gates

Each task must receive a separate reviewer pass before commit. Reviewers must check:

- fake and native host behavior are aligned, or any intentional difference is explicit in docs and tests
- `winit` names do not remain in `descriptor.rs` or scheduler APIs after the mapping task
- command planning has one clear semantic: validated app commands with capability diagnostics, not a duplicate host enum
- public API additions are front-door exports from `src/lib.rs` and have docs/examples where appropriate
- feature-gated accessibility behavior still compiles and is tested
- no sibling crates or root repo files were changed

---

## Task 0: Preflight And Ownership Check

**Files:**
- Inspect: `/Users/codex/Development/surgeist-window/AGENTS.md`
- Inspect: `/Users/codex/Development/surgeist-window/guidance/surgeist-rust-modeling-guide.md`
- Inspect: `/Users/codex/Development/surgeist-window/src/handler.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/transition.rs`

- [ ] **Step 1: Check repository status**

Run:

```sh
git status --short --branch
```

Expected: `## main...origin/main` with no local changes, unless the coordinator has intentionally staged only this plan or a prior task's committed work.

- [ ] **Step 2: Confirm the work belongs in `surgeist-window`**

Confirm all changes are in the window crate: handler lifecycle, event vocabulary, native adapter mapping, fake host alignment, command planning, and docs. If implementation requires sibling crate changes, stop and report a cross-crate issue instead of editing outside this repo.

- [ ] **Step 3: Assign one implementation worker**

Tell the worker:

```text
You are working in /Users/codex/Development/surgeist-window on main.
You are not alone in the codebase; do not revert others' work.
Stay inside this crate repo.
Follow AGENTS.md and guidance/surgeist-rust-modeling-guide.md.
Implement only the assigned task and report tests plus git status.
Do not commit or push.
```

- [ ] **Step 4: Assign a separate reviewer after implementation**

The reviewer should inspect the worker's diff against this plan and the modeling guide, run the task's focused tests, and report findings before the coordinator commits.

---

## Task 1: Make Native Event Delivery Real Or Explicitly Non-App-Facing

**Decision:** Keep `EventKind` app-facing and add real native delivery. This matches the current public export from `src/lib.rs`, the fake host event stream, and the crate-level docs that say this crate owns native events.

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/handler.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/dsl.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Add failing tests for generic event delivery**

Add focused tests in `src/tests.rs` with these names:

```rust
#[test]
fn native_event_delivery_invokes_handler_event_callback_for_modeled_events()
```

This test should use a test-only `WinitRunner` helper, not a real native event loop. Add the helper in Step 4 as `WinitRunner::call_with_event_for_test(event)`. The test should seed a live window in the runner registry, call the helper with a modeled event such as `EventKind::Focused { id, focused: true }`, and assert that a handler callback observed that exact event. It should also assert that the callback can enqueue a command into `runner.commands` and return an action from the event scope.

```rust
#[test]
fn fake_host_native_transition_dispatch_matches_native_event_callback_contract()
```

This test should exercise the fake host's native-transition dispatch with a stateful transition such as `NativeEventTransition::focused(id, true)`. It must assert that the same `EventKind` reaches the handler, remains recorded in the fake event stream, and the handler observes the patched state through `event.state()`.

```rust
#[test]
fn specialized_resize_input_close_callbacks_do_not_double_deliver_generic_event()
```

This test should assert that `Resized`, `Input`, `CloseRequested`, and `Destroyed` keep using their existing specialized callbacks unless the implementation deliberately documents and tests a different ordering. The default expected behavior is no duplicate generic callback for these specialized paths.

Run:

```sh
cargo test -p surgeist-window native_event_delivery_invokes_handler_event_callback_for_modeled_events -- --nocapture
cargo test -p surgeist-window fake_host_native_transition_dispatch_matches_native_event_callback_contract -- --nocapture
cargo test -p surgeist-window specialized_resize_input_close_callbacks_do_not_double_deliver_generic_event -- --nocapture
```

Expected before implementation: the crate may fail to compile because `Handler::event`, the `Event` scope, and `WinitRunner::call_with_event_for_test` do not exist yet. After those APIs are added, the helper test proves the native runner can build and invoke the generic event scope without `ActiveEventLoop`. Because the real `deliver_event` method needs `ActiveEventLoop`, Step 4 and the reviewer gate must inspect that `deliver_event` itself calls `call_with_event` and finishes the callback instead of returning to a no-op.

- [ ] **Step 2: Add an event scope type**

In `src/dsl.rs`, add `pub struct Event<'a>` near `Ready`, `Resize`, and `Input`.

The scope must expose:

```rust
impl<'a> Event<'a> {
    pub(crate) fn new(event: EventKind, context: Context<'a>) -> Self;
    pub fn id(&self) -> Id;
    pub fn event(&self) -> &EventKind;
    pub fn state(&self) -> Option<&WindowSnapshot>;
    pub fn context_mut(&mut self) -> &mut Context<'a>;
    pub fn window(&mut self) -> Option<Target<'_>>;
    pub fn draw(&mut self) -> &mut Self;
    pub fn again(&mut self) -> &mut Self;
    pub fn at(&mut self, time: Instant) -> &mut Self;
    pub fn close(&mut self) -> &mut Self;
    pub fn exit(&mut self) -> &mut Self;
}
```

`state()` and `window()` should return `Option` because `Destroyed` carries a window id whose live state may already be gone in some future routing. For the current task, generic event delivery should avoid `Destroyed` unless later tasks intentionally change closed delivery.

- [ ] **Step 3: Add a default handler callback**

In `src/handler.rs`, update imports and add:

```rust
fn event(&mut self, _event: &mut Event<'_>) -> Result<()> {
    Ok(())
}
```

The method must have a default implementation so existing handlers continue compiling.

- [ ] **Step 4: Route native generic events to the handler**

In `src/winit_adapter.rs`, add a `call_with_event` path mirroring `call_with_input`:

- create fresh command/action buffers
- build `Context` with `self.proxy.clone()`
- wrap the `EventKind` in the new `Event` scope
- call `self.handler.event(&mut event)`
- extend queued commands

Then replace the no-op `deliver_event` with:

- `debug_assert_eq!(event.id(), id)`
- `let action = self.call_with_event(event)`
- `self.finish_callback(event_loop, action)`

Add a test-only helper that does not require `ActiveEventLoop`:

```rust
#[cfg(test)]
pub(crate) fn call_with_event_for_test(&mut self, event: EventKind) -> Result<Action> {
    self.call_with_event(event)
}
```

The helper should not apply native commands; it exists to prove the native runner can build the same handler event scope without a real platform event loop.

Do not route `Input` through `deliver_event`; it already routes through `deliver_input`. Do not route `Resized`, `ScaleFactorChanged`, `CloseRequested`, or `Destroyed` through generic event delivery in this task unless the worker updates the plan first and gets reviewer approval.

- [ ] **Step 5: Align the fake host**

In `src/testing.rs`, add a fake-host native-transition dispatch method:

```rust
pub fn dispatch_native_transition<H: Handler>(
    &mut self,
    handler: &mut H,
    transition: NativeEventTransition,
) -> Result<Effect>
```

It must:

- require a live window for events that need live state before applying the patch
- apply `transition.patch()` to the fake registry before callback delivery, using the same `WindowStatePatch` semantics as native
- convert `transition.into_event()` into the delivered `EventKind`
- record `Event::from(event.clone())`
- build the same `Event<'_>` scope used by native
- apply commands emitted by the callback through `HostCommandPlan`
- return the resolved `Effect`

For event-only records that do not have a `NativeEventTransition` today, such as `Resumed`, `Suspended`, `FileDrag`, and `Accessibility`, keep existing fake-host methods or add a separate `dispatch_event` helper that is explicitly documented as event-only. Do not use event-only dispatch to claim stateful fake/native transition alignment.

- [ ] **Step 6: Export the event scope**

In `src/lib.rs`, export the new scope from the DSL front door:

```rust
pub use dsl::{
    App, Close, Closed, ControlsBuilder, Event, Frame, Input, Open, Ready, Resize, Scope, Selector,
    Target, app, controls, open, point, rect, size,
};
```

- [ ] **Step 7: Verify**

Run:

```sh
cargo test -p surgeist-window native_event_delivery_invokes_handler_event_callback_for_modeled_events -- --nocapture
cargo test -p surgeist-window fake_host_native_transition_dispatch_matches_native_event_callback_contract -- --nocapture
cargo test -p surgeist-window specialized_resize_input_close_callbacks_do_not_double_deliver_generic_event -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 8: Reviewer gate and commit**

Reviewer must confirm the native runner no longer drops generic `EventKind` silently and the fake host exercises the same callback contract.

Commit message:

```sh
git add src/handler.rs src/dsl.rs src/winit_adapter.rs src/testing.rs src/lib.rs src/tests.rs
git commit -m "feat: deliver modeled window events to handlers"
```

---

## Task 2: Move Winit Lowering Behind Adapter-Owned Mapping

**Files:**
- Create: `/Users/codex/Development/surgeist-window/src/winit_mapping.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/scheduler.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/loop_.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Add failing boundary tests**

Add or rename tests in `src/tests.rs` to make the expected ownership clear:

```rust
#[test]
fn winit_mapping_converts_window_request_to_native_attributes()
```

This replaces direct calls to `WindowRequest::to_winit_attributes()` and should call an adapter-owned helper such as `winit_mapping::window_attributes_from_request(&request)`. Existing tests that cover controls, level, theme, fullscreen, size, and position lowering should move to this helper too.

```rust
#[test]
fn draw_scheduler_exposes_backend_neutral_deadline()
```

This should assert scheduler behavior through `next_deadline()` and ready ids, not through `winit::event_loop::ControlFlow`.

Run:

```sh
cargo test -p surgeist-window winit_mapping_converts_window_request_to_native_attributes -- --nocapture
cargo test -p surgeist-window draw_scheduler_exposes_backend_neutral_deadline -- --nocapture
```

Expected before implementation: mapping helper does not exist or old tests still rely on direct descriptor/scheduler `winit` APIs.

- [ ] **Step 2: Create `src/winit_mapping.rs`**

Move `WindowRequest::to_winit_attributes` logic into a module-private or crate-private function:

```rust
pub(crate) fn window_attributes_from_request(
    request: &WindowRequest,
) -> Result<winit::window::WindowAttributes>
```

Move these descriptor-owned conversion impls into `src/winit_mapping.rs` too:

```rust
impl From<Controls> for winit::window::WindowButtons
impl From<Level> for winit::window::WindowLevel
impl From<Theme> for winit::window::Theme
```

Also move native scheduler lowering into:

```rust
pub(crate) fn control_flow_from_draw_scheduler(draw: &DrawScheduler) -> winit::event_loop::ControlFlow
pub(crate) fn native_control_flow(control_flow: winit::event_loop::ControlFlow) -> winit::event_loop::ControlFlow
```

If the worker finds `native_control_flow` should stay separate for macOS behavior, keep it in `winit_mapping.rs` and preserve existing tests.

- [ ] **Step 3: Remove `winit` lowering from descriptor**

Delete `WindowRequest::to_winit_attributes` and all `winit::window` conversion impls from `src/descriptor.rs`. `descriptor.rs` should keep stable request/snapshot/role/fullscreen data and must not reference `winit::` at all.

- [ ] **Step 4: Remove `winit` lowering from scheduler**

Delete `DrawScheduler::control_flow()` from `src/scheduler.rs`. Keep `next_deadline()`, `take_ready()`, and scheduling state backend-neutral.

- [ ] **Step 5: Update native adapter call sites**

In `src/winit_adapter.rs`:

- replace `native_request.to_winit_attributes()?` with `winit_mapping::window_attributes_from_request(&native_request)?`
- replace `native_control_flow(self.draw.control_flow())` with `winit_mapping::control_flow_from_draw_scheduler(&self.draw)`
- preserve pending-draw retry behavior in `WinitRunner::control_flow`

In `src/loop_.rs`, import `native_control_flow` from the new mapping boundary if still needed for initial event-loop setup.

- [ ] **Step 6: Keep the mapping module private**

In `src/lib.rs`, add:

```rust
mod winit_mapping;
```

Do not publicly export this module. Tests may use `super::winit_mapping` from crate-local unit tests.

- [ ] **Step 7: Verify**

Run:

```sh
if rg -n "winit::|to_winit_attributes" src/descriptor.rs src/scheduler.rs; then exit 1; fi
rg -n "winit::event_loop::ControlFlow|control_flow_from_draw_scheduler|native_control_flow" src/winit_adapter.rs src/winit_mapping.rs src/loop_.rs src/tests.rs
cargo test -p surgeist-window winit_mapping_converts_window_request_to_native_attributes -- --nocapture
cargo test -p surgeist-window draw_scheduler_exposes_backend_neutral_deadline -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: no `winit::` references remain in `src/descriptor.rs` or `src/scheduler.rs`; all checks pass.

- [ ] **Step 8: Reviewer gate and commit**

Reviewer must confirm backend mapping is now adapter-owned and domain modules are backend-neutral.

Commit message:

```sh
git add src/winit_mapping.rs src/descriptor.rs src/scheduler.rs src/winit_adapter.rs src/loop_.rs src/lib.rs src/tests.rs
git commit -m "refactor: move winit lowering into adapter mapping"
```

---

## Task 3: Collapse Duplicate HostCommand Surface

**Decision:** Remove the duplicate `HostCommand` enum for now. Keep `HostCommandPlan` as the capability-checked planning result around the original `Command`, because the current crate does not yet produce backend-resolved host operations distinct from app-authored intent.

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/transition.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Add failing planning-shape tests**

Add tests in `src/tests.rs`:

```rust
#[test]
fn host_command_plan_returns_validated_command_without_duplicate_host_enum()
```

Expected behavior: planning a supported `Command` returns `HostCommandPlan`, and `plan.command()` returns the original validated `Command` by reference. `plan.into_command()` returns the original `Command` by value.

```rust
#[test]
fn host_command_plan_keeps_backend_capability_rejections_before_application()
```

Expected behavior: unsupported roles, fullscreen modes, and cursor capabilities are still rejected by planning before fake/native application.

```rust
#[test]
fn host_command_enum_is_not_part_of_public_front_door()
```

Expected behavior: `src/lib.rs` no longer publicly exports `HostCommand`, and crate tests no longer import or pattern-match a duplicate host enum. This can be protected by removing the existing `std::mem::size_of::<HostCommand>()` front-door assertion and replacing it with assertions against `HostCommandPlan`.

- [ ] **Step 2: Remove the duplicate enum**

In `src/transition.rs`, delete the entire `pub enum HostCommand` definition.

Change `HostCommandPlan` to store the original validated command:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct HostCommandPlan {
    command: Command,
}
```

Keep these accessors:

```rust
pub fn command(&self) -> &Command
pub fn into_command(self) -> Command
```

- [ ] **Step 3: Keep capability checks in planning**

`HostCommandPlan::from_command` must still validate:

- open role support
- open fullscreen support
- set fullscreen support
- cursor capability support

Do not move live-registry checks into planning; unknown ids, duplicate names, native handle availability, and platform failures stay in fake/native application.

- [ ] **Step 4: Update fake and native application**

Update `testing::Host::apply_plan` and `WinitRunner::apply_host_command` to consume `Command` from `HostCommandPlan::into_command()`.

Both fake and native paths must consume the same `HostCommandPlan` output. Live-state checks still belong in fake/native application, not in planning.

- [ ] **Step 5: Update public exports and tests**

In `src/lib.rs`, remove `HostCommand` from the public reexports:

```rust
pub use transition::{HostCommandPlan, MetricsEvent, NativeEventTransition, WindowStatePatch};
```

Update tests that currently pattern-match `HostCommand` so they assert against `Command` returned by `HostCommandPlan`.

- [ ] **Step 6: Verify**

Run:

```sh
cargo test -p surgeist-window host_command_plan_returns_validated_command_without_duplicate_host_enum -- --nocapture
cargo test -p surgeist-window host_command_plan_keeps_backend_capability_rejections_before_application -- --nocapture
cargo test -p surgeist-window host_command_enum_is_not_part_of_public_front_door -- --nocapture
if rg -n "pub enum HostCommand|HostCommand::|size_of::<HostCommand>|\\bHostCommand\\b" src/lib.rs src/tests.rs src/testing.rs src/winit_adapter.rs; then exit 1; fi
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 7: Reviewer gate and commit**

Reviewer must answer this question explicitly:

```text
Does HostCommandPlan now have one clear semantic: capability-checked Command validation without a duplicate host enum?
```

Commit message:

```sh
git add src/transition.rs src/lib.rs src/winit_adapter.rs src/testing.rs src/tests.rs
git commit -m "refactor: clarify host command planning semantics"
```

---

## Task 4: Tighten Proxy Front-Door Documentation

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/registry.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/dsl.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`
- Modify: `/Users/codex/Development/surgeist-window/README.md`

- [ ] **Step 1: Add API/discoverability tests**

Add a test in `src/tests.rs`:

```rust
#[test]
fn proxy_public_front_door_exposes_typed_cross_thread_commands()
```

The test should compile against the public methods already available on `Proxy`: `send`, `open`, `close`, `draw`, `again`, `at`, and `exit`. Use a local helper function that accepts `&Proxy` and references these methods in a non-executed closure or type-checked helper so no real event loop is required.

- [ ] **Step 2: Update `Proxy` docs where the type is defined**

In `src/registry.rs`, replace the narrow doc:

```rust
/// Cross-thread event-loop wakeup handle.
```

with docs that state:

```rust
/// Cross-thread handle for sending typed window commands and event-loop actions.
///
/// Obtain a proxy from `Context::proxy()` inside a handler callback, clone it,
/// and move it to another thread when external work needs to wake the window
/// loop. Public command helpers are implemented on `Proxy` by the app-facing
/// DSL module: `send`, `open`, `close`, `draw`, `again`, `at`, and `exit`.
```

- [ ] **Step 3: Add method-level docs for each public proxy helper**

In `src/dsl.rs`, add short docs to:

- `Proxy::send`
- `Proxy::open`
- `Proxy::close`
- `Proxy::draw`
- `Proxy::again`
- `Proxy::at`

In `src/registry.rs`, add method docs to `Proxy::exit`, where that method is defined.

Docs should say these helpers enqueue work onto the native loop and may fail with `ErrorCode::CommandFailed` if the event loop is closed.

- [ ] **Step 4: Document `Context::proxy()` in README**

Add a short README section showing:

```rust
if let Some(proxy) = cx.proxy() {
    std::thread::spawn(move || {
        let _ = proxy.draw(window_id);
    });
}
```

Explain that the proxy is the safe cross-thread front door for typed window commands and draw/exit actions.

- [ ] **Step 5: Verify**

Run:

```sh
cargo test -p surgeist-window proxy_public_front_door_exposes_typed_cross_thread_commands -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 6: Reviewer gate and commit**

Reviewer must confirm this is a docs/discoverability cleanup, not a new behavior surface.

Commit message:

```sh
git add src/registry.rs src/dsl.rs src/tests.rs README.md
git commit -m "docs: clarify proxy command front door"
```

---

## Task 5: Final Feature Checks And Plan Completion

**Files:**
- Inspect all changed files.
- Optional generated artifact: `/Users/codex/Development/surgeist-window/api/public-api.txt`, only if this repo's API generator exists and public API refresh is required.

- [ ] **Step 1: Run full default checks**

Run:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 2: Run accessibility feature checks**

Run:

```sh
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
```

Expected: all pass.

- [ ] **Step 3: Check architectural boundaries**

Run:

```sh
if rg -n "winit::|to_winit_attributes" src/descriptor.rs src/scheduler.rs; then exit 1; fi
rg -n "fn deliver_event" src/winit_adapter.rs
if rg -n "pub enum HostCommand|HostCommand::|size_of::<HostCommand>|\\bHostCommand\\b" src/lib.rs src/transition.rs src/testing.rs src/winit_adapter.rs src/tests.rs; then exit 1; fi
rg -n "pub enum Command|pub struct HostCommandPlan" src/command.rs src/transition.rs
git diff --stat
git diff --check
```

Expected:

- no direct `winit` lowering remains in `descriptor.rs` or `scheduler.rs`
- `deliver_event` calls the handler path or has been intentionally removed from app-facing semantics
- `HostCommandPlan` is the only command-planning surface and returns validated `Command`
- no whitespace errors

- [ ] **Step 4: Final reviewer cycle**

Assign a clean reviewer who did not implement the tasks. Ask them to review the complete diff against the four confirmed issues and state whether reviewer findings are clean.

- [ ] **Step 5: Final commit or push**

If all logical task commits already exist, do not squash them. If final docs/API artifact updates remain, commit them separately with a concrete message.

Push `main` only when requested by the top-level coordinator or when this crate commit must be fetched for submodule pointer updates.

---

## Completion Criteria

This plan is complete only when:

- native generic `EventKind` delivery is no longer a no-op, or app-facing docs/tests no longer claim generic native event delivery
- fake and real paths share the same event callback contract for modeled events
- `descriptor.rs` and `scheduler.rs` no longer expose `winit` lowering APIs
- command planning has one clear documented semantic and tests protecting that semantic
- `Proxy` docs match its public typed command/action methods
- default and accessibility feature checks pass
- all reviewer cycles come back clean
