use super::command::Action;
use super::testing::{Effect, Event as HostEvent};
#[cfg(feature = "accessibility")]
use super::winit_adapter::accessibility_event_from_winit;
use super::winit_adapter::{
    NativeTransitionRoute, PointerPositionKey, WinitRunner, code_from_winit, ime_event_from_winit,
    key_from_winit, location_from_winit, native_transition_route,
};
use super::*;
use crate::descriptor::WindowSnapshotSeed;
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};

fn state(id: Id) -> WindowSnapshot {
    WindowSnapshot::from_seed(WindowSnapshotSeed {
        id,
        title: String::from("Test"),
        name: Some(String::from("surgeist-test")),
        metrics: Metrics::from_physical_size(
            id,
            PhysicalSize {
                width: 200,
                height: 100,
            },
            2.0,
        ),
        position: Some(Point { x: 10.0, y: 20.0 }),
        focused: false,
        visible: Some(true),
        minimized: Some(false),
        maximized: false,
        occluded: Some(false),
        fullscreen: false,
        theme: Some(Theme::Dark),
        role: Role::Root,
    })
}

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

#[test]
fn metrics_convert_physical_to_logical() {
    let metrics = Metrics::from_physical_size(
        Id::from_u64(7),
        PhysicalSize {
            width: 300,
            height: 150,
        },
        1.5,
    );

    assert_eq!(
        metrics.logical_size,
        Size {
            width: 200.0,
            height: 100.0
        }
    );
    assert_eq!(metrics.scale_factor, 1.5);
}

#[test]
fn metrics_convert_points_between_logical_and_physical_space() {
    let metrics = Metrics::from_physical_size(
        Id::from_u64(7),
        PhysicalSize {
            width: 300,
            height: 150,
        },
        1.5,
    );

    let physical = metrics.logical_to_physical_point(Point { x: 20.0, y: 10.0 });

    assert_eq!(physical, PhysicalPoint { x: 30, y: 15 });
    assert_eq!(
        metrics.physical_to_logical_point(physical),
        Point { x: 20.0, y: 10.0 }
    );
}

#[test]
fn metrics_preserve_outer_geometry() {
    let metrics = Metrics::from_physical_size(
        Id::from_u64(7),
        PhysicalSize {
            width: 300,
            height: 150,
        },
        1.5,
    )
    .with_outer_geometry(
        Some(Point { x: 10.0, y: 20.0 }),
        Some(Size {
            width: 220.0,
            height: 120.0,
        }),
    );

    assert_eq!(metrics.outer_position, Some(Point { x: 10.0, y: 20.0 }));
    assert_eq!(
        metrics.outer_size,
        Some(Size {
            width: 220.0,
            height: 120.0
        })
    );
}

#[test]
fn registry_tracks_multiple_windows() {
    let mut registry = Registry::new();
    let first = registry.reserve_id();
    let second = registry.reserve_id();

    registry.insert(Instance::new(first, state(first)));
    registry.insert(Instance::new(second, state(second)));

    assert_eq!(registry.len(), 2);
    assert!(registry.contains(first));
    assert_eq!(registry.remove(first).map(|entry| entry.id()), Some(first));
    assert!(!registry.contains(first));
    assert!(registry.contains(second));
}

#[test]
fn draw_scheduler_coalesces_and_chooses_earliest_deadline() {
    let id = Id::from_u64(1);
    let later = Instant::now() + std::time::Duration::from_secs(5);
    let earlier = Instant::now() + std::time::Duration::from_secs(1);
    let mut scheduler = DrawScheduler::new();

    scheduler.request(&Action::DrawAt { id, time: later });
    scheduler.request(&Action::DrawAt { id, time: earlier });

    assert_eq!(scheduler.next_deadline(), Some(earlier));
    assert_eq!(scheduler.take_ready(earlier), vec![id]);

    scheduler.request(&Action::DrawNext(id));
    scheduler.request(&Action::DrawNext(id));

    assert_eq!(scheduler.next_deadline(), None);
    assert_eq!(scheduler.take_ready(Instant::now()), vec![id]);
}

#[test]
fn delayed_draw_waits_until_deadline() {
    let id = Id::from_u64(1);
    let now = Instant::now();
    let deadline = now + std::time::Duration::from_millis(20);
    let mut scheduler = DrawScheduler::new();

    scheduler.request(&Action::DrawAt { id, time: deadline });

    assert_eq!(scheduler.take_ready(now), Vec::<Id>::new());
    assert_eq!(scheduler.next_deadline(), Some(deadline));
    assert_eq!(scheduler.take_ready(deadline), vec![id]);
}

#[test]
fn draw_scheduler_exposes_backend_neutral_deadline() {
    let id = Id::from_u64(1);
    let deadline = Instant::now() + std::time::Duration::from_millis(20);
    let mut scheduler = DrawScheduler::new();

    assert_eq!(scheduler.next_deadline(), None);
    assert_eq!(scheduler.take_ready(Instant::now()), Vec::<Id>::new());

    scheduler.request(&Action::DrawAt { id, time: deadline });

    assert_eq!(scheduler.next_deadline(), Some(deadline));
    assert_eq!(scheduler.take_ready(deadline), vec![id]);
}

#[test]
fn pointer_position_keys_separate_mouse_and_touch_contacts() {
    let window = Id::from_u64(1);
    let mouse = Point { x: 1.0, y: 2.0 };
    let first_touch = Point { x: 3.0, y: 4.0 };
    let second_touch = Point { x: 5.0, y: 6.0 };
    let mut positions = HashMap::new();

    positions.insert(PointerPositionKey::mouse(window), mouse);
    positions.insert(PointerPositionKey::touch(window, 10), first_touch);
    positions.insert(PointerPositionKey::touch(window, 11), second_touch);
    positions.remove(&PointerPositionKey::touch(window, 10));

    assert_eq!(
        positions.get(&PointerPositionKey::mouse(window)),
        Some(&mouse)
    );
    assert_eq!(
        positions.get(&PointerPositionKey::touch(window, 11)),
        Some(&second_touch)
    );
    assert!(!positions.contains_key(&PointerPositionKey::touch(window, 10)));
}

