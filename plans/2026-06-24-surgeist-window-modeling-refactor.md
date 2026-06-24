# Surgeist Window Modeling Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `surgeist-window` around explicit typed phases, host capabilities, command planning, and shared state transitions before new window functionality is added.

**Architecture:** Treat existing public compatibility as non-goal because Surgeist is not live yet. Keep app-facing window intent distinct from normalized host plans, observed runtime snapshots, backend capabilities, and test harness state. Make fake and real hosts consume shared validation and transition logic so future native features do not duplicate semantics in `testing::Host` and `WinitRunner`.

**Tech Stack:** Rust 2024, `winit` 0.30, existing `surgeist-window` crate, optional `accessibility` feature, crate-local unit tests in `src/tests.rs`, source-derived API refresh via `api/generator`, and guidance from `/Users/codex/Development/surgeist-window/guidance/surgeist-rust-modeling-guide.md`.

---

## Execution Location

This plan lives in the `surgeist-window` crate repo and must be executed from:

```text
/Users/codex/Development/surgeist-window/plans/2026-06-24-surgeist-window-modeling-refactor.md
```

Run implementation commands from `/Users/codex/Development/surgeist-window`.
Do not edit the root `surgeist` repo while implementing crate-local tasks. Root
submodule pointer updates happen later from the top-level coordinator after
crate checks are green and commits are pushed.

## Modeling Standard

Before implementing each task, read:

```text
/Users/codex/Development/surgeist-window/guidance/surgeist-rust-modeling-guide.md
```

This refactor specifically applies these sections:

- Model phases explicitly.
- Keep invariants at construction.
- Make conversion boundaries narrow.
- Normalize commands before applying them.
- Keep fake and real paths aligned.
- Capability checks are contracts.
- Public APIs need front doors.

## Current Problems

The current crate has good typed vocabulary, but important semantics are still implicit:

- `Command` is both app-facing intent and host execution input.
- `Descriptor` is app-authored request data, native host intent, and partial runtime seed data.
- `State` is observed runtime state but is publicly constructible with arbitrary field combinations.
- `testing::Host::apply` and `WinitRunner::apply_command` separately interpret the same command variants.
- Backend capability limits are scattered through branch checks and string errors.
- Native event translation combines backend decoding, registry mutation, pointer tracking, and event delivery in one large match.

This plan intentionally allows breaking public API shape to make these boundaries explicit before real apps depend on them.

## Target Model

Create these explicit phases:

- `WindowRequest`: app-authored window creation request.
- `WindowSnapshot`: observed runtime state.
- `HostCapabilities`: backend capability report.
- `HostCommand`: normalized host command after validation.
- `HostCommandPlan`: capability-checked normalized host command wrapper.
- `WindowStatePatch`: small state transition object used by fake and real hosts.
- `NativeEventTransition`: converted native event effects before delivery to handlers.

`HostCommandPlan` owns host-independent validation that needs only command data
and `HostCapabilities`: supported roles, fullscreen modes, cursor classes, and
other capability flags. Host application still owns registry/context validation
that needs live state, such as duplicate names, unknown window ids, actual
native window handles, and platform call failures. This split is intentional:
fake and real hosts share capability validation through planning, then each
host applies the normalized command against its own registry.

The plan keeps the existing `Open` DSL and `Command` name if convenient, but
their public shape may change. Compatibility with previous field-based
construction is not required.

## File Structure

- Modify `src/lib.rs`: update public front-door exports after model split.
- Modify `src/descriptor.rs`: split creation request, runtime snapshot, roles, modality, fullscreen, and metrics construction.
- Modify `src/command.rs`: distinguish app intent from normalized host command planning.
- Create `src/capability.rs`: host capability types and default `winit` capability report.
- Create `src/transition.rs`: state patch, host command plan, native event transition helpers.
- Modify `src/testing.rs`: make fake host consume shared command planning and state patches.
- Modify `src/winit_adapter.rs`: make native runner consume shared planning and event transition helpers.
- Modify `src/event.rs`: keep app-facing events but add constructors or helpers where event invariants matter.
- Modify `src/dsl.rs`: update builder/front-door APIs to produce the new request and command types.
- Modify `src/context.rs`: update command/action collection to use the new app-facing command API.
- Modify `src/loop_.rs`: update startup command storage and native runner handoff.
- Modify `src/tests.rs`: add characterization and migration tests for each task.
- Modify `api/public-api.txt`: refresh with the crate-local API generator after final API shape is intentional.
- Modify `README.md`: document the new modeling split and execution commands.

## Verification Commands

Run focused commands after each task:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Run feature checks before final commit:

```sh
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
cargo run --manifest-path api/generator/Cargo.toml
git diff -- api/public-api.txt
```

## Review Gates

Every task must receive a separate reviewer pass before commit. The reviewer must check:

- the task follows the modeling guide
- fake and real paths remain semantically aligned
- new types reduce coordination rather than moving complexity
- tests cover both successful and rejected paths
- no broad lint suppressions were introduced

---

## Task 0: Preflight And Baseline Characterization

