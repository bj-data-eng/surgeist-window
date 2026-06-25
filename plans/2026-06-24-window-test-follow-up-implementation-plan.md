# Window Test Follow-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Revise the `surgeist-window` test-review follow-up against the post-refactor model so only window-owned coverage work is handed back to that crate.

**Architecture:** The window crate now models app-authored requests, runtime snapshots, host capabilities, host command plans, state patches, and native event transitions explicitly. Test follow-up should verify those model boundaries and fake/native alignment rather than adding broad test-only event sinks around the old runner shape.

**Tech Stack:** Rust 2024, `surgeist-window`, `winit` 0.30, optional `accessibility` feature, crate-local unit tests in `src/tests.rs`, no new dependencies.

---

## Coordinator Scope

This plan is owned by `surgeist-test` as test-coordination work. The work below is meant for `surgeist-window` only. Do not include root workspace, `surgeist-test`, e2e, browser, or cross-crate integration work in the handoff.

Execution location after handoff:

```text
/Users/codex/Development/surgeist-window
```

Relevant window implementation plan:

```text
/Users/codex/Development/surgeist-window/plans/2026-06-24-surgeist-window-modeling-refactor.md
```

The earlier review findings have changed shape:

- Covered by the modeling refactor: `WindowRequest`, `WindowSnapshot`, `HostCapabilities`, `HostCommandPlan`, `WindowStatePatch`, `NativeEventTransition`, shared fake/native command planning, and many lifecycle DSL tests.
- Still needs cleanup: `dsl_public_vocabulary_uses_lifecycle_draw_names` remains low-value compile noise.
- Still needs sharper coverage: fake-host failure side effects, native transition event/patch pairs beyond focus and pointer movement, and feature-specific accessibility mapping.

---

## Files To Touch In Window

- Modify `/Users/codex/Development/surgeist-window/src/tests.rs`: add focused tests and replace the frivolous DSL test.
- Modify `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`: only if extracting feature-gated accessibility event mapping.
- Do not modify sibling crates, root `surgeist`, or `surgeist-test` while implementing this window plan.

---

## Task 0: Window-Lane Preflight

**Files:**
- Inspect: `/Users/codex/Development/surgeist-window/AGENTS.md`
- Inspect: `/Users/codex/Development/surgeist-window/plans/2026-06-24-surgeist-window-modeling-refactor.md`
- Inspect: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Check window repository status**

Run:

```sh
git status --short --branch
```

Expected: no unrelated local edits. If unrelated edits exist, report them and coordinate before assigning implementation. Do not revert user work.

- [ ] **Step 2: Confirm this plan is window-only**

Confirm the implementation only changes `surgeist-window` focused tests and, if needed, a small `winit_adapter.rs` extraction for accessibility mapping. Exclude root, `surgeist-test`, and e2e work.

- [ ] **Step 3: Assign one implementation worker**

Assign one worker to the `surgeist-window` lane. Tell the worker they are not alone in the codebase, must not revert others' work, and must keep edits inside `/Users/codex/Development/surgeist-window`.

- [ ] **Step 4: Require reviewer cycle**

After implementation, assign a separate reviewer to inspect the changed tests against the current window model. Do not declare the plan complete until reviewer findings are resolved and checks pass.

---

## Task 1: Replace Frivolous DSL Vocabulary Test

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Remove the low-value test**

Delete the whole test named:

```rust
fn dsl_public_vocabulary_uses_lifecycle_draw_names()
```

This test mostly asserts that values equal themselves and does not protect a meaningful behavior boundary.

- [ ] **Step 2: Add a behavior-focused DSL/context test**

Add this test near the existing DSL context tests:

```rust
#[test]
fn dsl_context_resolves_named_window_state_then_targets_commands_and_actions() {
    let mut window_loop = Loop::new(NoopHandler);
    let id = window_loop.registry.reserve_id();
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 480,
            height: 240,
        },
        2.0,
    );
    window_loop.registry.insert(Instance::new(
        id,
        WindowSnapshot::new(id, "Main", metrics).named("main").with_visible(true),
    ));

    {
        let mut cx = window_loop.context();
        let target = cx.window_id("main").expect("named window should resolve");

        assert_eq!(cx.state("main").map(WindowSnapshot::id), Some(id));

        cx.window(target)
            .title("Ready Main")
            .size(size(320, 160))
            .draw()
            .close();
        cx.again(target);
    }

    assert_eq!(
        window_loop.commands,
        vec![
            Command::SetTitle {
                id,
                title: String::from("Ready Main"),
            },
            Command::SetInnerSize {
                id,
                size: Size {
                    width: 320.0,
                    height: 160.0,
                },
            },
        ]
    );
    assert_eq!(
        window_loop.context().action(),
        &Action::Batch(vec![
            Action::DrawNext(id),
            Action::CloseRequested(id),
            Action::DrawNow(id),
        ])
    );
}
```

- [ ] **Step 3: Run the replacement test**

Run:

```sh
cargo test -p surgeist-window dsl_context_resolves_named_window_state_then_targets_commands_and_actions
```

Expected: the new test passes. If it fails because the public DSL surface changed again, update the test to assert the same behavioral contract with the current APIs rather than reintroducing compile-only assertions.

---

## Task 2: Lock Down Fake-Host Failure Side Effects

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Add direct-command failure tests**

Add these tests near the existing fake-host tests:

```rust
#[test]
fn fake_host_records_failed_direct_command_without_state_or_event_side_effects() {
    let mut host = testing::Host::new();
    let id = Id::from_u64(404);

    let error = host
        .apply(Command::RequestDraw { id })
        .expect_err("unknown draw target should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert_eq!(error.id, Some(id));
    assert_eq!(host.commands(), &[Command::RequestDraw { id }]);
    assert!(host.events().is_empty());
    assert!(host.take_ready_draws(Instant::now()).is_empty());
}

#[test]
fn fake_host_duplicate_open_records_attempt_without_creating_second_window() {
    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    host.clear();

    let error = host
        .apply(open("main"))
        .expect_err("duplicate name should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert_eq!(host.commands().len(), 1);
    assert!(matches!(
        &host.commands()[0],
        Command::Open { request } if request.name() == Some("main")
    ));
    assert!(host.events().is_empty());
    assert_eq!(host.registry().len(), 1);
}
```

- [ ] **Step 2: Add callback rollback test**

Add this test near the other lifecycle dispatch tests:

```rust
#[test]
fn fake_host_callback_failure_rolls_back_callback_commands_and_command_induced_state() {
    struct FailingReady;

    impl Handler for FailingReady {
        fn ready(&mut self, ready: &mut Ready<'_>) -> Result<()> {
            ready.target().title("Should not apply");
            Err(Error::new(ErrorCode::CommandFailed, "intentional failure"))
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    let original_title = host
        .registry()
        .get(id)
        .unwrap()
        .instance
        .state()
        .title()
        .to_owned();
    host.clear();

    let error = host.dispatch_ready(&mut FailingReady, id).unwrap_err();

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert!(host.commands().is_empty());
    assert!(host.events().is_empty());
    assert_eq!(
        host.registry().get(id).unwrap().instance.state().title(),
        original_title
    );
}
```

- [ ] **Step 3: Run focused fake-host tests**

Run:

```sh
cargo test -p surgeist-window fake_host_records_failed_direct_command_without_state_or_event_side_effects
cargo test -p surgeist-window fake_host_duplicate_open_records_attempt_without_creating_second_window
cargo test -p surgeist-window fake_host_callback_failure_rolls_back_callback_commands_and_command_induced_state
```

Expected: all pass. If any fail, reconcile the intended contract with the `surgeist-window` coordinator before changing implementation. Preferred contract:

```text
Direct Host::apply records attempted commands, even if validation fails.
Callback dispatch records only successfully applied callback-generated commands.
Failed commands must not mutate command-induced registry state, events, draw scheduler, cursor updates, or IME requests.
```

---

## Task 3: Cover Native Transition Patch/Event Pairs

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Add transition tests for non-pointer events**

Add these tests near the existing `NativeEventTransition` tests:

```rust
#[test]
fn native_event_transition_pairs_moved_theme_and_occlusion_patches_with_events() {
    let id = Id::from_u64(1);

    let moved = NativeEventTransition::moved(id, Point { x: 10.0, y: 20.0 });
    assert_eq!(
        moved.patch(),
        Some(&WindowStatePatch::Position {
            id,
            position: Point { x: 10.0, y: 20.0 },
        })
    );
    assert_eq!(
        moved.event(),
        Some(&EventKind::Moved {
            id,
            position: Point { x: 10.0, y: 20.0 },
        })
    );

    let themed = NativeEventTransition::theme_changed(id, Some(Theme::Dark));
    assert_eq!(
        themed.patch(),
        Some(&WindowStatePatch::Theme {
            id,
            theme: Some(Theme::Dark),
        })
    );
    assert_eq!(
        themed.event(),
        Some(&EventKind::ThemeChanged {
            id,
            theme: Some(Theme::Dark),
        })
    );

    let occluded = NativeEventTransition::occluded(id, true);
    assert_eq!(
        occluded.patch(),
        Some(&WindowStatePatch::Occluded { id, occluded: true })
    );
    assert_eq!(
        occluded.event(),
        Some(&EventKind::Occluded { id, occluded: true })
    );
}

#[test]
fn native_transition_route_sends_lifecycle_events_to_event_delivery() {
    let id = Id::from_u64(1);
    let event = NativeEventTransition::focused(id, true)
        .into_event()
        .expect("focus transition should emit event");

    assert_eq!(native_transition_route(&event), NativeTransitionRoute::Event);
}
```

- [ ] **Step 2: Add metrics patch/event preservation test**

Add:

```rust
#[test]
fn metrics_state_patch_preserves_outer_geometry_and_reports_resize_event() {
    let id = Id::from_u64(3);
    let mut snapshot = WindowSnapshot::new(
        id,
        "Main",
        Metrics::from_physical_size(
            id,
            PhysicalSize {
                width: 200,
                height: 100,
            },
            1.0,
        ),
    );
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 600,
            height: 300,
        },
        2.0,
    )
    .with_outer_geometry(
        Some(Point { x: 10.0, y: 20.0 }),
        Some(Size {
            width: 340.0,
            height: 220.0,
        }),
    );

    let event = WindowStatePatch::metrics(metrics.clone(), MetricsEvent::Resized)
        .apply(&mut snapshot)
        .expect("metrics patch should apply")
        .expect("resize patch should emit an event");

    assert_eq!(snapshot.metrics(), &metrics);
    assert_eq!(event, EventKind::Resized(metrics));
}
```

- [ ] **Step 3: Run focused transition tests**

Run:

```sh
cargo test -p surgeist-window native_event_transition_pairs_moved_theme_and_occlusion_patches_with_events
cargo test -p surgeist-window native_transition_route_sends_lifecycle_events_to_event_delivery
cargo test -p surgeist-window metrics_state_patch_preserves_outer_geometry_and_reports_resize_event
```

Expected: all pass. These are unit tests for the window-owned transition model, not live OS event-loop tests.

---