#[test]
fn file_drag_position_uses_last_mouse_position_when_available() {
    struct Noop;
    impl Handler for Noop {}

    let window = Id::from_u64(1);
    let mouse = Point { x: 8.0, y: 13.0 };
    let mut runner = WinitRunner::from_loop(Loop::new(Noop));

    assert_eq!(runner.last_mouse_position(window), None);

    runner
        .pointer_positions
        .insert(PointerPositionKey::mouse(window), mouse);

    assert_eq!(runner.last_mouse_position(window), Some(mouse));
}

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

#[test]
fn memory_clipboard_round_trips_text_and_image() {
    let mut clipboard = MemoryClipboard::new();
    clipboard.write_text("hello").unwrap();
    assert_eq!(clipboard.read_text().unwrap(), Some(String::from("hello")));

    clipboard
        .write_image(ClipboardImageRef {
            width: 1,
            height: 1,
            rgba: &[255, 0, 0, 255],
        })
        .unwrap();

    assert_eq!(
        clipboard.read_image().unwrap(),
        Some(ClipboardImage {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        })
    );
}

#[test]
fn missing_live_handle_reports_stable_error() {
    let id = Id::from_u64(9);
    let instance = Instance::new(id, state(id));
    let error = instance.as_ref().handle().unwrap_err();

    assert_eq!(error.code, ErrorCode::HandleUnavailable);
    assert_eq!(error.id, Some(id));
}

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
    let event = visible
        .apply(&mut snapshot)
        .expect("visible patch should apply");
    assert_eq!(snapshot.visible(), Some(false));
    assert!(event.is_none());
}

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

    let error = patch
        .apply(&mut snapshot)
        .expect_err("wrong id should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
}

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
    assert_eq!(
        pointer.physical_position,
        Some(PhysicalPoint { x: 24, y: 48 })
    );
}

#[test]
fn native_pointer_transition_routes_to_input_delivery() {
    let id = Id::from_u64(1);
    let transition = NativeEventTransition::mouse_moved(
        id,
        Point { x: 12.0, y: 24.0 },
        PhysicalPoint { x: 24, y: 48 },
        Some(Point { x: 1.0, y: 2.0 }),
        ModifierState::default(),
    );

    let Some(event) = transition.into_event() else {
        panic!("expected pointer input event");
    };
    assert_eq!(
        native_transition_route(&event),
        NativeTransitionRoute::Input
    );
    let EventKind::Input(InputEvent::Pointer(pointer)) = event else {
        panic!("expected pointer input event");
    };

    assert_eq!(pointer.id, id);
    assert_eq!(pointer.phase, PointerPhase::Moved);
    assert_eq!(pointer.position, Some(Point { x: 12.0, y: 24.0 }));
    assert_eq!(
        pointer.physical_position,
        Some(PhysicalPoint { x: 24, y: 48 })
    );
}

#[test]
fn native_transition_route_sends_lifecycle_events_to_event_delivery() {
    let id = Id::from_u64(1);
    let event = NativeEventTransition::focused(id, true)
        .into_event()
        .expect("focus transition should emit event");

    assert_eq!(
        native_transition_route(&event),
        NativeTransitionRoute::Event
    );
}

#[test]
fn native_event_delivery_invokes_handler_event_callback_for_modeled_events() {
    #[derive(Default)]
    struct Record {
        events: Vec<EventKind>,
        state_observed: bool,
    }

    struct Recorder(Rc<RefCell<Record>>);

    impl Handler for Recorder {
        fn event(&mut self, event: &mut Event<'_>) -> Result<()> {
            let mut record = self.0.borrow_mut();
            record.events.push(event.event().clone());
            record.state_observed = event.state().is_some();
            event.context_mut().open(open("child"));
            event.again();
            Ok(())
        }
    }

    let record = Rc::new(RefCell::new(Record::default()));
    let mut window_loop = Loop::new(Recorder(record.clone()));
    let id = window_loop.registry.reserve_id();
    window_loop.registry.insert(Instance::new(id, state(id)));
    let mut runner = WinitRunner::from_loop(window_loop);
    let event = EventKind::Focused { id, focused: true };

    let action = runner
        .call_with_event_for_test(event.clone())
        .expect("generic event callback should run");

    assert_eq!(record.borrow().events, vec![event]);
    assert!(record.borrow().state_observed);
    assert_eq!(action, Action::DrawNow(id));
    assert!(matches!(
        runner.commands.as_slice(),
        [Command::Open { request }] if request.name() == Some("child")
    ));
}

#[test]
fn created_event_close_skips_ready_delivery_when_window_is_destroyed() {
    #[derive(Default)]
    struct Record {
        created: usize,
        ready: usize,
    }

    struct Recorder(Rc<RefCell<Record>>);

    impl Handler for Recorder {
        fn event(&mut self, event: &mut Event<'_>) -> Result<()> {
            if matches!(event.event(), EventKind::Created(_)) {
                self.0.borrow_mut().created += 1;
                event.close();
            }
            Ok(())
        }

        fn ready(&mut self, ready: &mut Ready<'_>) -> Result<()> {
            self.0.borrow_mut().ready += 1;
            let _ = ready.state();
            Ok(())
        }
    }

    let record = Rc::new(RefCell::new(Record::default()));
    let mut window_loop = Loop::new(Recorder(record.clone()));
    let id = window_loop.registry.reserve_id();
    let window_state = state(id);
    window_loop
        .registry
        .insert(Instance::new(id, window_state.clone()));
    let mut runner = WinitRunner::from_loop(window_loop);

    runner
        .deliver_created_then_ready_for_test(window_state)
        .expect("created callback should be delivered without ready after close");

    assert_eq!(record.borrow().created, 1);
    assert_eq!(record.borrow().ready, 0);
    assert!(!runner.registry_contains_for_test(id));
}