**Files:**
- Inspect: `/Users/codex/Development/surgeist-window/AGENTS.md`
- Inspect: `/Users/codex/Development/surgeist-window/src/command.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Inspect: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Check repository status**

Run:

```sh
git status --short --branch
```

Expected: no unrelated local edits. If unrelated edits exist, stop and report them. Do not revert them.

- [ ] **Step 2: Add characterization tests for current fake-host command behavior**

Append these tests to `src/tests.rs` near the existing fake host tests:

```rust
#[test]
fn window_modeling_baseline_fake_host_records_failed_duplicate_command_without_new_window() {
    let mut host = testing::Host::new();

    host.apply(open("main")).unwrap();
    let error = host.apply(open("main")).expect_err("duplicate name should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert_eq!(host.commands().len(), 2);
    assert_eq!(host.registry().len(), 1);
}

#[test]
fn window_modeling_baseline_fake_host_destroy_removes_window_cursor_and_records_closed_state() {
    let mut host = testing::Host::new();

    host.apply(open("main")).unwrap();
    let id = host.window_id("main").expect("main window should exist");
    host.apply(Command::SetCursor {
        id,
        cursor: Cursor::Hidden,
    })
    .unwrap();
    host.apply(Command::Destroy { id }).unwrap();

    assert!(host.registry().get(id).is_none());
    assert!(
        host.events()
            .iter()
            .any(|event| matches!(event, testing::Event::Destroyed(destroyed) if *destroyed == id))
    );
}
```

- [ ] **Step 3: Run the characterization tests**

Run:

```sh
cargo test -p surgeist-window window_modeling_baseline_fake_host -- --nocapture
```

Expected: both tests pass. These tests capture current semantics before reshaping the model.

- [ ] **Step 4: Add characterization tests for current descriptor capability rejection**

Append these tests to `src/tests.rs` near descriptor tests:

```rust
#[test]
fn window_modeling_baseline_descriptor_rejects_non_root_roles_for_winit_attributes() {
    let descriptor = Descriptor {
        role: Role::Dialog {
            parent: Id::from_u64(1),
            modality: Modality::Window,
        },
        ..Descriptor::default()
    };

    let error = descriptor
        .to_winit_attributes()
        .expect_err("role support is not modeled yet");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn window_modeling_baseline_descriptor_rejects_exclusive_fullscreen_for_winit_attributes() {
    let descriptor = Descriptor {
        fullscreen: Fullscreen::Exclusive,
        ..Descriptor::default()
    };

    let error = descriptor
        .to_winit_attributes()
        .expect_err("exclusive fullscreen requires a native video mode");

    assert_eq!(error.code, ErrorCode::CommandFailed);
}
```

- [ ] **Step 5: Run the descriptor characterization tests**

Run:

```sh
cargo test -p surgeist-window window_modeling_baseline_descriptor -- --nocapture
```

Expected: both tests pass.

- [ ] **Step 6: Run baseline crate checks**

Run:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 7: Request reviewer pass**

Ask a clean reviewer to confirm the baseline tests document existing behavior only and do not begin the refactor.

- [ ] **Step 8: Commit**

Run:

```sh
git add src/tests.rs
git commit -m "test: characterize window modeling baseline"
```

## Task 1: Introduce Host Capabilities

**Files:**
- Create: `/Users/codex/Development/surgeist-window/src/capability.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write failing capability tests**

Add to `src/tests.rs`:

```rust
#[test]
fn host_capabilities_report_current_winit_support() {
    let capabilities = HostCapabilities::winit_default();

    assert!(capabilities.supports_role(RoleKind::Root));
    assert!(!capabilities.supports_role(RoleKind::Dialog));
    assert!(!capabilities.supports_role(RoleKind::Tool));
    assert!(!capabilities.supports_role(RoleKind::Popup));
    assert!(capabilities.supports_fullscreen(FullscreenMode::None));
    assert!(capabilities.supports_fullscreen(FullscreenMode::Borderless));
    assert!(!capabilities.supports_fullscreen(FullscreenMode::Exclusive));
    assert!(capabilities.supports_cursor(CursorCapability::Icon));
    assert!(capabilities.supports_cursor(CursorCapability::Hidden));
    assert!(!capabilities.supports_cursor(CursorCapability::Custom));
}

#[test]
fn host_capabilities_explain_rejections_with_stable_codes() {
    let capabilities = HostCapabilities::winit_default();

    let role_error = capabilities
        .require_role(RoleKind::Dialog)
        .expect_err("dialog roles are not yet supported by current winit plan");
    let fullscreen_error = capabilities
        .require_fullscreen(FullscreenMode::Exclusive)
        .expect_err("exclusive fullscreen is unsupported without video mode selection");

    assert_eq!(role_error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(fullscreen_error.code, ErrorCode::UnsupportedFeature);
}
```

Expected compile failure: `HostCapabilities`, `RoleKind`, `FullscreenMode`, and `CursorCapability` do not exist.

- [ ] **Step 2: Run the failing capability tests**

Run:

```sh
cargo test -p surgeist-window host_capabilities -- --nocapture
```

Expected: compile failure for missing capability types.

- [ ] **Step 3: Implement `src/capability.rs`**

Create `src/capability.rs`:

```rust
use super::{Error, ErrorCode};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RoleKind {
    Root,
    Dialog,
    Tool,
    Popup,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FullscreenMode {
    None,
    Borderless,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CursorCapability {
    Icon,
    Hidden,
    Custom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostCapabilities {
    root_role: bool,
    dialog_role: bool,
    tool_role: bool,
    popup_role: bool,
    borderless_fullscreen: bool,
    exclusive_fullscreen: bool,
    icon_cursor: bool,
    hidden_cursor: bool,
    custom_cursor: bool,
}

impl HostCapabilities {
    #[must_use]
    pub const fn winit_default() -> Self {
        Self {
            root_role: true,
            dialog_role: false,
            tool_role: false,
            popup_role: false,
            borderless_fullscreen: true,
            exclusive_fullscreen: false,
            icon_cursor: true,
            hidden_cursor: true,
            custom_cursor: false,
        }
    }

    #[must_use]
    pub const fn supports_role(&self, role: RoleKind) -> bool {
        match role {
            RoleKind::Root => self.root_role,
            RoleKind::Dialog => self.dialog_role,
            RoleKind::Tool => self.tool_role,
            RoleKind::Popup => self.popup_role,
        }
    }

    pub fn require_role(&self, role: RoleKind) -> Result<(), Error> {
        self.supports_role(role).then_some(()).ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedFeature,
                format!("native host does not support {role:?} window role"),
            )
        })
    }

    #[must_use]
    pub const fn supports_fullscreen(&self, fullscreen: FullscreenMode) -> bool {
        match fullscreen {
            FullscreenMode::None => true,
            FullscreenMode::Borderless => self.borderless_fullscreen,
            FullscreenMode::Exclusive => self.exclusive_fullscreen,
        }
    }

    pub fn require_fullscreen(&self, fullscreen: FullscreenMode) -> Result<(), Error> {
        self.supports_fullscreen(fullscreen)
            .then_some(())
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UnsupportedFeature,
                    format!("native host does not support {fullscreen:?} fullscreen"),
                )
            })
    }

    #[must_use]
    pub const fn supports_cursor(&self, cursor: CursorCapability) -> bool {
        match cursor {
            CursorCapability::Icon => self.icon_cursor,
            CursorCapability::Hidden => self.hidden_cursor,
            CursorCapability::Custom => self.custom_cursor,
        }
    }

    pub fn require_cursor(&self, cursor: CursorCapability) -> Result<(), Error> {
        self.supports_cursor(cursor).then_some(()).ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedFeature,
                format!("native host does not support {cursor:?} cursor"),
            )
        })
    }
}
```

- [ ] **Step 4: Export capability types**

Modify `src/lib.rs`:

```rust
mod capability;
pub use capability::{CursorCapability, FullscreenMode, HostCapabilities, RoleKind};
```

Place `mod capability;` with the other private modules and the `pub use` with other front-door exports.

- [ ] **Step 5: Run capability tests**

Run:

```sh
cargo test -p surgeist-window host_capabilities -- --nocapture
```

Expected: tests pass.

- [ ] **Step 6: Run crate checks**

Run:

```sh
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 7: Request reviewer pass**

Ask a clean reviewer to confirm capabilities are explicit contracts and not just renamed branch checks.

- [ ] **Step 8: Commit**

Run:

```sh
git add src/capability.rs src/lib.rs src/tests.rs
git commit -m "window: add host capability model"
```

## Task 2: Split Window Request From Runtime Snapshot

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/context.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/dsl.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/event.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/registry.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`

- [ ] **Step 1: Write failing request/snapshot tests**

Add to `src/tests.rs`:

```rust
#[test]
fn window_request_builds_authored_creation_intent_without_public_field_mutation() {
    let request = WindowRequest::builder("main")
        .title("Main Window")
        .inner_size(size(320, 240))
        .hidden()
        .borderless()
        .build();

    assert_eq!(request.name(), Some("main"));
    assert_eq!(request.title(), "Main Window");
    assert_eq!(request.inner_size(), Some(size(320, 240)));
    assert!(!request.visible());
    assert_eq!(request.fullscreen(), Fullscreen::Borderless);
    assert_eq!(request.role().kind(), RoleKind::Root);
}

#[test]
fn window_snapshot_is_observed_runtime_state_built_through_constructor() {
    let metrics = Metrics::from_physical_size(
        Id::from_u64(7),
        PhysicalSize {
            width: 640,
            height: 480,
        },
        2.0,
    );
    let snapshot = WindowSnapshot::new(Id::from_u64(7), "Main Window", metrics.clone())
        .named("main")
        .with_visible(true)
        .focused(true);

    assert_eq!(snapshot.id(), Id::from_u64(7));
    assert_eq!(snapshot.title(), "Main Window");
    assert_eq!(snapshot.name(), Some("main"));
    assert_eq!(snapshot.metrics(), &metrics);
    assert!(snapshot.is_visible());
    assert!(snapshot.is_focused());
}
```

Expected compile failure: `WindowRequest` and `WindowSnapshot` do not exist.

- [ ] **Step 2: Run failing request/snapshot tests**

Run:

```sh
cargo test -p surgeist-window window_request_builds -- --nocapture
cargo test -p surgeist-window window_snapshot_is -- --nocapture
```

Expected: compile failure for missing types.

- [ ] **Step 3: Introduce phase names without changing field visibility**

In `src/descriptor.rs`, rename the current `Descriptor` struct to
`WindowRequest` and the current `State` struct to `WindowSnapshot`. Keep the
same fields public for this step so existing internals compile unchanged. Add
temporary aliases immediately after the renamed structs:

```rust
pub type Descriptor = WindowRequest;
pub type State = WindowSnapshot;
```

Do not privatize fields in this step. The only behavior change in this step is
that the phase names exist and are exported.

- [ ] **Step 4: Add complete request builder and read accessors**

In `src/descriptor.rs`, add a complete builder for `WindowRequest`. The builder
must cover every current `Open` DSL setting so `src/dsl.rs` can delegate to it
without inventing additional API:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct WindowRequestBuilder {
    pub(crate) request: WindowRequest,
}

impl WindowRequest {
    #[must_use]
    pub fn builder(name: impl Into<String>) -> WindowRequestBuilder {
        WindowRequestBuilder {
            request: Self {
                name: Some(name.into()),
                ..Self::default()
            },
        }
    }
}

impl WindowRequestBuilder {
    #[must_use]
    pub fn build(self) -> WindowRequest {
        self.request
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.request.title = title.into();
        self
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.request.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn position(mut self, point: impl Into<Point>) -> Self {
        self.request.position = Some(point.into());
        self
    }

    #[must_use]
    pub fn inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub fn min_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.min_inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub fn max_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.request.max_inner_size = Some(size.into());
        self
    }

    #[must_use]
    pub const fn resizable(mut self, resizable: bool) -> Self {
        self.request.resizable = resizable;
        self
    }

    #[must_use]
    pub const fn fixed(self) -> Self {
        self.resizable(false)
    }

    #[must_use]
    pub fn controls(mut self, controls: impl Into<Controls>) -> Self {
        self.request.controls = controls.into();
        self
    }

    #[must_use]
    pub const fn decorations(mut self, enabled: bool) -> Self {
        self.request.decorations = enabled;
        self
    }

    #[must_use]
    pub const fn transparent(mut self, transparent: bool) -> Self {
        self.request.transparent = transparent;
        self
    }

    #[must_use]
    pub const fn visible(mut self, visible: bool) -> Self {
        self.request.visible = visible;
        self
    }

    #[must_use]
    pub const fn hidden(self) -> Self {
        self.visible(false)
    }

    #[must_use]
    pub fn fullscreen(mut self, fullscreen: impl Into<Fullscreen>) -> Self {
        self.request.fullscreen = fullscreen.into();
        self
    }

    #[must_use]
    pub fn borderless(mut self) -> Self {
        self.request.fullscreen = Fullscreen::Borderless;
        self
    }

    #[must_use]
    pub const fn level(mut self, level: Level) -> Self {
        self.request.level = level;
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: impl Into<Option<Theme>>) -> Self {
        self.request.theme = theme.into();
        self
    }

    #[must_use]
    pub const fn role(mut self, role: Role) -> Self {
        self.request.role = role;
        self
    }

    #[must_use]
    pub const fn root(self) -> Self {
        self.role(Role::Root)
    }

    #[must_use]
    pub const fn dialog(self, parent: Id) -> Self {
        self.role(Role::Dialog {
            parent,
            modality: Modality::Window,
        })
    }

    #[must_use]
    pub const fn modal(self, parent: Id, modality: Modality) -> Self {
        self.role(Role::Dialog { parent, modality })
    }

    #[must_use]
    pub const fn tool(self, parent: Option<Id>) -> Self {
        self.role(Role::Tool { parent })
    }

    #[must_use]
    pub const fn popup(self, parent: Id) -> Self {
        self.role(Role::Popup { parent })
    }
}
```

Add read accessors for every field the fake host, native adapter, tests, and
later planning tasks need:

```rust
use crate::RoleKind;

impl Role {
    #[must_use]
    pub const fn kind(&self) -> RoleKind {
        match self {
            Self::Root => RoleKind::Root,
            Self::Dialog { .. } => RoleKind::Dialog,
            Self::Tool { .. } => RoleKind::Tool,
            Self::Popup { .. } => RoleKind::Popup,
        }
    }
}

impl WindowRequest {
    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub const fn position(&self) -> Option<Point> {
        self.position
    }

    #[must_use]
    pub const fn inner_size(&self) -> Option<Size> {
        self.inner_size
    }

    #[must_use]
    pub const fn min_inner_size(&self) -> Option<Size> {
        self.min_inner_size
    }

    #[must_use]
    pub const fn max_inner_size(&self) -> Option<Size> {
        self.max_inner_size
    }

    #[must_use]
    pub const fn resizable(&self) -> bool {
        self.resizable
    }

    #[must_use]
    pub const fn controls(&self) -> Controls {
        self.controls
    }

    #[must_use]
    pub const fn decorations(&self) -> bool {
        self.decorations
    }

    #[must_use]
    pub const fn transparent(&self) -> bool {
        self.transparent
    }

    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }

    #[must_use]
    pub fn fullscreen(&self) -> Fullscreen {
        self.fullscreen.clone()
    }

    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    #[must_use]
    pub const fn theme(&self) -> Option<Theme> {
        self.theme
    }

    #[must_use]
    pub const fn role(&self) -> &Role {
        &self.role
    }
}
```

- [ ] **Step 5: Add snapshot constructors, accessors, and mutation helpers while fields are still public**

Add constructor and read APIs for `WindowSnapshot`, but keep fields public until
Step 7 completes the internal migration:

```rust
impl WindowSnapshot {
    #[must_use]
    pub fn new(id: Id, title: impl Into<String>, metrics: Metrics) -> Self {
        Self {
            id,
            title: title.into(),
            name: None,
            metrics,
            position: None,
            focused: false,
            visible: None,
            minimized: None,
            maximized: false,
            occluded: None,
            fullscreen: false,
            theme: None,
            role: Role::Root,
        }
    }

    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    #[must_use]
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    #[must_use]
    pub const fn id(&self) -> Id {
        self.id
    }

    #[must_use]
    pub const fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.title.as_str()
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub const fn visible(&self) -> Option<bool> {
        self.visible
    }

    #[must_use]
    pub const fn theme(&self) -> Option<Theme> {
        self.theme
    }

    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible.unwrap_or(true)
    }
}
```

Use `visible()` for the raw `Option<bool>` accessor. Use `with_visible(bool)`
for the builder-style snapshot mutation method to avoid overloading the getter
name.

- [ ] **Step 6: Migrate internals and tests to the new APIs**

Before fields become private, update `src/context.rs`, `src/dsl.rs`,
`src/event.rs`, `src/registry.rs`, `src/testing.rs`, `src/tests.rs`, and
`src/winit_adapter.rs` to prefer `WindowRequest`, `WindowSnapshot`,
`WindowRequestBuilder`, and the new accessors.

Update `Open` in `src/dsl.rs` so it stores `WindowRequestBuilder` while
preserving the current `Open` API. Do not force all existing call sites to move
from `.at()`, `.size()`, `.min()`, `.max()`, `.descriptor()`,
`.into_descriptor()`, or `.into_command()` in Task 2; keep those methods as
delegating compatibility methods until Task 8 removes stale names deliberately.

```rust
pub struct Open {
    builder: WindowRequestBuilder,
}

