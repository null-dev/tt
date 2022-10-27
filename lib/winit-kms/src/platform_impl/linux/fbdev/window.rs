use std::{collections::VecDeque, os::unix::prelude::AsRawFd, sync::Arc};

use parking_lot::Mutex;

#[cfg(feature = "wayland")]
use sctk::reexports::calloop;

use crate::error;

use crate::{
    dpi::{PhysicalPosition, PhysicalSize, Position, Size},
    error::{ExternalError, NotSupportedError},
    monitor::{MonitorHandle, VideoMode},
    platform::unix::Card,
    platform_impl,
    window::{CursorIcon, Fullscreen, WindowAttributes},
};
use crate::platform_impl::fbdev::FBInfo;

pub struct Window {
    ping: calloop::ping::Ping,
    cursor: Arc<Mutex<PhysicalPosition<f64>>>,
    info: FBInfo,
}

impl Window {
    pub fn new<T>(
        event_loop_window_target: &super::event_loop::EventLoopWindowTarget<T>,
        _attributes: WindowAttributes,
        _platform_attributes: platform_impl::PlatformSpecificWindowBuilderAttributes,
    ) -> Result<Self, error::OsError> {
        Ok(Self {
            cursor: event_loop_window_target.cursor_arc.clone(),
            ping: event_loop_window_target.event_loop_awakener.clone(),
            info: event_loop_window_target.info.clone(),
        })
    }
    #[inline]
    pub fn id(&self) -> super::WindowId {
        super::WindowId
    }

    #[inline]
    pub fn set_title(&self, _title: &str) {}

    #[inline]
    pub fn set_visible(&self, _visible: bool) {}

    #[inline]
    pub fn is_visible(&self) -> Option<bool> {
        Some(true)
    }

    #[inline]
    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
        Ok(PhysicalPosition::new(0, 0))
    }

    #[inline]
    pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
        Ok(PhysicalPosition::new(0, 0))
    }

    #[inline]
    pub fn set_outer_position(&self, _position: Position) {}

    #[inline]
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        self.info.physical_size()
    }

    #[inline]
    pub fn outer_size(&self) -> PhysicalSize<u32> {
        self.inner_size()
    }

    #[inline]
    pub fn set_inner_size(&self, _size: Size) {}

    #[inline]
    pub fn set_min_inner_size(&self, _dimensions: Option<Size>) {}

    #[inline]
    pub fn set_max_inner_size(&self, _dimensions: Option<Size>) {}

    #[inline]
    pub fn set_resizable(&self, _resizable: bool) {}

    #[inline]
    pub fn is_resizable(&self) -> bool {
        false
    }

    #[inline]
    pub fn set_cursor_icon(&self, _cursor: CursorIcon) {}

    #[inline]
    pub fn set_cursor_grab(&self, _grab: bool) -> Result<(), ExternalError> {
        Ok(())
    }

    #[inline]
    pub fn set_cursor_visible(&self, _visible: bool) {}

    #[inline]
    pub fn drag_window(&self) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn set_cursor_hittest(&self, _hittest: bool) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn scale_factor(&self) -> f64 {
        1.0
    }

    #[inline]
    pub fn set_cursor_position(&self, position: Position) -> Result<(), ExternalError> {
        *self.cursor.lock() = position.to_physical(1.0);
        Ok(())
    }

    #[inline]
    pub fn set_maximized(&self, _maximized: bool) {}

    #[inline]
    pub fn is_maximized(&self) -> bool {
        true
    }

    #[inline]
    pub fn set_minimized(&self, _minimized: bool) {}

    #[inline]
    pub fn fullscreen(&self) -> Option<Fullscreen> {
        Some(Fullscreen::Exclusive(VideoMode {
            video_mode: platform_impl::VideoMode::FbDev(super::VideoMode {
                info: self.info.clone()
            }),
        }))
    }

    #[inline]
    pub fn set_fullscreen(&self, monitor: Option<Fullscreen>) {
        // TODO What to do here? FB is fullscreen by default!
    }

    #[inline]
    pub fn set_decorations(&self, _decorations: bool) {}

    pub fn is_decorated(&self) -> bool {
        false
    }

    #[inline]
    pub fn set_ime_position(&self, _position: Position) {}

    #[inline]
    pub fn set_ime_allowed(&self, _allowed: bool) {}

    #[inline]
    pub fn request_redraw(&self) {
        self.ping.ping();
    }

    #[inline]
    pub fn current_monitor(&self) -> Option<super::MonitorHandle> {
        Some(super::MonitorHandle {
            info: self.info.clone()
        })
    }

    #[inline]
    pub fn available_monitors(&self) -> VecDeque<super::MonitorHandle> {
        let mut result = VecDeque::new();
        result.push_back(super::MonitorHandle {
            info: self.info.clone()
        });
        result
    }

    #[inline]
    pub fn raw_window_handle(&self) -> raw_window_handle::DrmHandle {
        let mut rwh = raw_window_handle::DrmHandle::empty();
        // TODO
        // rwh.fd = self.card.as_raw_fd();
        // rwh.plane = self.plane.into();
        rwh
    }

    #[inline]
    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        self.current_monitor().map(|m| MonitorHandle {
            inner: platform_impl::MonitorHandle::FbDev(m)
        })
    }
}