#[test]
fn fake_host_native_transition_dispatch_matches_native_event_callback_contract() {
    #[derive(Default)]
    struct Recorder {
        event: Option<EventKind>,
        focused: Option<bool>,
        calls: usize,
    }

    impl Handler for Recorder {
        fn event(&mut self, event: &mut Event<'_>) -> Result<()> {
            self.calls += 1;
            self.event = Some(event.event().clone());
            self.focused = event.state().map(WindowSnapshot::is_focused);
            event.draw();
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    host.clear();
    let mut handler = Recorder::default();

    let effect = host
        .dispatch_native_transition(&mut handler, NativeEventTransition::focused(id, true))
        .expect("native transition should dispatch through generic event callback");

    assert_eq!(
        handler.event,
        Some(EventKind::Focused { id, focused: true })
    );
    assert_eq!(handler.focused, Some(true));
    assert_eq!(host.events(), &[HostEvent::Focused { id, focused: true }]);
    assert_eq!(effect, Effect::Draw(id));

    host.clear();
    let effect = host
        .dispatch_native_transition(
            &mut handler,
            NativeEventTransition::new(Some(WindowStatePatch::visible(id, false)), None),
        )
        .expect("patch-only native transition should still apply state");

    assert_eq!(effect, Effect::Wait);
    assert_eq!(handler.calls, 1);
    assert!(host.events().is_empty());
    assert_eq!(
        host.registry()
            .get(id)
            .map(|window| window.instance.state().visible()),
        Some(Some(false))
    );
}

#[test]
fn specialized_resize_input_close_callbacks_do_not_double_deliver_generic_event() {
    #[derive(Default)]
    struct Recorder {
        generic: usize,
        resize: usize,
        input: usize,
        close: usize,
        closed: usize,
    }

    impl Handler for Recorder {
        fn event(&mut self, _event: &mut Event<'_>) -> Result<()> {
            self.generic += 1;
            Ok(())
        }

        fn resize(&mut self, _win: &mut Resize<'_>) -> Result<()> {
            self.resize += 1;
            Ok(())
        }

        fn input(&mut self, _input: &mut Input<'_>) -> Result<()> {
            self.input += 1;
            Ok(())
        }

        fn close(&mut self, close: &mut Close<'_>) -> Result<()> {
            self.close += 1;
            close.close();
            Ok(())
        }

        fn closed(&mut self, _closed: &mut Closed<'_>) -> Result<()> {
            self.closed += 1;
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    let mut handler = Recorder::default();

    host.dispatch_resize(
        &mut handler,
        Metrics::from_physical_size(
            id,
            PhysicalSize {
                width: 400,
                height: 200,
            },
            1.0,
        ),
    )
    .unwrap();
    host.dispatch_input(
        &mut handler,
        InputEvent::Modifiers {
            id,
            modifiers: ModifierState::default(),
        },
    )
    .unwrap();
    host.dispatch_close(&mut handler, id).unwrap();
    host.dispatch_closed(&mut handler, id).unwrap();

    let mouse_moved_error = host
        .dispatch_native_transition(
            &mut handler,
            NativeEventTransition::mouse_moved(
                id,
                Point { x: 12.0, y: 24.0 },
                PhysicalPoint { x: 12, y: 24 },
                None,
                ModifierState::default(),
            ),
        )
        .expect_err("fake native transition dispatch must not generic-deliver input");
    assert_eq!(mouse_moved_error.code, ErrorCode::UnsupportedFeature);

    let resized_error = host
        .dispatch_native_transition(
            &mut handler,
            NativeEventTransition::resized(Metrics::from_physical_size(
                id,
                PhysicalSize {
                    width: 500,
                    height: 300,
                },
                1.0,
            )),
        )
        .expect_err("fake native transition dispatch must not generic-deliver resize");
    assert_eq!(resized_error.code, ErrorCode::UnsupportedFeature);

    let scale_error = host
        .dispatch_native_transition(
            &mut handler,
            NativeEventTransition::scale_factor_changed(Metrics::from_physical_size(
                id,
                PhysicalSize {
                    width: 500,
                    height: 300,
                },
                2.0,
            )),
        )
        .expect_err("fake native transition dispatch must not generic-deliver scale changes");
    assert_eq!(scale_error.code, ErrorCode::UnsupportedFeature);

    assert_eq!(handler.resize, 1);
    assert_eq!(handler.input, 1);
    assert_eq!(handler.close, 1);
    assert_eq!(handler.closed, 1);
    assert_eq!(handler.generic, 0);
}

#[test]
fn winit_mapping_converts_window_request_to_native_attributes() {
    let request = WindowRequest::builder("surgeist-window")
        .title("Window")
        .position(Point { x: 12.0, y: 24.0 })
        .inner_size(Size {
            width: 800.0,
            height: 600.0,
        })
        .min_inner_size(Size {
            width: 320.0,
            height: 240.0,
        })
        .max_inner_size(Size {
            width: 1600.0,
            height: 1200.0,
        })
        .fixed()
        .controls(Controls {
            close: true,
            minimize: false,
            maximize: true,
        })
        .decorations(false)
        .transparent(true)
        .hidden()
        .borderless()
        .level(Level::AlwaysOnTop)
        .theme(Some(Theme::Dark))
        .root()
        .build();

    let attributes = super::winit_mapping::window_attributes_from_request(&request).unwrap();

    assert_eq!(attributes.title, "Window");
    assert_eq!(
        attributes.position,
        Some(winit::dpi::Position::Logical(
            winit::dpi::LogicalPosition::new(12.0, 24.0)
        ))
    );
    assert_eq!(
        attributes.inner_size,
        Some(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            800.0, 600.0
        )))
    );
    assert_eq!(
        attributes.min_inner_size,
        Some(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            320.0, 240.0
        )))
    );
    assert_eq!(
        attributes.max_inner_size,
        Some(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            1600.0, 1200.0
        )))
    );
    assert!(!attributes.resizable);
    assert_eq!(
        attributes.enabled_buttons,
        winit::window::WindowButtons::CLOSE | winit::window::WindowButtons::MAXIMIZE
    );
    assert!(!attributes.decorations);
    assert!(attributes.transparent);
    assert!(!attributes.visible);
    assert_eq!(
        attributes.window_level,
        winit::window::WindowLevel::AlwaysOnTop
    );
    assert_eq!(attributes.preferred_theme, Some(winit::window::Theme::Dark));
    assert!(matches!(
        attributes.fullscreen,
        Some(winit::window::Fullscreen::Borderless(None))
    ));
}

#[test]
fn exclusive_fullscreen_requires_native_video_mode() {
    let request = WindowRequest::builder("exclusive")
        .fullscreen(Fullscreen::Exclusive)
        .build();

    let error = super::winit_mapping::window_attributes_from_request(&request).unwrap_err();

    assert_eq!(error.code, ErrorCode::CommandFailed);
}