impl Open {
    #[must_use]
    pub fn unnamed() -> Self {
        Self {
            builder: WindowRequestBuilder {
                request: WindowRequest::default(),
            },
        }
    }

    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            builder: WindowRequest::builder(name),
        }
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.builder = self.builder.name(name);
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.builder = self.builder.title(title);
        self
    }

    #[must_use]
    pub fn position(mut self, point: impl Into<Point>) -> Self {
        self.builder = self.builder.position(point);
        self
    }

    #[must_use]
    pub fn at(self, point: impl Into<Point>) -> Self {
        self.position(point)
    }

    #[must_use]
    pub fn inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.inner_size(size);
        self
    }

    #[must_use]
    pub fn size(self, size: impl Into<Size>) -> Self {
        self.inner_size(size)
    }

    #[must_use]
    pub fn min_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.min_inner_size(size);
        self
    }

    #[must_use]
    pub fn min(self, size: impl Into<Size>) -> Self {
        self.min_inner_size(size)
    }

    #[must_use]
    pub fn max_inner_size(mut self, size: impl Into<Size>) -> Self {
        self.builder = self.builder.max_inner_size(size);
        self
    }

    #[must_use]
    pub fn max(self, size: impl Into<Size>) -> Self {
        self.max_inner_size(size)
    }

    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.builder = self.builder.resizable(resizable);
        self
    }

    #[must_use]
    pub fn fixed(mut self) -> Self {
        self.builder = self.builder.fixed();
        self
    }

    #[must_use]
    pub fn controls(mut self, controls: impl Into<Controls>) -> Self {
        self.builder = self.builder.controls(controls);
        self
    }

    #[must_use]
    pub fn decorations(mut self, enabled: bool) -> Self {
        self.builder = self.builder.decorations(enabled);
        self
    }

    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.builder = self.builder.transparent(transparent);
        self
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> Self {
        self.builder = self.builder.visible(visible);
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.builder = self.builder.hidden();
        self
    }

    #[must_use]
    pub fn fullscreen(mut self, fullscreen: impl Into<Fullscreen>) -> Self {
        self.builder = self.builder.fullscreen(fullscreen);
        self
    }

    #[must_use]
    pub fn borderless(mut self) -> Self {
        self.builder = self.builder.borderless();
        self
    }

    #[must_use]
    pub fn level(mut self, level: Level) -> Self {
        self.builder = self.builder.level(level);
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: impl Into<Option<Theme>>) -> Self {
        self.builder = self.builder.theme(theme);
        self
    }

    #[must_use]
    pub fn role(mut self, role: Role) -> Self {
        self.builder = self.builder.role(role);
        self
    }

    #[must_use]
    pub fn root(mut self) -> Self {
        self.builder = self.builder.root();
        self
    }

    #[must_use]
    pub fn dialog(mut self, parent: Id) -> Self {
        self.builder = self.builder.dialog(parent);
        self
    }

    #[must_use]
    pub fn modal(mut self, modality: Modality) -> Self {
        if let Role::Dialog { parent, .. } = self.builder.request.role.clone() {
            self.builder = self.builder.modal(parent, modality);
        }
        self
    }

    #[must_use]
    pub fn tool(mut self, parent: Option<Id>) -> Self {
        self.builder = self.builder.tool(parent);
        self
    }

    #[must_use]
    pub fn popup(mut self, parent: Id) -> Self {
        self.builder = self.builder.popup(parent);
        self
    }

    #[must_use]
    pub fn descriptor(&self) -> &WindowRequest {
        &self.builder.request
    }

    #[must_use]
    pub fn into_descriptor(self) -> WindowRequest {
        self.builder.build()
    }

    #[must_use]
    pub fn build(self) -> WindowRequest {
        self.into_descriptor()
    }

    #[must_use]
    pub fn into_command(self) -> Command {
        Command::Open {
            descriptor: self.into_descriptor(),
        }
    }
}

impl From<Open> for WindowRequest {
    fn from(open: Open) -> Self {
        open.into_descriptor()
    }
}

impl From<Open> for Command {
    fn from(open: Open) -> Self {
        open.into_command()
    }
}
```

Update descriptor literal tests to use `WindowRequest::builder("window")` plus
builder methods instead of struct literals. For example:

```rust
let request = WindowRequest::builder("window")
    .title("Window")
    .inner_size(size(320, 240))
    .min_inner_size(size(100, 80))
    .max_inner_size(size(800, 600))
    .borderless()
    .build();
