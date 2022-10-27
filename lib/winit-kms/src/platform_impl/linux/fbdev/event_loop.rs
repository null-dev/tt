use parking_lot::Mutex;
use std::{
    cell::RefCell,
    collections::VecDeque,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc::SendError, Arc},
    time::{Duration, Instant},
};
use udev::Enumerator;
use xkbcommon::xkb;

#[cfg(feature = "wayland")]
use sctk::reexports::calloop;

use crate::error;

use crate::{
    dpi::PhysicalPosition,
    event::{DeviceId, Event, KeyboardInput, StartCause, WindowEvent},
    event_loop::{self, ControlFlow, EventLoopClosed},
    monitor::MonitorHandle,
    platform::unix::Card,
    platform_impl::{self, platform::sticky_exit_callback, OsError},
    window::WindowId,
};
use crate::platform_impl::fbdev::FBInfo;

use super::{
    input::{Interface, LibinputInputBackend, REPEAT_RATE},
};

macro_rules! to_platform_impl {
    ($p:ident, $params:expr) => {
        $p(platform_impl::$p::FbDev($params))
    };
}

macro_rules! window_id {
    () => {
        to_platform_impl!(WindowId, super::WindowId)
    };
}

/// An event loop's sink to deliver events from the Wayland event callbacks
/// to the winit's user.
type EventSink = Vec<Event<'static, ()>>;

pub struct EventLoopWindowTarget<T> {
    /// Allows window to edit cursor position
    pub(crate) cursor_arc: Arc<Mutex<PhysicalPosition<f64>>>,

    /// Event loop handle.
    pub event_loop_handle: calloop::LoopHandle<'static, EventSink>,

    pub(crate) event_sink: EventSink,

    /// A proxy to wake up event loop.
    pub event_loop_awakener: calloop::ping::Ping,

    pub(crate) info: FBInfo,

    _marker: std::marker::PhantomData<T>,
}

impl<T> EventLoopWindowTarget<T> {
    #[inline]
    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        Some(MonitorHandle {
            inner: platform_impl::MonitorHandle::FbDev(self.monitor())
        })
    }

    #[inline]
    pub fn available_monitors(&self) -> VecDeque<super::MonitorHandle> {
        let mut result = VecDeque::new();
        result.push_back(self.monitor());
        result
    }

    fn monitor(&self) -> super::MonitorHandle {
        super::MonitorHandle {
            info: self.info.clone()
        }
    }
}

fn find_fb_path() -> std::io::Result<Option<PathBuf>> {
    linuxfb::Framebuffer::list().map(|fbs| fbs.into_iter().next())
}

pub struct EventLoop<T: 'static> {
    /// Event loop.
    event_loop: calloop::EventLoop<'static, EventSink>,

    /// Pending user events.
    pending_user_events: Rc<RefCell<Vec<T>>>,

    /// Sender of user events.
    user_events_sender: calloop::channel::Sender<T>,

    /// Window target.
    window_target: event_loop::EventLoopWindowTarget<T>,
}