#[test]
fn native_window_request_rejects_unimplemented_roles() {
    let request = WindowRequest::builder("dialog")
        .dialog(Id::from_u64(1))
        .build();

    let error = super::winit_mapping::window_attributes_from_request(&request).unwrap_err();

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert!(error.message.contains("roles"));
}

#[test]
fn window_modeling_baseline_request_rejects_non_root_roles_for_winit_attributes() {
    let request = WindowRequest::builder("dialog")
        .dialog(Id::from_u64(1))
        .build();

    let error = super::winit_mapping::window_attributes_from_request(&request)
        .expect_err("role support is not modeled yet");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn window_modeling_baseline_request_rejects_exclusive_fullscreen_for_winit_attributes() {
    let request = WindowRequest::builder("exclusive")
        .fullscreen(Fullscreen::Exclusive)
        .build();

    let error = super::winit_mapping::window_attributes_from_request(&request)
        .expect_err("exclusive fullscreen requires a native video mode");

    assert_eq!(error.code, ErrorCode::CommandFailed);
}

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
fn host_command_plan_rejects_exclusive_fullscreen_open_before_host_application() {
    let capabilities = HostCapabilities::winit_default();
    let request = WindowRequest::builder("exclusive")
        .fullscreen(Fullscreen::Exclusive)
        .build();
    let command = Command::Open { request };

    let error = HostCommandPlan::from_command(command, &capabilities)
        .expect_err("exclusive fullscreen opens are not supported by current host capabilities");

    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn host_command_plan_rejects_exclusive_fullscreen_command_before_host_application() {
    let capabilities = HostCapabilities::winit_default();
    let command = Command::SetFullscreen {
        id: Id::from_u64(1),
        fullscreen: Fullscreen::Exclusive,
    };

    let error = HostCommandPlan::from_command(command, &capabilities)
        .expect_err("exclusive fullscreen commands are not supported by current host capabilities");

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

#[test]
fn modifier_state_converts_from_winit() {
    let modifiers = winit::keyboard::ModifiersState::SHIFT
        | winit::keyboard::ModifiersState::CONTROL
        | winit::keyboard::ModifiersState::SUPER;

    assert_eq!(
        ModifierState::from(modifiers),
        ModifierState {
            shift: true,
            control: true,
            alt: false,
            super_key: true,
        }
    );
}

#[test]
fn pointer_button_converts_from_winit() {
    assert_eq!(
        PointerButton::from(winit::event::MouseButton::Left),
        PointerButton::Primary
    );
    assert_eq!(
        PointerButton::from(winit::event::MouseButton::Other(42)),
        PointerButton::Other(42)
    );
}

#[test]
fn wheel_delta_converts_from_winit() {
    assert_eq!(
        WheelDelta::from(winit::event::MouseScrollDelta::LineDelta(1.5, -2.0)),
        WheelDelta::Lines { x: 1.5, y: -2.0 }
    );
    assert_eq!(
        WheelDelta::from(winit::event::MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(12.0, 24.0)
        )),
        WheelDelta::Pixels { x: 12.0, y: 24.0 }
    );
}

#[test]
fn keyboard_identity_converts_from_winit() {
    assert_eq!(
        key_from_winit(&winit::keyboard::Key::Character("x".into())),
        keyboard_types::Key::Character(String::from("x"))
    );
    assert_eq!(
        key_from_winit(&winit::keyboard::Key::Named(
            winit::keyboard::NamedKey::Enter
        )),
        keyboard_types::Key::Enter
    );
    assert_eq!(
        code_from_winit(&winit::keyboard::PhysicalKey::Code(
            winit::keyboard::KeyCode::KeyA
        )),
        keyboard_types::Code::KeyA
    );
    assert_eq!(
        location_from_winit(winit::keyboard::KeyLocation::Numpad),
        keyboard_types::Location::Numpad
    );
}

#[test]
fn ime_event_converts_from_winit() {
    assert_eq!(
        ime_event_from_winit(
            Id::from_u64(1),
            winit::event::Ime::Preedit(String::from("draft"), Some((1, 2))),
        ),
        ImeEvent::Preedit {
            id: Id::from_u64(1),
            text: String::from("draft"),
            cursor: Some((1, 2)),
        }
    );
    assert_eq!(
        ime_event_from_winit(
            Id::from_u64(1),
            winit::event::Ime::Commit(String::from("done"))
        ),
        ImeEvent::Commit {
            id: Id::from_u64(1),
            text: String::from("done"),
        }
    );
}

#[test]
fn fake_host_applies_open_and_state_commands() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::builder("fake")
            .title("Fake")
            .position(Point { x: 11.0, y: 22.0 })
            .inner_size(Size {
                width: 320.0,
                height: 240.0,
            })
            .theme(Some(Theme::Light))
            .build(),
    })
    .unwrap();

    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };

    host.apply(Command::SetTitle {
        id,
        title: String::from("Renamed"),
    })
    .unwrap();
    host.apply(Command::SetVisible { id, visible: false })
        .unwrap();
    host.apply(Command::SetInnerSize {
        id,
        size: Size {
            width: 640.0,
            height: 480.0,
        },
    })
    .unwrap();

    let state = host.registry().get(id).unwrap();
    assert_eq!(state.instance.state.title(), "Renamed");
    assert_eq!(state.instance.state.visible(), Some(false));
    assert_eq!(
        state.instance.state.metrics().logical_size,
        Size {
            width: 640.0,
            height: 480.0,
        }
    );
    assert_eq!(
        state.instance.state.metrics().outer_position,
        Some(Point { x: 11.0, y: 22.0 })
    );
    assert!(
        host.events()
            .iter()
            .any(|event| matches!(event, HostEvent::Resized(metrics) if metrics.id == id))
    );
}

#[test]
fn fake_host_exercises_draw_and_close_contract() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::default(),
    })
    .unwrap();
    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };

    host.apply(Command::RequestDraw { id }).unwrap();
    assert_eq!(host.take_ready_draws(Instant::now()), vec![id]);

    host.apply(Command::Destroy { id }).unwrap();
    assert!(!host.registry().contains(id));
    assert!(
        host.events()
            .iter()
            .any(|event| matches!(event, HostEvent::Destroyed(destroyed) if *destroyed == id))
    );
}