```

Update state reads such as `state.visible` and `state.theme` to
`state.visible()` and `state.theme()`. Leave direct writes in native and
fake host code alone until Step 7 adds mutation helpers.

- [ ] **Step 7: Privatize fields and finish field-write migration**

Make `WindowRequest` and `WindowSnapshot` fields private only after Step 6
compiles. Then add a snapshot seed constructor so `testing.rs`,
`winit_adapter.rs`, and existing tests do not need struct literals:

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WindowSnapshotSeed {
    pub(crate) id: Id,
    pub(crate) title: String,
    pub(crate) name: Option<String>,
    pub(crate) metrics: Metrics,
    pub(crate) position: Option<Point>,
    pub(crate) focused: bool,
    pub(crate) visible: Option<bool>,
    pub(crate) minimized: Option<bool>,
    pub(crate) maximized: bool,
    pub(crate) occluded: Option<bool>,
    pub(crate) fullscreen: bool,
    pub(crate) theme: Option<Theme>,
    pub(crate) role: Role,
}

impl WindowSnapshot {
    #[must_use]
    pub(crate) fn from_seed(seed: WindowSnapshotSeed) -> Self {
        Self {
            id: seed.id,
            title: seed.title,
            name: seed.name,
            metrics: seed.metrics,
            position: seed.position,
            focused: seed.focused,
            visible: seed.visible,
            minimized: seed.minimized,
            maximized: seed.maximized,
            occluded: seed.occluded,
            fullscreen: seed.fullscreen,
            theme: seed.theme,
            role: seed.role,
        }
    }
}
```

Keep `WindowSnapshotSeed` crate-internal. It is a migration and test helper for
trusted crate code, not part of the public front-door API.

Update code that currently reads or writes `state.title`, `state.visible`, `state.metrics`, and related fields to use crate-visible methods. Add `pub(crate)` setters on `WindowSnapshot` for runtime transitions:

```rust
pub(crate) fn set_title(&mut self, title: String) { self.title = title; }
pub(crate) fn set_visible(&mut self, visible: Option<bool>) { self.visible = visible; }
pub(crate) fn set_metrics(&mut self, metrics: Metrics) { self.metrics = metrics; }
pub(crate) fn set_position(&mut self, position: Option<Point>) { self.position = position; }
pub(crate) fn set_focused(&mut self, focused: bool) { self.focused = focused; }
pub(crate) fn set_theme(&mut self, theme: Option<Theme>) { self.theme = theme; }
pub(crate) fn set_occluded(&mut self, occluded: Option<bool>) { self.occluded = occluded; }
pub(crate) fn set_fullscreen(&mut self, fullscreen: bool) { self.fullscreen = fullscreen; }
```

Update all existing test helpers that build `State` or `Descriptor` literals. In
`src/tests.rs`, import the crate-internal seed with:

```rust
use crate::descriptor::WindowSnapshotSeed;
```

Use this exact pattern for the current `state(id)` helper in `src/tests.rs`:

```rust
fn state(id: Id) -> WindowSnapshot {
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 800,
            height: 600,
        },
        1.0,
    );
    WindowSnapshot::from_seed(WindowSnapshotSeed {
        id,
        title: String::from("Window"),
        name: None,
        metrics,
        position: None,
        focused: false,
        visible: Some(true),
        minimized: Some(false),
        maximized: false,
        occluded: Some(false),
        fullscreen: false,
        theme: None,
        role: Role::Root,
    })
}
```

Update `fake_state_from_descriptor` in `src/testing.rs` and `state_from_winit`
in `src/winit_adapter.rs` to import `crate::descriptor::WindowSnapshotSeed` and
construct `WindowSnapshot::from_seed` with a full `WindowSnapshotSeed` value
containing `id`, `title`, `name`, `metrics`, `position`, `focused`, `visible`,
`minimized`, `maximized`, `occluded`, `fullscreen`, `theme`, and `role` rather
than struct literals. Update native and fake host writes to use the setters
above.

- [ ] **Step 8: Export new phase names**

Modify `src/lib.rs` to export:

```rust
pub use descriptor::{WindowRequest, WindowRequestBuilder, WindowSnapshot};
```

Keep aliases only until Task 8 removes compatibility names deliberately.

- [ ] **Step 9: Run tests**

Run:

```sh
cargo test -p surgeist-window window_request_builds -- --nocapture
cargo test -p surgeist-window window_snapshot_is -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 10: Request reviewer pass**

Ask a clean reviewer to confirm authored request and runtime snapshot are now phase-separated and not merely aliases in spirit.

- [ ] **Step 11: Commit**

Run:

```sh
git add src/descriptor.rs src/context.rs src/dsl.rs src/event.rs src/lib.rs src/registry.rs src/testing.rs src/tests.rs src/winit_adapter.rs
git commit -m "window: split request and snapshot phases"
```

## Task 3: Add Shared State Patches

**Files:**
- Create: `/Users/codex/Development/surgeist-window/src/transition.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write failing state patch tests**

Add to `src/tests.rs`:

```rust
#[test]
fn window_state_patch_updates_snapshot_and_reports_event() {
    let id = Id::from_u64(1);
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 800,
            height: 600,
        },
        1.0,
    );
    let mut snapshot = WindowSnapshot::new(id, "Main", metrics);

    let patch = WindowStatePatch::title(id, "Renamed");
    let event = patch.apply(&mut snapshot).expect("patch should apply");

    assert_eq!(snapshot.title(), "Renamed");
    assert!(event.is_none());

    let visible = WindowStatePatch::visible(id, false);
    let event = visible.apply(&mut snapshot).expect("visible patch should apply");
    assert_eq!(snapshot.visible(), Some(false));
    assert!(event.is_none());
}

#[test]
fn window_state_patch_rejects_wrong_target() {
    let id = Id::from_u64(1);
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 800,
            height: 600,
        },
        1.0,
    );
    let mut snapshot = WindowSnapshot::new(id, "Main", metrics);
    let patch = WindowStatePatch::title(Id::from_u64(2), "Wrong");

    let error = patch.apply(&mut snapshot).expect_err("wrong id should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
}
```

Expected compile failure: `WindowStatePatch` does not exist and `WindowSnapshot::visible()` accessor may not exist yet.

- [ ] **Step 2: Run failing patch tests**

Run:

```sh
cargo test -p surgeist-window window_state_patch -- --nocapture
```

Expected: compile failure for missing patch type or accessors.

- [ ] **Step 3: Implement `WindowStatePatch`**

Create `src/transition.rs`:

```rust
use super::{
    Error, ErrorCode, EventKind, Id, Metrics, Point, Theme, WindowSnapshot,
};

#[derive(Clone, Debug, PartialEq)]
pub enum WindowStatePatch {
    Title { id: Id, title: String },
    Position { id: Id, position: Point },
    Visible { id: Id, visible: bool },
    Metrics { metrics: Metrics, event: MetricsEvent },
    Focused { id: Id, focused: bool },
    Theme { id: Id, theme: Option<Theme> },
    Occluded { id: Id, occluded: bool },
    Fullscreen { id: Id, fullscreen: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricsEvent {
    Resized,
    ScaleFactorChanged,
}

impl WindowStatePatch {
    #[must_use]
    pub fn title(id: Id, title: impl Into<String>) -> Self {
        Self::Title {
            id,
            title: title.into(),
        }
    }

    #[must_use]
    pub const fn visible(id: Id, visible: bool) -> Self {
        Self::Visible { id, visible }
    }

    #[must_use]
    pub const fn metrics(metrics: Metrics, event: MetricsEvent) -> Self {
        Self::Metrics { metrics, event }
    }

    #[must_use]
    pub fn id(&self) -> Id {
        match self {
            Self::Title { id, .. }
            | Self::Position { id, .. }
            | Self::Visible { id, .. }
            | Self::Focused { id, .. }
            | Self::Theme { id, .. }
            | Self::Occluded { id, .. }
            | Self::Fullscreen { id, .. } => *id,
            Self::Metrics { metrics, .. } => metrics.id(),
        }
    }

    pub fn apply(self, snapshot: &mut WindowSnapshot) -> Result<Option<EventKind>, Error> {
        let id = self.id();
        if snapshot.id() != id {
            return Err(Error::new(ErrorCode::CommandFailed, "patch target does not match window")
                .with_id(id));
        }
        match self {
            Self::Title { title, .. } => {
                snapshot.set_title(title);
                Ok(None)
            }
            Self::Position { position, .. } => {
                snapshot.set_position(Some(position));
                Ok(Some(EventKind::Moved { id, position }))
            }
            Self::Visible { visible, .. } => {
                snapshot.set_visible(Some(visible));
                Ok(None)
            }
            Self::Metrics { metrics, event } => {
                snapshot.set_metrics(metrics.clone());
                let event = match event {
                    MetricsEvent::Resized => EventKind::Resized(metrics),
                    MetricsEvent::ScaleFactorChanged => EventKind::ScaleFactorChanged(metrics),
                };
                Ok(Some(event))
            }
            Self::Focused { focused, .. } => {
                snapshot.set_focused(focused);
                Ok(Some(EventKind::Focused { id, focused }))
            }
            Self::Theme { theme, .. } => {
                snapshot.set_theme(theme);
                Ok(Some(EventKind::ThemeChanged { id, theme }))
            }
            Self::Occluded { occluded, .. } => {
                snapshot.set_occluded(Some(occluded));
                Ok(Some(EventKind::Occluded { id, occluded }))
            }
            Self::Fullscreen { fullscreen, .. } => {
                snapshot.set_fullscreen(fullscreen);
                Ok(None)
            }
        }
    }
}
```