impl<T: 'static> EventLoop<T> {
    pub fn new() -> Result<EventLoop<T>, error::OsError> {
        let fb_path = std::env::var("WINIT_FBDEV_PATH")
            .map(|path| PathBuf::from(path))
            .ok()
            .or_else(|| find_fb_path().ok().flatten())
            .ok_or_else(|| os_error!(OsError::FbDevMisc("failed to compile XKB keymap")))?;

        let fb = linuxfb::Framebuffer::new(fb_path)
            .map_err(|e| os_error!(OsError::FbDevError(format!("failed to open fbdev device: {e:?}"))))?;

        // Opening our input manager with no seat means we must do so as root
        // (or be part of the `input` user group)
        let mut input = input::Libinput::new_with_udev(Interface);
        input.udev_assign_seat("seat0").unwrap();

        // XKB allows us to keep track of the state of the keyboard and produce keyboard events
        // very similarly to how a Wayland Compositor would.
        let xkb_ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        // Empty strings translates to "default" for XKB (in this context)
        let keymap = xkb::Keymap::new_from_names(
            &xkb_ctx,
            "",
            "",
            "",
            "",
            std::env::var("WINIT_XKB_OPTIONS").ok(),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| os_error!(OsError::FbDevMisc("failed to compile XKB keymap")))?;

        let state = xkb::State::new(&keymap);

        // TODO(compose) Re-enable
        /*
        // It's not a strict requirement that we use a compose table, but it's ***sooo*** useful
        // when using an english keyboard to write in other languages. Or even just speacial
        // charecters
        //
        // For example, to type è, you would use <Compose> + <e> + <Grave>
        // Or to type •, you would use <Compose> + <.> + <=>
        let compose_table = xkb::compose::Table::new_from_locale(
            &xkb_ctx,
            // These env variables in Linux are the most likely to contain your locale,
            // "en_US.UTF-8" for example
            std::env::var_os("LC_ALL")
                .unwrap_or_else(|| {
                    std::env::var_os("LC_CTYPE").unwrap_or_else(|| {
                        std::env::var_os("LANG").unwrap_or_else(|| std::ffi::OsString::from("C"))
                    })
                })
                .as_os_str(),
            xkb::compose::COMPILE_NO_FLAGS,
        )
        .map_err(|_| {
            // e ^^^ would return ()
            os_error!(OsError::FbDevMisc("failed to compile XKB compose table"))
        })?;
        let xkb_compose = xkb::compose::State::new(&compose_table, xkb::compose::STATE_NO_FLAGS);
         */

        // TODO Fix this (fb.get_size() always returns 32x32???)
        // let (disp_width, disp_height) = fb.get_size();
        let (disp_width, disp_height) = (800, 480);
        let fb_id = fb.get_id();

        let event_loop: calloop::EventLoop<'static, EventSink> =
            calloop::EventLoop::try_new().unwrap();

        let handle = event_loop.handle();

        // A source of user events.
        let pending_user_events = Rc::new(RefCell::new(Vec::new()));
        let pending_user_events_clone = pending_user_events.clone();
        let (user_events_sender, user_events_channel) = calloop::channel::channel();

        // User events channel.
        handle
            .insert_source(user_events_channel, move |event, _, _| {
                if let calloop::channel::Event::Msg(msg) = event {
                    pending_user_events_clone.borrow_mut().push(msg);
                }
            })
            .unwrap();

        // An event's loop awakener to wake up for redraw events from winit's windows.
        let (event_loop_awakener, event_loop_awakener_source) = calloop::ping::make_ping().unwrap();

        let event_sink = EventSink::new();

        // Handler of redraw requests.
        handle
            .insert_source(
                event_loop_awakener_source,
                move |_event, _metadata, data| {
                    data.push(Event::RedrawRequested(window_id!()));
                },
            )
            .unwrap();

        // This is used so that when you hold down a key, the same `KeyboardInput` event will be
        // repeated until the key is released or another key is pressed down
        let repeat_handler = calloop::timer::Timer::new().unwrap();

        let repeat_handle = repeat_handler.handle();

        let repeat_loop: calloop::Dispatcher<
            'static,
            calloop::timer::Timer<(KeyboardInput, Option<char>)>,
            EventSink,
        > = calloop::Dispatcher::new(
            repeat_handler,
            move |event, metadata, data: &mut EventSink| {
                data.push(Event::WindowEvent {
                    window_id: window_id!(),
                    event: WindowEvent::KeyboardInput {
                        device_id: DeviceId(platform_impl::DeviceId::FbDev(super::DeviceId)),
                        input: event.0,
                        is_synthetic: false,
                    },
                });

                if let Some(c) = event.1 {
                    data.push(Event::WindowEvent {
                        window_id: window_id!(),
                        event: WindowEvent::ReceivedCharacter(c),
                    });
                }

                // Repeat the key with the same key event as was input from the LibinputInterface
                metadata.add_timeout(Duration::from_millis(REPEAT_RATE), event);
            },
        );

        // It is an Arc<Mutex<>> so that windows can change the cursor position
        let cursor_arc = Arc::new(Mutex::new(PhysicalPosition::new(0.0, 0.0)));

        // Our input handler
        let input_backend: LibinputInputBackend = LibinputInputBackend::new(
            input,
            (disp_width.into(), disp_height.into()), // plane, fb
            repeat_handle,
            state,
            keymap,
            // TODO(compose) Re-enable
            // xkb_compose,
            cursor_arc.clone(),
        );

        // When an input is received, add it to our EventSink
        let input_loop: calloop::Dispatcher<'static, LibinputInputBackend, EventSink> =
            calloop::Dispatcher::new(
                input_backend,
                move |event, _metadata, data: &mut EventSink| {
                    data.push(event);
                },
            );

        handle.register_dispatcher(input_loop).unwrap();
        handle.register_dispatcher(repeat_loop).unwrap();

        let window_target = event_loop::EventLoopWindowTarget {
            p: platform_impl::EventLoopWindowTarget::FbDev(EventLoopWindowTarget {
                info: FBInfo {
                    size: (disp_width, disp_height),
                    name: fb_id,
                },
                cursor_arc,
                event_loop_handle: handle,
                event_sink,
                event_loop_awakener,
                _marker: PhantomData,
            }),
            _marker: PhantomData,
        };

        Ok(EventLoop {
            event_loop,
            pending_user_events,
            user_events_sender,
            window_target,
        })
    }

    pub fn run<F>(mut self, callback: F) -> !
    where
        F: FnMut(Event<'_, T>, &event_loop::EventLoopWindowTarget<T>, &mut ControlFlow) + 'static,
    {
        let exit_code = self.run_return(callback);
        std::process::exit(exit_code);
    }

    pub fn run_return<F>(&mut self, mut callback: F) -> i32
    where
        F: FnMut(Event<'_, T>, &event_loop::EventLoopWindowTarget<T>, &mut ControlFlow),
    {
        let mut control_flow = ControlFlow::Poll;
        let pending_user_events = self.pending_user_events.clone();
        let mut event_sink_back_buffer = Vec::new();

        callback(
            Event::NewEvents(StartCause::Init),
            &self.window_target,
            &mut control_flow,
        );

        callback(
            Event::RedrawRequested(window_id!()),
            &self.window_target,
            &mut control_flow,
        );

        let exit_code = loop {
            match control_flow {
                ControlFlow::ExitWithCode(code) => break code,
                ControlFlow::Poll => {
                    // Non-blocking dispatch.
                    let timeout = Duration::from_millis(0);
                    if let Err(error) = self.loop_dispatch(Some(timeout)) {
                        break error.raw_os_error().unwrap_or(1);
                    }

                    callback(
                        Event::NewEvents(StartCause::Poll),
                        &self.window_target,
                        &mut control_flow,
                    );
                }
                ControlFlow::Wait => {
                    if let Err(error) = self.loop_dispatch(None) {
                        break error.raw_os_error().unwrap_or(1);
                    }

                    callback(
                        Event::NewEvents(StartCause::WaitCancelled {
                            start: Instant::now(),
                            requested_resume: None,
                        }),
                        &self.window_target,
                        &mut control_flow,
                    );
                }
                ControlFlow::WaitUntil(deadline) => {
                    let start = Instant::now();

                    // Compute the amount of time we'll block for.
                    let duration = if deadline > start {
                        deadline - start
                    } else {
                        Duration::from_millis(0)
                    };

                    if let Err(error) = self.loop_dispatch(Some(duration)) {
                        break error.raw_os_error().unwrap_or(1);
                    }

                    let now = Instant::now();

                    if now < deadline {
                        callback(
                            Event::NewEvents(StartCause::WaitCancelled {
                                start,
                                requested_resume: Some(deadline),
                            }),
                            &self.window_target,
                            &mut control_flow,
                        )
                    } else {
                        callback(
                            Event::NewEvents(StartCause::ResumeTimeReached {
                                start,
                                requested_resume: deadline,
                            }),
                            &self.window_target,
                            &mut control_flow,
                        )
                    }
                }
            }

            // Handle pending user events. We don't need back buffer, since we can't dispatch
            // user events indirectly via callback to the user.
            for user_event in pending_user_events.borrow_mut().drain(..) {
                sticky_exit_callback(
                    Event::UserEvent(user_event),
                    &self.window_target,
                    &mut control_flow,
                    &mut callback,
                );
            }

            // The purpose of the back buffer and that swap is to not hold borrow_mut when
            // we're doing callback to the user, since we can double borrow if the user decides
            // to create a window in one of those callbacks.
            self.with_window_target(|window_target| {
                let state = &mut window_target.event_sink;
                std::mem::swap::<Vec<Event<'static, ()>>>(&mut event_sink_back_buffer, state);
            });

            // Handle pending window events.
            for event in event_sink_back_buffer.drain(..) {
                let event = event.map_nonuser_event().unwrap();
                sticky_exit_callback(event, &self.window_target, &mut control_flow, &mut callback);
            }

            // Send events cleared.
            sticky_exit_callback(
                Event::MainEventsCleared,
                &self.window_target,
                &mut control_flow,
                &mut callback,
            );

            // Send RedrawEventCleared.
            sticky_exit_callback(
                Event::RedrawEventsCleared,
                &self.window_target,
                &mut control_flow,
                &mut callback,
            );
        };

        callback(Event::LoopDestroyed, &self.window_target, &mut control_flow);
        exit_code
    }

    #[inline]
    pub fn create_proxy(&self) -> EventLoopProxy<T> {
        EventLoopProxy::new(self.user_events_sender.clone())
    }

    #[inline]
    pub fn window_target(&self) -> &event_loop::EventLoopWindowTarget<T> {
        &self.window_target
    }

    fn with_window_target<U, F: FnOnce(&mut EventLoopWindowTarget<T>) -> U>(&mut self, f: F) -> U {
        let state = match &mut self.window_target.p {
            platform_impl::EventLoopWindowTarget::FbDev(window_target) => window_target,
            #[cfg(any(feature = "x11", feature = "wayland"))]
            _ => unreachable!(),
        };

        f(state)
    }

    fn loop_dispatch<D: Into<Option<std::time::Duration>>>(
        &mut self,
        timeout: D,
    ) -> std::io::Result<()> {
        let state = match &mut self.window_target.p {
            platform_impl::EventLoopWindowTarget::FbDev(window_target) => {
                &mut window_target.event_sink
            }
            #[cfg(any(feature = "x11", feature = "wayland"))]
            _ => unreachable!(),
        };

        self.event_loop.dispatch(timeout, state)
    }
}

/// A handle that can be sent across the threads and used to wake up the `EventLoop`.
pub struct EventLoopProxy<T: 'static> {
    user_events_sender: calloop::channel::Sender<T>,
}

impl<T: 'static> Clone for EventLoopProxy<T> {
    fn clone(&self) -> Self {
        EventLoopProxy {
            user_events_sender: self.user_events_sender.clone(),
        }
    }
}

impl<T: 'static> EventLoopProxy<T> {
    pub fn new(user_events_sender: calloop::channel::Sender<T>) -> Self {
        Self { user_events_sender }
    }

    pub fn send_event(&self, event: T) -> Result<(), EventLoopClosed<T>> {
        self.user_events_sender
            .send(event)
            .map_err(|SendError(error)| EventLoopClosed(error))
    }
}