#[test]
fn fake_host_reports_unknown_command_target() {
    let mut host = testing::Host::new();
    let id = Id::from_u64(404);

    let error = host
        .apply(Command::RequestDraw { id })
        .expect_err("unknown window should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert_eq!(error.id, Some(id));
}

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

#[test]
fn window_modeling_baseline_fake_host_records_failed_duplicate_command_without_new_window() {
    let mut host = testing::Host::new();

    host.apply(open("main")).unwrap();
    let error = host
        .apply(open("main"))
        .expect_err("duplicate name should fail");

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

    host.apply(Command::SetVisible { id, visible: false })
        .unwrap();
    host.apply(Command::SetTheme {
        id,
        theme: Some(Theme::Dark),
    })
    .unwrap();

    let state = host.registry().get(id).unwrap();
    let state = state.instance.state();
    assert_eq!(state.visible(), Some(false));
    assert_eq!(state.theme(), Some(Theme::Dark));
    assert!(host.events().iter().any(|event| {
        matches!(event, testing::Event::ThemeChanged { id: event_id, theme: Some(Theme::Dark) } if *event_id == id)
    }));
}

#[test]
fn fake_host_deduplicates_cursor_updates() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::default(),
    })
    .unwrap();
    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };

    host.apply(Command::SetCursor {
        id,
        cursor: Cursor::Icon(CursorIcon::Pointer),
    })
    .unwrap();
    host.apply(Command::SetCursor {
        id,
        cursor: Cursor::Icon(CursorIcon::Pointer),
    })
    .unwrap();
    host.apply(Command::SetCursor {
        id,
        cursor: Cursor::Hidden,
    })
    .unwrap();

    assert_eq!(
        host.cursor_updates(),
        &[
            (id, Cursor::Icon(CursorIcon::Pointer)),
            (id, Cursor::Hidden)
        ]
    );
}

#[test]
fn fake_host_records_ime_request_order() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::default(),
    })
    .unwrap();
    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };
    let config = ImeConfig {
        purpose: ImePurpose::Normal,
        hint: ImeHint::None,
        cursor_area: Some(Rect {
            origin: Point { x: 1.0, y: 2.0 },
            size: Size {
                width: 3.0,
                height: 4.0,
            },
        }),
        surrounding_text: None,
    };

    host.apply(Command::SetIme {
        id,
        request: ImeRequest::Disable,
    })
    .unwrap();
    host.apply(Command::SetIme {
        id,
        request: ImeRequest::Enable(config.clone()),
    })
    .unwrap();
    host.apply(Command::SetIme {
        id,
        request: ImeRequest::Restart(config.clone()),
    })
    .unwrap();

    assert_eq!(
        host.ime_requests(),
        &[
            (id, ImeRequest::Disable),
            (id, ImeRequest::Enable(config.clone())),
            (id, ImeRequest::Restart(config)),
        ]
    );
}

#[test]
fn fake_host_emits_lifecycle_events() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::default(),
    })
    .unwrap();
    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };
    host.clear();

    host.suspend(id).unwrap();
    host.resume(id).unwrap();

    assert_eq!(
        host.events(),
        &[HostEvent::Suspended(id), HostEvent::Resumed(id)]
    );
}

#[test]
fn fake_host_forwards_accessibility_events() {
    let mut host = testing::Host::new();
    host.apply(Command::Open {
        request: WindowRequest::default(),
    })
    .unwrap();
    let id = match &host.events()[0] {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected Created event, got {event:?}"),
    };
    host.clear();

    host.accessibility(AccessibilityEvent::InitialTreeRequested(id))
        .unwrap();
    host.accessibility(AccessibilityEvent::ActionRequested(
        AccessibilityActionRequest {
            id,
            action: String::from("press"),
        },
    ))
    .unwrap();
    host.accessibility(AccessibilityEvent::Deactivated(id))
        .unwrap();

    assert_eq!(
        host.events(),
        &[
            HostEvent::Accessibility(AccessibilityEvent::InitialTreeRequested(id)),
            HostEvent::Accessibility(AccessibilityEvent::ActionRequested(
                AccessibilityActionRequest {
                    id,
                    action: String::from("press"),
                }
            )),
            HostEvent::Accessibility(AccessibilityEvent::Deactivated(id)),
        ]
    );
}

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

#[test]
fn accessibility_event_exposes_target_id() {
    let id = Id::from_u64(8);

    assert_eq!(AccessibilityEvent::InitialTreeRequested(id).id(), id);
    assert_eq!(
        AccessibilityEvent::ActionRequested(AccessibilityActionRequest {
            id,
            action: String::from("focus"),
        })
        .id(),
        id
    );
    assert_eq!(AccessibilityEvent::Deactivated(id).id(), id);
}

#[test]
fn dsl_open_builder_lowers_to_request_and_command() {
    let parent = Id::from_u64(11);
    let open = open("inspector")
        .title("Inspector")
        .at(point(12, 24))
        .size(size(420, 640))
        .min(size(320, 240))
        .max(size(1200, 900))
        .fixed()
        .controls(controls().minimize(false).maximize(false))
        .decorations(false)
        .transparent(true)
        .hidden()
        .borderless()
        .level(Level::AlwaysOnTop)
        .theme(Some(Theme::Dark))
        .tool(Some(parent));

    let request = open.request().clone();

    assert_eq!(request.name(), Some("inspector"));
    assert_eq!(request.title(), "Inspector");
    assert_eq!(request.position(), Some(Point { x: 12.0, y: 24.0 }));
    assert_eq!(
        request.inner_size(),
        Some(Size {
            width: 420.0,
            height: 640.0,
        })
    );
    assert_eq!(
        request.min_inner_size(),
        Some(Size {
            width: 320.0,
            height: 240.0,
        })
    );
    assert_eq!(
        request.max_inner_size(),
        Some(Size {
            width: 1200.0,
            height: 900.0,
        })
    );
    assert!(!request.resizable());
    assert_eq!(
        request.controls(),
        Controls {
            close: true,
            minimize: false,
            maximize: false,
        }
    );
    assert!(!request.decorations());
    assert!(request.transparent());
    assert!(!request.visible());
    assert_eq!(request.fullscreen(), Fullscreen::Borderless);
    assert_eq!(request.level(), Level::AlwaysOnTop);
    assert_eq!(request.theme(), Some(Theme::Dark));
    assert_eq!(
        request.role(),
        &Role::Tool {
            parent: Some(parent),
        }
    );

    assert_eq!(Command::from(open), Command::Open { request });
}