Add `Metrics::id()` in `src/descriptor.rs` if not already present:

```rust
#[must_use]
pub const fn id(&self) -> Id {
    self.id
}
```

- [ ] **Step 4: Export transition types**

Modify `src/lib.rs`:

```rust
mod transition;
pub use event::EventKind;
pub use transition::{MetricsEvent, WindowStatePatch};
```

`EventKind` is already the production event vocabulary used by `src/event.rs`.
Export it intentionally as a release-facing public event vocabulary. This is
required because public transition APIs in Task 7 expose converted events.
Include this in the API artifact refresh in Task 8.

- [ ] **Step 5: Use patches in fake host for state-updating commands**

Modify `src/testing.rs` command branches for title, position, visible, inner size, fullscreen, and theme to use `WindowStatePatch` where possible. For example:

```rust
Command::SetTitle { id, title } => {
    self.apply_patch(WindowStatePatch::title(id, title))?;
}
```

Add helper:

```rust
fn apply_patch(&mut self, patch: WindowStatePatch) -> Result<()> {
    let id = patch.id();
    let state = self.state_mut(id)?;
    if let Some(event) = patch.apply(state)? {
        self.events.push(event.into());
    }
    Ok(())
}
```

Add `impl From<EventKind> for testing::Event` in `src/testing.rs` or a local conversion helper so fake events stay aligned with production event vocabulary.

- [ ] **Step 6: Use patches in native runner for state updates**

Modify `src/winit_adapter.rs` state updates for title, visible, theme, moved, focused, occluded, resize, and scale factor to use `WindowStatePatch`. Native side effects still happen in `WinitRunner`, but snapshot mutation goes through the patch.

- [ ] **Step 7: Run patch tests and crate checks**

Run:

```sh
cargo test -p surgeist-window window_state_patch -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 8: Request reviewer pass**

Ask a clean reviewer to verify fake and real state mutation paths now share patch semantics.

- [ ] **Step 9: Commit**

Run:

```sh
git add src/transition.rs src/lib.rs src/descriptor.rs src/testing.rs src/winit_adapter.rs src/tests.rs
git commit -m "window: centralize state transitions"
```

## Task 4: Add Host Command Planning

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/command.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/context.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/cursor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/dsl.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/loop_.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/transition.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write failing host command plan tests**

Add to `src/tests.rs`:

```rust
#[test]
fn host_command_plan_rejects_unsupported_dialog_role_before_host_application() {
    let capabilities = HostCapabilities::winit_default();
    let request = WindowRequest::builder("dialog")
        .title("Dialog")
        .dialog(Id::from_u64(1))
        .build();
    let command = Command::Open { request };

    let error = HostCommandPlan::from_command(command, &capabilities)
        .expect_err("dialog role is not supported by current host capabilities");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn host_command_plan_rejects_custom_cursor_before_host_application() {
    let capabilities = HostCapabilities::winit_default();
    let command = Command::SetCursor {
        id: Id::from_u64(1),
        cursor: Cursor::Custom(CustomCursorId::from_u64(9)),
    };

    let error = HostCommandPlan::from_command(command, &capabilities)
        .expect_err("custom cursors are not supported by current host capabilities");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn host_command_plan_keeps_supported_commands_as_normalized_host_commands() {
    let capabilities = HostCapabilities::winit_default();
    let command = Command::SetTitle {
        id: Id::from_u64(1),
        title: String::from("Renamed"),
    };

    let plan = HostCommandPlan::from_command(command, &capabilities).unwrap();

    assert!(matches!(
        plan.command(),
        HostCommand::SetTitle { id, title }
            if *id == Id::from_u64(1) && title == "Renamed"
    ));
}
```

Expected compile failure: `HostCommandPlan` and `HostCommand` do not exist, and `Command::Open` may still use `descriptor` instead of `request`.

- [ ] **Step 2: Run failing host command plan tests**

Run:

```sh
cargo test -p surgeist-window host_command_plan -- --nocapture
```

Expected: compile failure for missing plan types.

- [ ] **Step 3: Define normalized host command types**

In `src/transition.rs`, merge the top `use super` import block from Task 3
with the new command-planning imports so there is still only one import block:

```rust
use super::{
    Command, Controls, Cursor, CursorCapability, CursorGrab, Error, ErrorCode, EventKind,
    Fullscreen, FullscreenMode, HostCapabilities, Id, ImeRequest, Level, Metrics, Point, Size,
    Theme, WindowRequest, WindowSnapshot,
};
```

Then add:

```rust

#[derive(Clone, Debug, PartialEq)]
pub enum HostCommand {
    Open { request: WindowRequest },
    SetTitle { id: Id, title: String },
    SetPosition { id: Id, position: Point },
    SetVisible { id: Id, visible: bool },
    SetResizable { id: Id, resizable: bool },
    SetControls { id: Id, controls: Controls },
    SetDecorations { id: Id, decorations: bool },
    SetTransparent { id: Id, transparent: bool },
    SetInnerSize { id: Id, size: Size },
    SetMinInnerSize { id: Id, size: Option<Size> },
    SetMaxInnerSize { id: Id, size: Option<Size> },
    SetFullscreen { id: Id, fullscreen: Fullscreen },
    SetLevel { id: Id, level: Level },
    SetTheme { id: Id, theme: Option<Theme> },
    SetCursor { id: Id, cursor: Cursor },
    SetCursorGrab { id: Id, grab: CursorGrab },
    SetIme { id: Id, request: ImeRequest },
    RequestUserAttention { id: Id },
    RequestDraw { id: Id },
    Destroy { id: Id },
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostCommandPlan {
    command: HostCommand,
}

impl HostCommandPlan {
    pub fn from_command(command: Command, capabilities: &HostCapabilities) -> Result<Self, Error> {
        let command = match command {
            Command::Open { request } => {
                capabilities.require_role(request.role().kind())?;
                capabilities.require_fullscreen(request.fullscreen().mode())?;
                HostCommand::Open { request }
            }
            Command::SetFullscreen { id, fullscreen } => {
                capabilities.require_fullscreen(fullscreen.mode())?;
                HostCommand::SetFullscreen { id, fullscreen }
            }
            Command::SetCursor { id, cursor } => {
                capabilities.require_cursor(cursor.capability())?;
                HostCommand::SetCursor { id, cursor }
            }
            Command::SetTitle { id, title } => HostCommand::SetTitle { id, title },
            Command::SetPosition { id, position } => HostCommand::SetPosition { id, position },
            Command::SetVisible { id, visible } => HostCommand::SetVisible { id, visible },
            Command::SetResizable { id, resizable } => HostCommand::SetResizable { id, resizable },
            Command::SetControls { id, controls } => HostCommand::SetControls { id, controls },
            Command::SetDecorations { id, decorations } => {
                HostCommand::SetDecorations { id, decorations }
            }
            Command::SetTransparent { id, transparent } => {
                HostCommand::SetTransparent { id, transparent }
            }
            Command::SetInnerSize { id, size } => HostCommand::SetInnerSize { id, size },
            Command::SetMinInnerSize { id, size } => HostCommand::SetMinInnerSize { id, size },
            Command::SetMaxInnerSize { id, size } => HostCommand::SetMaxInnerSize { id, size },
            Command::SetLevel { id, level } => HostCommand::SetLevel { id, level },
            Command::SetTheme { id, theme } => HostCommand::SetTheme { id, theme },
            Command::SetCursorGrab { id, grab } => HostCommand::SetCursorGrab { id, grab },
            Command::SetIme { id, request } => HostCommand::SetIme { id, request },
            Command::RequestUserAttention { id } => HostCommand::RequestUserAttention { id },
            Command::RequestDraw { id } => HostCommand::RequestDraw { id },
            Command::Destroy { id } => HostCommand::Destroy { id },
        };
        Ok(Self { command })
    }

    #[must_use]
    pub fn command(&self) -> &HostCommand {
        &self.command
    }

    #[must_use]
    pub fn into_command(self) -> HostCommand {
        self.command
    }
}
```

Add planning helper methods:

```rust
impl Fullscreen {
    #[must_use]
    pub const fn mode(&self) -> FullscreenMode {
        match self {
            Self::None => FullscreenMode::None,
            Self::Borderless => FullscreenMode::Borderless,
            Self::Exclusive => FullscreenMode::Exclusive,
        }
    }
}

impl Cursor {
    #[must_use]
    pub const fn capability(&self) -> CursorCapability {
        match self {
            Self::Icon(_) => CursorCapability::Icon,
            Self::Hidden => CursorCapability::Hidden,
            Self::Custom(_) => CursorCapability::Custom,
        }
    }
}
```

- [ ] **Step 4: Update `Command::Open` naming**

Modify `src/command.rs`:

```rust
pub enum Command {
    Open { request: WindowRequest },
    SetTitle { id: Id, title: String },
    SetPosition { id: Id, position: Point },
    SetVisible { id: Id, visible: bool },
    SetResizable { id: Id, resizable: bool },
    SetControls { id: Id, controls: Controls },
    SetDecorations { id: Id, decorations: bool },
    SetTransparent { id: Id, transparent: bool },
    SetInnerSize { id: Id, size: Size },
    SetMinInnerSize { id: Id, size: Option<Size> },
    SetMaxInnerSize { id: Id, size: Option<Size> },
    SetFullscreen { id: Id, fullscreen: Fullscreen },
    SetLevel { id: Id, level: Level },
    SetTheme { id: Id, theme: Option<Theme> },
    SetCursor { id: Id, cursor: Cursor },
    SetCursorGrab { id: Id, grab: CursorGrab },
    SetIme { id: Id, request: ImeRequest },
    RequestUserAttention { id: Id },
    RequestDraw { id: Id },
    Destroy { id: Id },
}
```

Migrate the `Command::Open` field rename from `descriptor` to `request` across
the crate. Use this search before editing to find the mechanical migration
surface:

```sh
rg -n "Command::Open|\\bdescriptor\\b" src
```

After editing, rerun:

```sh
rg -n "Command::Open \\{ descriptor|\\bdescriptor:" src
```

Expected: no `Command::Open { descriptor` construction sites remain. Remaining
`descriptor` hits are allowed only for temporary type aliases, comments, or
Task 8 cleanup notes that still intentionally mention the old name.

- [ ] **Step 5: Export host command plan types**

Modify `src/lib.rs`:

```rust
pub use transition::{HostCommand, HostCommandPlan, MetricsEvent, WindowStatePatch};
```

- [ ] **Step 6: Run host command plan tests**

Run:

```sh
cargo test -p surgeist-window host_command_plan -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 7: Request reviewer pass**

Ask a clean reviewer to confirm capability-sensitive rejection happens in planning, not separately in fake and native application branches.

- [ ] **Step 8: Commit**

Run:

```sh
git add src/command.rs src/transition.rs src/descriptor.rs src/cursor.rs src/lib.rs src/dsl.rs src/context.rs src/loop_.rs src/testing.rs src/winit_adapter.rs src/tests.rs
git commit -m "window: add host command planning"
```

## Task 5: Move Fake Host To Shared Planning

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write fake/real alignment tests for planned command behavior**

Add to `src/tests.rs`:

```rust
#[test]
fn fake_host_rejects_unsupported_commands_through_host_planning() {
    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();

    let cursor_error = host
        .apply(Command::SetCursor {
            id,
            cursor: Cursor::Custom(CustomCursorId::from_u64(1)),
        })
        .expect_err("custom cursor should be rejected by capabilities");
    let fullscreen_error = host
        .apply(Command::SetFullscreen {
            id,
            fullscreen: Fullscreen::Exclusive,
        })
        .expect_err("exclusive fullscreen should be rejected by capabilities");

    assert_eq!(cursor_error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(fullscreen_error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn fake_host_uses_shared_state_patch_for_visible_and_theme_commands() {
    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();

    host.apply(Command::SetVisible { id, visible: false }).unwrap();
    host.apply(Command::SetTheme {
        id,
        theme: Some(Theme::Dark),
    })
    .unwrap();

    let state = host.registry().get(id).unwrap().state();
    assert_eq!(state.visible(), Some(false));
    assert_eq!(state.theme(), Some(Theme::Dark));
    assert!(host.events().iter().any(|event| {
        matches!(event, testing::Event::ThemeChanged { id: event_id, theme: Some(Theme::Dark) } if *event_id == id)
    }));
}
```

- [ ] **Step 2: Run the focused tests**

Run:

```sh
cargo test -p surgeist-window fake_host_rejects_unsupported -- --nocapture
cargo test -p surgeist-window fake_host_uses_shared -- --nocapture
```

Expected: first test may fail if fake host still bypasses planning.

- [ ] **Step 3: Add capabilities to fake host**

Modify `testing::Host`:

```rust
#[derive(Debug)]
pub struct Host {
    registry: Registry,
    draw: DrawScheduler,
    capabilities: HostCapabilities,
    events: Vec<Event>,
    commands: Vec<Command>,
    closed: HashMap<Id, WindowSnapshot>,
    cursors: HashMap<Id, Cursor>,
    cursor_updates: Vec<(Id, Cursor)>,
    ime_requests: Vec<(Id, ImeRequest)>,
}

impl Default for Host {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
            draw: DrawScheduler::default(),
            capabilities: HostCapabilities::winit_default(),
            events: Vec::new(),
            commands: Vec::new(),
            closed: HashMap::new(),
            cursors: HashMap::new(),
            cursor_updates: Vec::new(),
            ime_requests: Vec::new(),
        }
    }
}
```

Remove the old `#[derive(Default)]` from `Host`. Keep `Host::new()` returning
`Self::default()`, now backed by the manual `Default` impl above. Add:

```rust
#[must_use]
pub fn capabilities(&self) -> &HostCapabilities {
    &self.capabilities
}
```

- [ ] **Step 4: Plan commands before fake host application**

Modify `Host::apply`:

```rust
pub fn apply(&mut self, command: impl Into<Command>) -> Result<()> {
    let command = command.into();
    self.commands.push(command.clone());
    let plan = HostCommandPlan::from_command(command, &self.capabilities)?;
    self.apply_plan(plan)
}
```

Add:

```rust
fn apply_plan(&mut self, plan: HostCommandPlan) -> Result<()> {
    match plan.into_command() {
        HostCommand::Open { request } => self.apply_open_request(request),
        HostCommand::SetTitle { id, title } => {
            self.apply_patch(WindowStatePatch::title(id, title))
        }
        HostCommand::SetPosition { id, position } => {
            self.apply_patch(WindowStatePatch::Position { id, position })
        }
        HostCommand::SetVisible { id, visible } => {
            self.apply_patch(WindowStatePatch::visible(id, visible))
        }
        HostCommand::SetInnerSize { id, size } => self.apply_inner_size(id, size),
        HostCommand::SetFullscreen { id, fullscreen } => self.apply_patch(
            WindowStatePatch::Fullscreen {
                id,
                fullscreen: !matches!(fullscreen, Fullscreen::None),
            },
        ),
        HostCommand::SetTheme { id, theme } => {
            self.apply_patch(WindowStatePatch::Theme { id, theme })
        }
        HostCommand::SetCursor { id, cursor } => self.apply_cursor(id, cursor),
        HostCommand::SetIme { id, request } => self.apply_ime_request(id, request),
        HostCommand::RequestDraw { id } => self.apply_draw_request(id),
        HostCommand::Destroy { id } => self.apply_destroy(id),
        HostCommand::SetResizable { id, .. }
        | HostCommand::SetControls { id, .. }
        | HostCommand::SetDecorations { id, .. }
        | HostCommand::SetTransparent { id, .. }
        | HostCommand::SetMinInnerSize { id, .. }
        | HostCommand::SetMaxInnerSize { id, .. }
        | HostCommand::SetLevel { id, .. }
        | HostCommand::SetCursorGrab { id, .. }
        | HostCommand::RequestUserAttention { id } => self.require_window(id),
    }
}
```

Move the old match body into these helper methods:

- `apply_open_request(request)` creates the fake state, inserts it in the registry, and pushes the created event.
- `apply_inner_size(id, size)` updates metrics using the current fake-host size behavior, then routes the metrics write through `WindowStatePatch::metrics`.
- `apply_cursor(id, cursor)` records the cursor update after `require_window(id)` succeeds.
- `apply_ime_request(id, request)` records the IME request after `require_window(id)` succeeds.
- `apply_draw_request(id)` schedules a draw after `require_window(id)` succeeds.
- `apply_destroy(id)` removes the active window state and records it in `closed`.

Do not mutate `WindowSnapshot` directly in these helpers when a
`WindowStatePatch` variant exists for the change.

Use these helper bodies as the implementation target:

```rust
fn apply_open_request(&mut self, request: WindowRequest) -> Result<()> {
    validate_name(&self.registry, request.name())?;
    let id = self.registry.reserve_id();
    let state = fake_state_from_request(id, &request);
    self.registry.insert(Instance::new(id, state.clone()));
    self.events.push(Event::Created(state));
    Ok(())
}

fn apply_inner_size(&mut self, id: Id, size: Size) -> Result<()> {
    let state = self.state_mut(id)?;
    let scale = state.metrics().scale_factor;
    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: (size.width * scale).round().max(0.0) as u32,
            height: (size.height * scale).round().max(0.0) as u32,
        },
        scale,
    )
    .with_outer_geometry(state.metrics().outer_position, state.metrics().outer_size);
    self.apply_patch(WindowStatePatch::metrics(metrics, MetricsEvent::Resized))
}

fn apply_cursor(&mut self, id: Id, cursor: Cursor) -> Result<()> {
    self.require_window(id)?;
    if self.cursors.get(&id) != Some(&cursor) {
        self.cursors.insert(id, cursor.clone());
        self.cursor_updates.push((id, cursor));
    }
    Ok(())
}

fn apply_ime_request(&mut self, id: Id, request: ImeRequest) -> Result<()> {
    self.require_window(id)?;
    self.ime_requests.push((id, request));
    Ok(())
}

fn apply_draw_request(&mut self, id: Id) -> Result<()> {
    self.require_window(id)?;
    self.draw.request(&Action::DrawNext(id));
    Ok(())
}

fn apply_destroy(&mut self, id: Id) -> Result<()> {
    self.require_window(id)?;
    if let Some(instance) = self.registry.remove(id) {
        self.closed.insert(id, instance.state().clone());
    }
    self.cursors.remove(&id);
    self.events.push(Event::Destroyed(id));
    Ok(())
}
```

Rename `fake_state_from_descriptor` to `fake_state_from_request` in this task
and update its argument type to `&WindowRequest`.

- [ ] **Step 5: Run fake host focused tests and full crate checks**

Run:

```sh
cargo test -p surgeist-window fake_host_rejects_unsupported -- --nocapture
cargo test -p surgeist-window fake_host_uses_shared -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 6: Request reviewer pass**

Ask a clean reviewer to compare fake host behavior against the modeling guide's fake-real alignment section.

- [ ] **Step 7: Commit**

Run:

```sh
git add src/testing.rs src/tests.rs
git commit -m "window: route fake host through command planning"
```

## Task 6: Move Native Runner To Shared Planning

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write native runner planning tests**

Add internal tests to `src/tests.rs`:

```rust
#[test]
fn winit_runner_exposes_current_capabilities_for_command_planning() {
    struct Noop;
    impl Handler for Noop {}

    let runner = WinitRunner::from_loop(Loop::new(Noop));

    assert_eq!(runner.capabilities(), &HostCapabilities::winit_default());
}

#[test]
fn winit_runner_rejects_unsupported_command_before_native_application_in_tests() {
    struct Noop;
    impl Handler for Noop {}

    let runner = WinitRunner::from_loop(Loop::new(Noop));
    let command = Command::Open {
        request: WindowRequest::builder("dialog")
            .dialog(Id::from_u64(1))
            .build(),
    };

    let error = runner
        .plan_command_for_test(command)
        .expect_err("dialog role should be rejected before native create");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}
```

Expected compile failure: `WinitRunner::capabilities` and `plan_command_for_test` do not exist.

- [ ] **Step 2: Run failing tests**

Run:

```sh
cargo test -p surgeist-window winit_runner_exposes_current_capabilities -- --nocapture
cargo test -p surgeist-window winit_runner_rejects_unsupported -- --nocapture
```

Expected: compile failure for missing runner methods.

- [ ] **Step 3: Add capabilities to `WinitRunner`**

Modify `src/winit_adapter.rs`:

```rust
pub(crate) struct WinitRunner<H> {
    handler: H,
    registry: Registry,
    draw: DrawScheduler,
    capabilities: HostCapabilities,
    _clipboard: Box<dyn Clipboard>,
    pub(crate) commands: Vec<Command>,
    pub(crate) startup: Vec<Command>,
    windows: HashMap<winit::window::WindowId, Id>,
    hovered_files: HashMap<Id, Vec<String>>,
    modifiers: ModifierState,
    pub(crate) pointer_positions: HashMap<PointerPositionKey, Point>,
    cursor_state: HashMap<Id, Cursor>,
    pending_draws: HashSet<Id>,
    #[cfg(feature = "accessibility")]
    accessibility: HashMap<Id, accesskit_winit::Adapter>,
    pub(crate) proxy: Option<Proxy>,
    startup_applied: bool,
}
```

Initialize with `HostCapabilities::winit_default()`.

Add:

```rust
#[must_use]
pub(crate) fn capabilities(&self) -> &HostCapabilities {
    &self.capabilities
}

#[cfg(test)]
pub(crate) fn plan_command_for_test(&self, command: Command) -> Result<HostCommandPlan> {
    HostCommandPlan::from_command(command, &self.capabilities)
}
```

- [ ] **Step 4: Plan commands in native user event handling**

Modify:

```rust
UserEvent::Command(command) => {
    let result = HostCommandPlan::from_command(command, &self.capabilities)
        .and_then(|plan| self.apply_host_command(event_loop, plan.into_command()));
    if let Err(error) = result {
        eprintln!("{error}");
        event_loop.exit();
    }
}
```

Rename `apply_command` to `apply_host_command` and make it accept `HostCommand` instead of `Command`.

- [ ] **Step 5: Route queued startup and callback commands through planning**

Find the existing `apply_commands` helper in `src/winit_adapter.rs`. Keep the
queueing behavior from startup and handler callbacks, but make every queued
command pass through `HostCommandPlan` before native application:

```rust
fn apply_commands(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<()> {
    let commands = std::mem::take(&mut self.commands);
    for command in commands {
        let plan = HostCommandPlan::from_command(command, &self.capabilities)?;
        self.apply_host_command(event_loop, plan.into_command())?;
    }
    Ok(())
}
```

This preserves queue order while making startup commands, user-event commands,
and commands produced by handler callbacks use the same capability gate.

- [ ] **Step 6: Remove duplicate capability branch checks from native application**

In `apply_host_command`, unsupported role/fullscreen/custom cursor branches should no longer create capability errors because planning already handled them. Keep native fallible errors such as actual cursor grab failure.

- [ ] **Step 7: Run runner tests and full crate checks**

Run:

```sh
cargo test -p surgeist-window winit_runner_exposes_current_capabilities -- --nocapture
cargo test -p surgeist-window winit_runner_rejects_unsupported -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 8: Request reviewer pass**

Ask a clean reviewer to confirm native planning uses the same capability model as fake host planning.

- [ ] **Step 9: Commit**

Run:

```sh
git add src/winit_adapter.rs src/tests.rs
git commit -m "window: route native runner through command planning"
```

## Task 7: Narrow Native Event Translation

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/transition.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`

- [ ] **Step 1: Write failing event transition tests**

Add to `src/tests.rs`:

```rust
#[test]
fn native_event_transition_updates_focus_state_and_emits_event() {
    let id = Id::from_u64(1);
    let transition = NativeEventTransition::focused(id, true);

    assert_eq!(
        transition.patch(),
        Some(&WindowStatePatch::Focused { id, focused: true })
    );
    assert_eq!(
        transition.event(),
        Some(&EventKind::Focused { id, focused: true })
    );
}

#[test]
fn native_pointer_transition_preserves_logical_and_physical_positions() {
    let id = Id::from_u64(1);
    let input = NativeEventTransition::mouse_moved(
        id,
        Point { x: 12.0, y: 24.0 },
        PhysicalPoint { x: 24, y: 48 },
        None,
        ModifierState::default(),
    );

    let Some(EventKind::Input(InputEvent::Pointer(pointer))) = input.event() else {
        panic!("expected pointer input event");
    };

    assert_eq!(pointer.position, Some(Point { x: 12.0, y: 24.0 }));
    assert_eq!(pointer.physical_position, Some(PhysicalPoint { x: 24, y: 48 }));
}
```

Expected compile failure: `NativeEventTransition` does not exist.

- [ ] **Step 2: Run failing event transition tests**

Run:

```sh
cargo test -p surgeist-window native_event_transition -- --nocapture
cargo test -p surgeist-window native_pointer_transition -- --nocapture
```

Expected: compile failure for missing event transition type.

- [ ] **Step 3: Implement `NativeEventTransition`**

In `src/transition.rs`, merge these event-transition imports into the existing
top `use super` import block from Tasks 3 and 4. Do not add a second `use
super` block:

```rust
use super::{
    Command, Controls, Cursor, CursorCapability, CursorGrab, Error, ErrorCode, EventKind,
    Fullscreen, FullscreenMode, HostCapabilities, Id, ImeRequest, InputEvent, Level, Metrics,
    ModifierState, PhysicalPoint, Point, PointerDeviceData, PointerEvent, PointerKind,
    PointerPhase, Size, Theme, WindowRequest, WindowSnapshot,
};
```

Then add:

```rust

#[derive(Clone, Debug, PartialEq)]
pub struct NativeEventTransition {
    patch: Option<WindowStatePatch>,
    event: Option<EventKind>,
}

impl NativeEventTransition {
    #[must_use]
    pub const fn new(patch: Option<WindowStatePatch>, event: Option<EventKind>) -> Self {
        Self { patch, event }
    }

    #[must_use]
    pub fn focused(id: Id, focused: bool) -> Self {
        let patch = WindowStatePatch::Focused { id, focused };
        let event = EventKind::Focused { id, focused };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn moved(id: Id, position: Point) -> Self {
        let patch = WindowStatePatch::Position { id, position };
        let event = EventKind::Moved { id, position };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn theme_changed(id: Id, theme: Option<Theme>) -> Self {
        let patch = WindowStatePatch::Theme { id, theme };
        let event = EventKind::ThemeChanged { id, theme };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn occluded(id: Id, occluded: bool) -> Self {
        let patch = WindowStatePatch::Occluded { id, occluded };
        let event = EventKind::Occluded { id, occluded };
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn resized(metrics: Metrics) -> Self {
        let event = EventKind::Resized(metrics.clone());
        let patch = WindowStatePatch::metrics(metrics, MetricsEvent::Resized);
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn scale_factor_changed(metrics: Metrics) -> Self {
        let event = EventKind::ScaleFactorChanged(metrics.clone());
        let patch = WindowStatePatch::metrics(metrics, MetricsEvent::ScaleFactorChanged);
        Self::new(Some(patch), Some(event))
    }

    #[must_use]
    pub fn mouse_moved(
        id: Id,
        position: Point,
        physical_position: PhysicalPoint,
        delta: Option<Point>,
        modifiers: ModifierState,
    ) -> Self {
        Self::new(
            None,
            Some(EventKind::Input(InputEvent::Pointer(PointerEvent {
                id,
                phase: PointerPhase::Moved,
                kind: PointerKind::Mouse,
                pointer_id: None,
                position: Some(position),
                physical_position: Some(physical_position),
                delta,
                button: None,
                modifiers,
                device: PointerDeviceData::default(),
                timestamp: None,
            }))),
        )
    }

    #[must_use]
    pub fn patch(&self) -> Option<&WindowStatePatch> {
        self.patch.as_ref()
    }

    #[must_use]
    pub fn event(&self) -> Option<&EventKind> {
        self.event.as_ref()
    }

    #[must_use]
    pub fn into_event(self) -> Option<EventKind> {
        self.event
    }
}
```

Use exactly these constructors in this task: `focused`, `moved`,
`theme_changed`, `occluded`, `resized`, `scale_factor_changed`, and
`mouse_moved`. Keyboard input, IME, file drag, cursor enter/leave, mouse button,
wheel, and touch event construction stay in `src/winit_adapter.rs` for this
plan; do not partially move them without tests.

- [ ] **Step 4: Export native event transition**

Modify `src/lib.rs` so the transition export includes `NativeEventTransition`:

```rust
pub use transition::{
    HostCommand, HostCommandPlan, MetricsEvent, NativeEventTransition, WindowStatePatch,
};
```

- [ ] **Step 5: Migrate lifecycle event branches**

In `src/winit_adapter.rs`, migrate exactly these `WindowEvent` branches to
build `NativeEventTransition` values and apply them through one helper:

- `WindowEvent::Moved(position)` -> `NativeEventTransition::moved(id, point)`
- `WindowEvent::Focused(focused)` -> `NativeEventTransition::focused(id, focused)`
- `WindowEvent::ThemeChanged(theme)` -> `NativeEventTransition::theme_changed(id, theme)`
- `WindowEvent::Occluded(occluded)` -> `NativeEventTransition::occluded(id, occluded)`
- `WindowEvent::Resized(size)` -> build `NativeEventTransition::resized(metrics)`, apply only its patch, then call `deliver_resize(event_loop, id)`.
- `WindowEvent::ScaleFactorChanged { .. }` -> build `NativeEventTransition::scale_factor_changed(metrics)`, apply only its patch, then call `deliver_resize(event_loop, id)`.

Use this helper for those branches:

```rust
fn apply_native_transition(
    &mut self,
    event_loop: &winit::event_loop::ActiveEventLoop,
    id: Id,
    transition: NativeEventTransition,
) {
    if let Some(patch) = transition.patch().cloned()
        && let Some(instance) = self.registry.get_mut(id)
    {
        let _ = patch.apply(instance.state_mut());
    }
    if let Some(event) = transition.into_event() {
        self.deliver_event(event_loop, id, event);
    }
}
```

For `Resized` and `ScaleFactorChanged`, preserve the existing resize handler
flow. Do not call `apply_native_transition` for those two branches because it
would deliver the event directly. Instead, clone/apply `transition.patch()` to
the state and then call `deliver_resize(event_loop, id)`. The transition
centralizes snapshot mutation and event vocabulary; it must not drop the
current handler resize callback behavior.

- [ ] **Step 6: Migrate pointer movement construction**

Migrate only `WindowEvent::CursorMoved` pointer event construction in this task.
Keep pointer position storage and delta calculation in `WinitRunner` because it
depends on runner state. Once `position`, `physical_position`, `delta`, and
`modifiers` are known, build the event with:

```rust
let transition = NativeEventTransition::mouse_moved(
    id,
    position,
    physical_position,
    delta,
    self.modifiers,
);
self.apply_native_transition(event_loop, id, transition);
```

Leave mouse buttons, wheel, touch, keyboard, IME, file drag, and cursor
enter/leave construction unchanged in this task. They need their own focused
tests before being moved.

- [ ] **Step 7: Run event transition tests and full crate checks**

Run:

```sh
cargo test -p surgeist-window native_event_transition -- --nocapture
cargo test -p surgeist-window native_pointer_transition -- --nocapture
cargo test -p surgeist-window
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 8: Request reviewer pass**

Ask a clean reviewer to confirm native event translation is narrower and testable without opening real windows.

- [ ] **Step 9: Commit**

Run:

```sh
git add src/lib.rs src/transition.rs src/winit_adapter.rs src/tests.rs
git commit -m "window: narrow native event translation"
```

## Task 8: Remove Compatibility Aliases And Refresh Front Door

**Files:**
- Modify: `/Users/codex/Development/surgeist-window/src/lib.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/descriptor.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/command.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/context.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/dsl.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/event.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/registry.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/testing.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/tests.rs`
- Modify: `/Users/codex/Development/surgeist-window/src/winit_adapter.rs`
- Modify: `/Users/codex/Development/surgeist-window/api/public-api.txt`
- Modify: `/Users/codex/Development/surgeist-window/README.md`

- [ ] **Step 1: Write public API front-door test**

Add to `src/tests.rs`:

```rust
#[test]
fn window_public_front_door_uses_modeled_phase_names() {
    let _ = std::mem::size_of::<WindowRequest>();
    let _ = std::mem::size_of::<WindowRequestBuilder>();
    let _ = std::mem::size_of::<WindowSnapshot>();
    let _ = std::mem::size_of::<HostCapabilities>();
    let _ = std::mem::size_of::<HostCommandPlan>();
    let _ = std::mem::size_of::<HostCommand>();
    let _ = std::mem::size_of::<WindowStatePatch>();
    let _ = std::mem::size_of::<NativeEventTransition>();
}
```

- [ ] **Step 2: Remove old compatibility aliases**

Remove:

```rust
pub type Descriptor = WindowRequest;
pub type State = WindowSnapshot;
```

Update all code and tests to use `WindowRequest` and `WindowSnapshot`. If keeping `Descriptor` or `State` is desired for naming taste, keep them as the real types only if their names still communicate the phase. Otherwise prefer the phase names.

- [ ] **Step 3: Update public exports**

Modify `src/lib.rs` so front-door exports include the modeled types and remove stale compatibility exports:

```rust
pub use descriptor::{
    Controls, Fullscreen, Level, Metrics, Modality, Role, Theme, WindowRequest,
    WindowRequestBuilder, WindowSnapshot,
};
pub use transition::{
    HostCommand, HostCommandPlan, MetricsEvent, NativeEventTransition, WindowStatePatch,
};
```

- [ ] **Step 4: Update README**

Add a short modeling section to `README.md`:

```markdown
## Model

`surgeist-window` separates app-authored window requests, normalized host command
plans, observed runtime snapshots, and backend capabilities. The fake test host
and the native `winit` runner share command planning and state transition
helpers so tests exercise the same semantics as production paths.
```

- [ ] **Step 5: Refresh API artifact**

Run:

```sh
cargo run --manifest-path api/generator/Cargo.toml
```

Review:

```sh
git diff -- api/public-api.txt
```

Expected: API artifact reflects the new modeled phase names.

- [ ] **Step 6: Run final checks**

Run:

```sh
cargo test -p surgeist-window
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
cargo fmt --check
```

Expected: all pass.

- [ ] **Step 7: Request final reviewer pass**

Ask a clean reviewer to review the whole branch against:

```text
/Users/codex/Development/surgeist-window/guidance/surgeist-rust-modeling-guide.md
```

Reviewer must explicitly answer:

- Are authored request, normalized command, runtime snapshot, backend capability, and test state separated?
- Do fake and real paths share command planning?
- Are capability checks modeled as contracts?
- Does the refactor reduce future coordination?
- Are any broad enum or public field issues still blocking new functionality?

- [ ] **Step 8: Commit**

Run:

```sh
git add README.md api/public-api.txt src/command.rs src/context.rs src/descriptor.rs src/dsl.rs src/event.rs src/lib.rs src/registry.rs src/testing.rs src/tests.rs src/winit_adapter.rs
git commit -m "window: refresh modeled public API"
```

## Final Verification

After all commits, run:

```sh
git status --short --branch
cargo test -p surgeist-window
cargo test -p surgeist-window --features accessibility
cargo clippy -p surgeist-window --all-targets -- -D warnings
cargo clippy -p surgeist-window --all-targets --features accessibility -- -D warnings
cargo fmt --check
```

Expected: clean git status and all commands pass.

## Completion Criteria

This plan is complete only when:

- all tasks are implemented in the `surgeist-window` crate repo
- each task has a clean reviewer pass
- final reviewer pass comes back clean against the modeling guide
- crate checks and accessibility feature checks pass
- API artifact is refreshed
- crate commits are pushed for top-level pointer update

## Coordinator Handoff

After completion, report to the top-level coordinator:

- final `surgeist-window` commit SHA
- pushed branch or `main` status
- final check outputs
- reviewer summary
- any root integration or sibling crate requests

The top-level coordinator then updates the root submodule pointer only after root integration checks pass.
