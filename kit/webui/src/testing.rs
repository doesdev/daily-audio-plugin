//! For tests that open a panel the way a host would: a hidden host
//! window on this thread, and a message pump to keep the webview going.

use crate::Hwnd;
use std::ffi::c_void;
use std::time::{Duration, Instant};

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    w: usize,
    l: isize,
    time: u32,
    x: i32,
    y: i32,
    private: u32,
}

windows_link::link!("user32.dll" "system" fn CreateWindowExW(
    ex_style: u32, class: *const u16, title: *const u16, style: u32,
    x: i32, y: i32, w: i32, h: i32,
    parent: Hwnd, menu: *mut c_void, instance: *mut c_void, param: *const c_void
) -> Hwnd);
windows_link::link!("user32.dll" "system" fn DestroyWindow(hwnd: Hwnd) -> i32);
windows_link::link!("user32.dll" "system" fn PeekMessageW(msg: *mut Msg, hwnd: Hwnd, min: u32, max: u32, remove: u32) -> i32);
windows_link::link!("user32.dll" "system" fn TranslateMessage(msg: *const Msg) -> i32);
windows_link::link!("user32.dll" "system" fn DispatchMessageW(msg: *const Msg) -> isize);
windows_link::link!("ole32.dll" "system" fn OleInitialize(reserved: *mut c_void) -> i32);

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const PM_REMOVE: u32 = 1;

/// A hidden top-level window, as a host's plugin window would be.
pub struct HostWindow(Hwnd);

impl HostWindow {
    pub fn open() -> Self {
        unsafe { OleInitialize(std::ptr::null_mut()) };
        let (class, title): (Vec<u16>, Vec<u16>) = (
            "STATIC".encode_utf16().chain([0]).collect(),
            "host".encode_utf16().chain([0]).collect(),
        );
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                800,
                600,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        assert!(!hwnd.is_null(), "host window");
        Self(hwnd)
    }

    pub fn hwnd(&self) -> Hwnd {
        self.0
    }
}

impl Drop for HostWindow {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0) };
    }
}

/// Runs this thread's messages for `ms`.
pub fn pump(ms: u64) {
    let until = Instant::now() + Duration::from_millis(ms);
    let mut msg: Msg = unsafe { std::mem::zeroed() };
    while Instant::now() < until {
        while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