#[test]
fn dsl_open_builder_lowers_role_theme_and_control_variants() {
    assert_eq!(Open::unnamed().request().name(), None);
    assert_eq!(
        open("first").name("second").request().name(),
        Some("second")
    );
    assert_eq!(open("root").root().request().role(), &Role::Root);
    assert_eq!(
        open("dialog")
            .dialog(Id::from_u64(4))
            .modal(Modality::App)
            .request()
            .role(),
        &Role::Dialog {
            parent: Id::from_u64(4),
            modality: Modality::App,
        }
    );
    assert_eq!(
        open("popup").popup(Id::from_u64(8)).request().role(),
        &Role::Popup {
            parent: Id::from_u64(8),
        }
    );
    assert_eq!(open("theme").theme(None).request().theme(), None);
    assert_eq!(
        Controls::from(controls().all(false).close(true)),
        Controls {
            close: true,
            minimize: false,
            maximize: false,
        }
    );
}

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
        WindowSnapshot::new(id, "Main", metrics)
            .named("main")
            .with_visible(true),
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
            }
        ]
    );
    assert_eq!(
        window_loop.context().action(),
        &Action::Batch(vec![
            Action::DrawNext(id),
            Action::CloseRequested(id),
            Action::DrawNow(id)
        ])
    );
}

#[test]
fn dsl_target_helpers_lower_to_exact_commands() {
    let mut window_loop = Loop::new(NoopHandler);
    let id = window_loop.registry.reserve_id();
    window_loop.registry.insert(Instance::new(id, state(id)));
    let ime = ImeRequest::Enable(ImeConfig {
        purpose: ImePurpose::Email,
        hint: ImeHint::Spellcheck,
        cursor_area: Some(rect(1, 2, 3, 4)),
        surrounding_text: None,
    });

    {
        let mut cx = window_loop.context();
        cx.window(id)
            .at(point(10, 20))
            .hide()
            .show()
            .resizable(false)
            .controls(controls().all(false))
            .decorations(false)
            .transparent(true)
            .min(Some(size(100, 80)))
            .max(Option::<Size>::None)
            .fullscreen(Fullscreen::Borderless)
            .level(Level::AlwaysOnBottom)
            .theme(None)
            .cursor(Cursor::Hidden)
            .cursor_grab(CursorGrab::Locked)
            .ime(ime.clone())
            .attention();
    }

    assert_eq!(
        window_loop.commands,
        vec![
            Command::SetPosition {
                id,
                position: point(10, 20),
            },
            Command::SetVisible { id, visible: false },
            Command::SetVisible { id, visible: true },
            Command::SetResizable {
                id,
                resizable: false,
            },
            Command::SetControls {
                id,
                controls: Controls {
                    close: false,
                    minimize: false,
                    maximize: false,
                },
            },
            Command::SetDecorations {
                id,
                decorations: false,
            },
            Command::SetTransparent {
                id,
                transparent: true,
            },
            Command::SetMinInnerSize {
                id,
                size: Some(size(100, 80)),
            },
            Command::SetMaxInnerSize { id, size: None },
            Command::SetFullscreen {
                id,
                fullscreen: Fullscreen::Borderless,
            },
            Command::SetLevel {
                id,
                level: Level::AlwaysOnBottom,
            },
            Command::SetTheme { id, theme: None },
            Command::SetCursor {
                id,
                cursor: Cursor::Hidden,
            },
            Command::SetCursorGrab {
                id,
                grab: CursorGrab::Locked,
            },
            Command::SetIme { id, request: ime },
            Command::RequestUserAttention { id },
        ]
    );
}

#[test]
fn dsl_fake_host_accepts_builders_and_dispatches_scoped_events() {
    #[derive(Default)]
    struct Recorder {
        created: Option<Id>,
        resized: Option<Metrics>,
        draws: Vec<Id>,
    }

    impl Handler for Recorder {
        fn ready(&mut self, win: &mut Ready<'_>) -> Result<()> {
            self.created = Some(win.id());
            win.draw();
            Ok(())
        }

        fn resize(&mut self, win: &mut Resize<'_>) -> Result<()> {
            self.resized = Some(win.metrics().clone());
            win.target().title("Resized");
            win.again();
            Ok(())
        }

        fn draw(&mut self, frame: &mut Frame<'_>) -> Result<()> {
            self.draws.push(frame.id());
            frame.exit();
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main").title("Main").size(size(320, 200)))
        .unwrap();
    let id = host.window_id("main").expect("named window");
    let mut handler = Recorder::default();

    let action = host.dispatch_ready(&mut handler, id).unwrap();
    assert_eq!(handler.created, Some(id));
    assert_eq!(action, Effect::Draw(id));

    let metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 800,
            height: 400,
        },
        2.0,
    );
    let action = host.dispatch_resize(&mut handler, metrics.clone()).unwrap();
    assert_eq!(handler.resized, Some(metrics));
    assert_eq!(
        action,
        Effect::Batch(vec![Effect::Draw(id), Effect::Again(id)])
    );
    assert!(host.commands().iter().any(|command| matches!(
        command,
        Command::SetTitle { id: command_id, title }
            if *command_id == id && title == "Resized"
    )));

    let action = host.dispatch_draw(&mut handler, id).unwrap();
    assert_eq!(handler.draws, vec![id]);
    assert_eq!(action, Effect::Exit);
}

#[test]
fn dsl_app_validates_startup_names_and_root_roles() {
    let duplicate_app = app(NoopHandler).open(open("main")).open(open("main"));
    let error = duplicate_app
        .validate_startup()
        .expect_err("duplicate startup names should fail");
    assert_eq!(error.code, ErrorCode::CommandFailed);

    let dialog_app = app(NoopHandler).open(open("dialog").dialog(Id::from_u64(1)));
    let error = dialog_app
        .validate_startup()
        .expect_err("startup parent roles should fail before native creation");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
}

