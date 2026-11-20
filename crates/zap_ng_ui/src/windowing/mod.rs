#[cfg(winit)]
pub mod winit;

pub use zap_ng_ui_core::windowing::*;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub use winit::WindowingSystem;