## Task 4: Add Feature-Specific Accessibility Mapping Coverage

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`

- [ ] **Step 1: Add feature-gated test import**

Update the `winit_adapter` imports in `/Users/codex/Development/surgeist-window/src/tests.rs`:

```rust
use super::winit_adapter::{
    NativeTransitionRoute, PointerPositionKey, WinitRunner, code_from_winit, ime_event_from_winit,
    key_from_winit, location_from_winit, native_transition_route,
};
#[cfg(feature = "accessibility")]
use super::winit_adapter::accessibility_event_from_winit;
```

- [ ] **Step 2: Add feature-gated mapping test**

Add near the accessibility tests:

```rust
#[cfg(feature = "accessibility")]
#[test]
fn accessibility_feature_maps_accesskit_window_events() {
    let id = Id::from_u64(12);

    assert_eq!(
        accessibility_event_from_winit(id, accesskit_winit::WindowEvent::InitialTreeRequested),
        AccessibilityEvent::InitialTreeRequested(id)
    );
    assert_eq!(
        accessibility_event_from_winit(id, accesskit_winit::WindowEvent::AccessibilityDeactivated),
        AccessibilityEvent::Deactivated(id)
    );
}
```

This test intentionally covers only the constructible fieldless `WindowEvent`
variants. `ActionRequested` wraps `accesskit::ActionRequest`, and `accesskit` is
not a direct dependency of this crate. The helper's exhaustive match still keeps
that branch compiled.

- [ ] **Step 3: Extract the feature-gated adapter mapping**

In `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`, add:

```rust
#[cfg(feature = "accessibility")]
pub(crate) fn accessibility_event_from_winit(
    id: Id,
    event: accesskit_winit::WindowEvent,
) -> AccessibilityEvent {
    match event {
        accesskit_winit::WindowEvent::InitialTreeRequested => {
            AccessibilityEvent::InitialTreeRequested(id)
        }
        accesskit_winit::WindowEvent::ActionRequested(request) => {
            AccessibilityEvent::ActionRequested(AccessibilityActionRequest {
                id,
                action: format!("{:?}", request.action),
            })
        }
        accesskit_winit::WindowEvent::AccessibilityDeactivated => {
            AccessibilityEvent::Deactivated(id)
        }
    }
}
```

Then replace the inline mapping in `WinitRunner::user_event`:

```rust
let event = accessibility_event_from_winit(id, event.window_event);
self.deliver_event(event_loop, id, EventKind::Accessibility(event));
```

- [ ] **Step 4: Run feature-specific checks**

Run:

```sh
cargo test -p surgeist-window --features accessibility accessibility_feature_maps_accesskit_window_events
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
```

Expected: all pass. This remains `surgeist-window` work because the mapping lives in the native host adapter.

---

## Task 5: Final Window Verification And Review

**Files:**
- Inspect: `/Users/codex/Development/surgeist-window/src/tests.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`

- [ ] **Step 1: Run baseline checks**

Run:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 2: Run feature checks**

Run:

```sh
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
```

Expected: all pass.

- [ ] **Step 3: Reviewer gate**

Ask a clean reviewer to inspect the final changes. The reviewer must verify:

- the frivolous DSL test is gone
- new tests are behavior-focused and not compile-only assertions
- fake-host failure side effects are explicitly covered
- native transition tests exercise the new modeled boundary
- accessibility coverage is feature-gated and adapter-local
- no cross-crate or root integration work was added

- [ ] **Step 4: Commit in `surgeist-window` only**

After reviewer findings are clean and checks pass:

```sh
git add src/tests.rs src/winit_adapter.rs
git commit -m "test: align window coverage with modeled host boundaries"
```

---

## Completion Criteria

- `surgeist-window` contains no remaining `dsl_public_vocabulary_uses_lifecycle_draw_names` test.
- The replacement DSL test proves named state lookup, target command lowering, and action resolution.
- Fake-host failure tests document attempted-command recording plus rollback of callback-generated commands and command-induced state.
- Native transition tests cover event/patch pairs beyond focus and pointer movement.
- Accessibility mapping has a feature-gated unit test and a small adapter-local helper.
- `cargo test -p surgeist-window` passes.
- `cargo test -p surgeist-window --features accessibility` passes.
- `cargo clippy -p surgeist-window --all-targets -- -D warnings` passes.
- `cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings` passes.
- `cargo fmt --check` passes.
- A separate reviewer re-reviews the final window changes and comes back clean.