#[test]
fn dsl_app_queues_startup_open_commands() {
    let window_loop = app(NoopHandler)
        .open(open("main").title("Main"))
        .open(open("tools").title("Tools"))
        .into_loop();

    assert!(window_loop.commands.is_empty());
    assert_eq!(window_loop.startup.len(), 2);
    assert!(matches!(
        &window_loop.startup[0],
        Command::Open { request } if request.name() == Some("main")
    ));
    assert!(matches!(
        &window_loop.startup[1],
        Command::Open { request } if request.name() == Some("tools")
    ));
}

#[test]
fn dsl_startup_commands_are_inserted_once_before_first_resume_callback() {
    let window_loop = app(NoopHandler)
        .open(open("main").title("Main"))
        .open(open("tools").title("Tools"))
        .into_loop();
    let mut runner = WinitRunner::from_loop(window_loop);

    runner.stage_startup();
    runner.stage_startup();

    assert!(runner.startup.is_empty());
    assert_eq!(runner.commands.len(), 2);
    assert!(matches!(
        &runner.commands[0],
        Command::Open { request } if request.name() == Some("main")
    ));
    assert!(matches!(
        &runner.commands[1],
        Command::Open { request } if request.name() == Some("tools")
    ));
}

#[test]
fn dsl_fake_host_rejects_duplicate_runtime_window_names() {
    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();

    let error = host
        .apply(open("main"))
        .expect_err("duplicate runtime names should fail");

    assert_eq!(error.code, ErrorCode::CommandFailed);
}

