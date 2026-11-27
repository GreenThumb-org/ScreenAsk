use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl MonitorRect {
    pub fn width(&self) -> i32 {
        (self.right - self.left).max(1)
    }

    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FocusedMonitorInfo {
    pub monitor: MonitorRect,
    pub work: MonitorRect,
    pub dpi_scale: f32,
}

impl Default for FocusedMonitorInfo {
    fn default() -> Self {
        Self {
            monitor: MonitorRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            },
            work: MonitorRect {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            },
            dpi_scale: 1.0,
        }
    }
}

pub fn open_folder_in_explorer(path: &Path) {
    let _ = fs::create_dir_all(path);
    let _ = Command::new("explorer").arg(path).spawn();
}

#[cfg(windows)]
pub fn show_and_focus_hwnd(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };
    unsafe {
        let h = hwnd as windows_sys::Win32::Foundation::HWND;
        ShowWindow(h, SW_SHOW);
        ShowWindow(h, SW_RESTORE);
        SetForegroundWindow(h);
    }
}

#[cfg(windows)]
pub fn close_hwnd(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        PostMessageW, ShowWindow, SW_SHOWNOACTIVATE, WM_CLOSE,
    };
    unsafe {
        let h = hwnd as windows_sys::Win32::Foundation::HWND;
        // Wake hidden Win32 window so winit/eframe immediately processes WM_CLOSE.
        ShowWindow(h, SW_SHOWNOACTIVATE);
        PostMessageW(h, WM_CLOSE, 0, 0);
    }
}

/// Gets the current outer window rect `(left, top, right, bottom)` in physical screen pixels.
#[cfg(windows)]
pub fn get_window_outer_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    if hwnd == 0 {
        return None;
    }
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd as windows_sys::Win32::Foundation::HWND, &mut r) != 0 {
            Some((r.left, r.top, r.right, r.bottom))
        } else {
            None
        }
    }
}

/// Gets the non-client frame overhead `(extra_w, extra_h)` in physical pixels for this HWND.
#[cfg(windows)]
pub fn get_window_frame_overhead(hwnd: isize) -> (i32, i32) {
    if hwnd == 0 {
        return (0, 0);
    }
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};
    unsafe {
        let h = hwnd as windows_sys::Win32::Foundation::HWND;
        let mut outer: RECT = std::mem::zeroed();
        let mut inner: RECT = std::mem::zeroed();
        if GetWindowRect(h, &mut outer) != 0 && GetClientRect(h, &mut inner) != 0 {
            let ow = outer.right - outer.left;
            let oh = outer.bottom - outer.top;
            let iw = inner.right - inner.left;
            let ih = inner.bottom - inner.top;
            ((ow - iw).max(0), (oh - ih).max(0))
        } else {
            (0, 0)
        }
    }
}

/// Atomically resizes the window while keeping its top-left corner `(anchor_left, anchor_top)`
/// strictly fixed in physical screen pixels.
#[cfg(windows)]
pub fn resize_hwnd_anchored_top_left(
    hwnd: isize,
    anchor_left: i32,
    anchor_top: i32,
    outer_w: i32,
    outer_h: i32,
) {
    if hwnd == 0 {
        return;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
    };
    unsafe {
        SetWindowPos(
            hwnd as windows_sys::Win32::Foundation::HWND,
            std::ptr::null_mut(),
            anchor_left,
            anchor_top,
            outer_w,
            outer_h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

#[cfg(not(windows))]
pub fn show_and_focus_hwnd(_hwnd: isize) {}

#[cfg(not(windows))]
pub fn close_hwnd(_hwnd: isize) {}

#[cfg(not(windows))]
pub fn get_window_outer_rect(_hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    None
}

#[cfg(not(windows))]
pub fn get_window_frame_overhead(_hwnd: isize) -> (i32, i32) {
    (0, 0)
}

#[cfg(not(windows))]
pub fn resize_hwnd_anchored_top_left(
    _hwnd: isize,
    _anchor_left: i32,
    _anchor_top: i32,
    _outer_w: i32,
    _outer_h: i32,
) {
}

#[cfg(windows)]
pub struct SingleInstanceGuard {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl SingleInstanceGuard {
    pub fn acquire() -> Option<Self> {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let name: Vec<u16> = "Local\\ScreenAskSingleInstanceMutex\0"
            .encode_utf16()
            .collect();
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if handle.is_null() {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
pub fn focused_monitor_info() -> FocusedMonitorInfo {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO,
        MONITOR_DEFAULTTOPRIMARY, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

    unsafe {
        let mut hmon = {
            let fg = GetForegroundWindow();
            if !fg.is_null() {
                MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST)
            } else {
                std::ptr::null_mut()
            }
        };

        if hmon.is_null() {
            let mut pt = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                hmon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
            }
        }

        if hmon.is_null() {
            hmon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        }

        if hmon.is_null() {
            return FocusedMonitorInfo::default();
        }

        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) == 0 {
            return FocusedMonitorInfo::default();
        }

        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let dpi_scale = if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0
            && dpi_x > 0
        {
            dpi_x as f32 / 96.0
        } else {
            1.0
        };

        FocusedMonitorInfo {
            monitor: MonitorRect {
                left: mi.rcMonitor.left,
                top: mi.rcMonitor.top,
                right: mi.rcMonitor.right,
                bottom: mi.rcMonitor.bottom,
            },
            work: MonitorRect {
                left: mi.rcWork.left,
                top: mi.rcWork.top,
                right: mi.rcWork.right,
                bottom: mi.rcWork.bottom,
            },
            dpi_scale,
        }
    }
}

#[cfg(not(windows))]
pub fn focused_monitor_info() -> FocusedMonitorInfo {
    FocusedMonitorInfo::default()
}
