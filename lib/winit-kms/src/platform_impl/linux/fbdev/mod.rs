use crate::dpi::{PhysicalPosition, PhysicalSize};

pub mod event_loop;
pub mod input;
pub mod window;
use crate::{monitor, platform_impl};
pub use event_loop::EventLoop;
pub use event_loop::EventLoopProxy;
pub use event_loop::EventLoopWindowTarget;
use std::os::unix;
use std::os::unix::prelude::FromRawFd;
use std::sync::Arc;
pub use window::Window;

#[derive(Debug, Clone)]
/// A simple wrapper for a device node.
pub struct Card(pub(crate) Arc<i32>);

/// Implementing `AsRawFd` is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling `as_raw_fd()` on the inner File.
impl unix::io::AsRawFd for Card {
    fn as_raw_fd(&self) -> unix::io::RawFd {
        *self.0
    }
}

impl Drop for Card {
    fn drop(&mut self) {
        unsafe { std::fs::File::from_raw_fd(*self.0) };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId;

#[allow(dead_code)]
impl DeviceId {
    pub const unsafe fn dummy() -> Self {
        DeviceId
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FBInfo {
    size: (u32, u32),
    name: String,
}

impl FBInfo {
    fn physical_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.size.0, self.size.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonitorHandle {
    info: FBInfo
}

impl MonitorHandle {
    #[inline]
    pub fn name(&self) -> Option<String> {
        Some(self.info.name.clone())
    }

    #[inline]
    pub fn native_identifier(&self) -> u32 {
        // TODO Maybe parse this from the fbdev path?
        0
    }

    #[inline]
    pub fn size(&self) -> PhysicalSize<u32> {
        self.info.physical_size()
    }

    #[inline]
    pub fn position(&self) -> PhysicalPosition<i32> {
        PhysicalPosition::new(0, 0)
    }

    #[inline]
    pub fn scale_factor(&self) -> f64 {
        1.0
    }

    #[inline]
    pub fn video_modes(&self) -> impl Iterator<Item = monitor::VideoMode> {
        return vec![monitor::VideoMode {
            video_mode: platform_impl::VideoMode::FbDev(VideoMode {
                info: self.info.clone()
            })
        }].into_iter();
    }
}

// TODO Support actually querying for modes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VideoMode {
    info: FBInfo
}

impl VideoMode {
    #[inline]
    pub fn size(&self) -> PhysicalSize<u32> {
        self.info.physical_size()
    }

    #[inline]
    pub fn bit_depth(&self) -> u16 {
        // TODO Calculate
        32
    }

    #[inline]
    pub fn refresh_rate(&self) -> u16 {
        // TODO Calculate from pixclock
        60
    }

    #[inline]
    pub fn monitor(&self) -> monitor::MonitorHandle {
        monitor::MonitorHandle {
            inner: platform_impl::platform::MonitorHandle::FbDev(MonitorHandle {
                info: self.info.clone()
            })
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId;

#[allow(dead_code)]
impl WindowId {
    pub const unsafe fn dummy() -> Self {
        Self
    }
}