#[test]
fn dsl_lifecycle_dispatch_rolls_back_callback_commands_on_error() {
    struct FailingHandler;

    impl Handler for FailingHandler {
        fn resize(&mut self, win: &mut Resize<'_>) -> Result<()> {
            win.target().title("Should not apply");
            Err(Error::new(ErrorCode::CommandFailed, "intentional failure"))
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    host.clear();

    let error = host
        .dispatch_resize(
            &mut FailingHandler,
            Metrics::from_physical_size(
                id,
                PhysicalSize {
                    width: 320,
                    height: 200,
                },
                1.0,
            ),
        )
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::CommandFailed);
    assert!(host.commands().is_empty());
}

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

#[test]
fn dsl_lifecycle_dispatch_drains_commands_before_returning_action() {
    struct OpenAndExit;

    impl Handler for OpenAndExit {
        fn input(&mut self, input: &mut Input<'_>) -> Result<()> {
            input.context_mut().open(open("child"));
            input.exit();
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    host.clear();

    let action = host
        .dispatch_input(
            &mut OpenAndExit,
            InputEvent::Modifiers {
                id,
                modifiers: ModifierState::default(),
            },
        )
        .unwrap();

    assert_eq!(action, Effect::Exit);
    assert!(matches!(
        host.events().first(),
        Some(HostEvent::Created(state)) if state.name() == Some("child")
    ));
}

#[test]
fn lifecycle_ready_and_resize_preserve_automatic_draw_with_delayed_draws() {
    struct DelayedDraw {
        time: Instant,
    }

    impl Handler for DelayedDraw {
        fn ready(&mut self, win: &mut Ready<'_>) -> Result<()> {
            win.at(self.time);
            Ok(())
        }

        fn resize(&mut self, win: &mut Resize<'_>) -> Result<()> {
            win.at(self.time);
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    let time = Instant::now() + std::time::Duration::from_millis(25);
    let mut handler = DelayedDraw { time };

    assert_eq!(
        host.dispatch_ready(&mut handler, id).unwrap(),
        Effect::Batch(vec![Effect::Draw(id), Effect::At { id, time }])
    );

    assert_eq!(
        host.dispatch_resize(
            &mut handler,
            Metrics::from_physical_size(
                id,
                PhysicalSize {
                    width: 320,
                    height: 200,
                },
                1.0,
            ),
        )
        .unwrap(),
        Effect::Batch(vec![Effect::Draw(id), Effect::At { id, time }])
    );
}

#[test]
fn dsl_frame_exposes_metrics_and_fake_handle_error() {
    struct RendererProbe {
        metrics: Option<Metrics>,
        handle_error: Option<ErrorCode>,
    }

    impl Handler for RendererProbe {
        fn draw(&mut self, frame: &mut Frame<'_>) -> Result<()> {
            self.metrics = Some(frame.metrics().clone());
            self.handle_error = Some(frame.handle().unwrap_err().code);
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main").size(size(320, 180))).unwrap();
    let id = host.window_id("main").unwrap();
    let mut handler = RendererProbe {
        metrics: None,
        handle_error: None,
    };

    let action = host.dispatch_draw(&mut handler, id).unwrap();

    assert_eq!(action, Effect::Wait);
    assert_eq!(
        handler.metrics.unwrap().logical_size,
        Size {
            width: 320.0,
            height: 180.0,
        }
    );
    assert_eq!(handler.handle_error, Some(ErrorCode::HandleUnavailable));
}

#[test]
fn dsl_idle_is_opt_in() {
    struct IdleHandler;

    impl Handler for IdleHandler {
        fn wants_idle(&self) -> bool {
            true
        }

        fn idle(&mut self, cx: &mut Context<'_>) -> Result<()> {
            cx.exit();
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    let mut noop = NoopHandler;
    let mut handler = IdleHandler;

    assert_eq!(host.idle(&mut noop).unwrap(), None);
    assert_eq!(host.idle(&mut handler).unwrap(), Some(Effect::Exit));
}

#[test]
fn lifecycle_startup_open_delivers_ready_and_draw() {
    #[derive(Default)]
    struct Studio {
        ready: Vec<Id>,
        draws: Vec<Id>,
        ready_size: Option<Size>,
    }

    impl Handler for Studio {
        fn ready(&mut self, win: &mut Ready<'_>) -> Result<()> {
            self.ready.push(win.id());
            self.ready_size = Some(win.metrics().logical_size);
            assert_eq!(win.state().name(), Some("main"));
            win.draw();
            Ok(())
        }

        fn draw(&mut self, frame: &mut Frame<'_>) -> Result<()> {
            self.draws.push(frame.id());
            assert_eq!(
                frame.size(),
                Size {
                    width: 640.0,
                    height: 360.0,
                }
            );
            Ok(())
        }
    }

    let app =
        app(Studio::default()).open(open("main").title("Surgeist Studio").size(size(640, 360)));
    let mut window_loop = app.into_loop();
    let mut host = testing::Host::new();

    assert_eq!(window_loop.startup.len(), 1);
    host.apply(window_loop.startup.remove(0)).unwrap();
    let id = host.window_id("main").unwrap();

    let ready = host.dispatch_ready(window_loop.handler_mut(), id).unwrap();
    assert_eq!(ready, Effect::Draw(id));

    let draw = host.dispatch_draw(window_loop.handler_mut(), id).unwrap();
    assert_eq!(draw, Effect::Wait);
    assert_eq!(window_loop.handler().ready, vec![id]);
    assert_eq!(window_loop.handler().draws, vec![id]);
    assert_eq!(
        window_loop.handler().ready_size,
        Some(Size {
            width: 640.0,
            height: 360.0,
        })
    );
}

#[test]
fn lifecycle_scopes_share_common_window_surface() {
    fn assert_scope<'a, T: Scope<'a>>() {}
    assert_scope::<Ready<'static>>();
    assert_scope::<Resize<'static>>();
    assert_scope::<Input<'static>>();
    assert_scope::<Close<'static>>();
    assert_scope::<Frame<'static>>();

    struct Probe {
        observed: Option<(Id, Size, f64, bool, bool, bool, bool)>,
    }

    impl Handler for Probe {
        fn ready(&mut self, win: &mut Ready<'_>) -> Result<()> {
            self.observed = Some((
                win.id(),
                win.size(),
                win.scale(),
                win.is_focused(),
                win.is_visible(),
                win.is_occluded(),
                win.is_resizing(),
            ));
            win.target().title("Scoped").draw();
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("scope").title("Scope")).unwrap();
    let id = match host.events().last().unwrap() {
        HostEvent::Created(state) => state.id(),
        event => panic!("expected created event, got {event:?}"),
    };
    let mut probe = Probe { observed: None };

    assert_eq!(
        host.dispatch_ready(&mut probe, id).unwrap(),
        Effect::Draw(id)
    );
    assert_eq!(
        probe.observed,
        Some((
            id,
            Size {
                width: 800.0,
                height: 600.0,
            },
            1.0,
            false,
            true,
            false,
            false,
        ))
    );
    assert_eq!(
        host.events()
            .iter()
            .find_map(|event| match event {
                HostEvent::Created(state) if state.id() == id => Some(state.name()),
                _ => None,
            })
            .flatten(),
        Some("scope")
    );
    assert_eq!(
        host.commands().last(),
        Some(&Command::SetTitle {
            id,
            title: String::from("Scoped")
        })
    );
}

#[test]
fn lifecycle_resize_input_close_and_closed_are_scoped() {
    #[derive(Default)]
    struct Studio {
        resized: Vec<Size>,
        inputs: usize,
        close_requested: Vec<Id>,
        closed: Vec<Id>,
    }

    impl Handler for Studio {
        fn resize(&mut self, win: &mut Resize<'_>) -> Result<()> {
            self.resized.push(win.size());
            Ok(())
        }

        fn input(&mut self, input: &mut Input<'_>) -> Result<()> {
            assert!(input.key_pressed(keyboard_types::Code::Escape));
            self.inputs += 1;
            input.close().draw();
            Ok(())
        }

        fn close(&mut self, close: &mut Close<'_>) -> Result<()> {
            self.close_requested.push(close.id());
            close.close();
            Ok(())
        }

        fn closed(&mut self, closed: &mut Closed<'_>) -> Result<()> {
            self.closed.push(closed.id());
            assert_eq!(closed.state().name(), Some("main"));
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main").size(size(320, 180))).unwrap();
    let id = host.window_id("main").unwrap();
    let mut handler = Studio::default();
    let resized_metrics = Metrics::from_physical_size(
        id,
        PhysicalSize {
            width: 800,
            height: 400,
        },
        2.0,
    );

    assert_eq!(
        host.dispatch_resize(&mut handler, resized_metrics).unwrap(),
        Effect::Draw(id)
    );

    assert_eq!(
        host.dispatch_input(
            &mut handler,
            InputEvent::Key(KeyEvent {
                id,
                logical_key: keyboard_types::Key::Escape,
                physical_key: keyboard_types::Code::Escape,
                location: keyboard_types::Location::Standard,
                state: KeyState::Pressed,
                repeat: false,
                synthetic: false,
                modifiers: ModifierState::default(),
                timestamp: None,
            }),
        )
        .unwrap(),
        Effect::Batch(vec![Effect::CloseRequested(id), Effect::Draw(id)])
    );

    assert_eq!(host.dispatch_close(&mut handler, id).unwrap(), Effect::Wait);
    assert!(!host.registry().contains(id));
    host.dispatch_closed(&mut handler, id).unwrap();

    assert_eq!(
        handler.resized,
        vec![Size {
            width: 400.0,
            height: 200.0,
        }]
    );
    assert_eq!(handler.inputs, 1);
    assert_eq!(handler.close_requested, vec![id]);
    assert_eq!(handler.closed, vec![id]);
}

#[test]
fn lifecycle_close_cancel_keeps_window_live() {
    #[derive(Default)]
    struct CancelClose {
        close_requested: Vec<Id>,
        closed: Vec<Id>,
    }

    impl Handler for CancelClose {
        fn close(&mut self, close: &mut Close<'_>) -> Result<()> {
            self.close_requested.push(close.id());
            close.cancel();
            Ok(())
        }

        fn closed(&mut self, closed: &mut Closed<'_>) -> Result<()> {
            self.closed.push(closed.id());
            Ok(())
        }
    }

    let mut host = testing::Host::new();
    host.apply(open("main")).unwrap();
    let id = host.window_id("main").unwrap();
    let mut handler = CancelClose::default();

    assert_eq!(host.dispatch_close(&mut handler, id).unwrap(), Effect::Wait);
    assert!(host.registry().contains(id));
    assert_eq!(handler.close_requested, vec![id]);
    assert!(handler.closed.is_empty());
}

#[derive(Default)]
struct NoopHandler;

impl Handler for NoopHandler {}
