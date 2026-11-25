//! The editor's own child window inside the one the host hands over.
//! It owns the UI timer and its tick callback, so there is no global
//! registry: the window keeps a pointer to its callback and frees it
//! when it is destroyed.

use std::ffi::c_void;
use std::sync::OnceLock;

pub type Hwnd = *mut c_void;
type Wndproc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[repr(C)]
struct WndClassW {
    style: u32,
    wndproc: Option<Wndproc>,
    cls_extra: i32,
    wnd_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu_name: *const u16,
    class_name: *const u16,
}

windows_link::link!("user32.dll" "system" fn RegisterClassW(class: *const WndClassW) -> u16);
windows_link::link!("user32.dll" "system" fn UnregisterClassW(name: *const u16, instance: *mut c_void) -> i32);
windows_link::link!("user32.dll" "system" fn CreateWindowExW(
    ex_style: u32, class: *const u16, title: *const u16, style: u32,
    x: i32, y: i32, w: i32, h: i32,
    parent: Hwnd, menu: *mut c_void, instance: *mut c_void, param: *const c_void
) -> Hwnd);
windows_link::link!("user32.dll" "system" fn DestroyWindow(hwnd: Hwnd) -> i32);
windows_link::link!("user32.dll" "system" fn DefWindowProcW(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize);
windows_link::link!("user32.dll" "system" fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize);
windows_link::link!("user32.dll" "system" fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize);
windows_link::link!("user32.dll" "system" fn SetTimer(hwnd: Hwnd, id: usize, ms: u32, func: *const c_void) -> usize);
windows_link::link!("user32.dll" "system" fn KillTimer(hwnd: Hwnd, id: usize) -> i32);
windows_link::link!("user32.dll" "system" fn SetWindowPos(
    hwnd: Hwnd, after: Hwnd, x: i32, y: i32, w: i32, h: i32, flags: u32
) -> i32);
windows_link::link!("user32.dll" "system" fn SetFocus(hwnd: Hwnd) -> Hwnd);
windows_link::link!("kernel32.dll" "system" fn GetModuleHandleExW(
    flags: u32, name: *const u16, module: *mut *mut c_void
) -> i32);

const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_CLIPCHILDREN: u32 = 0x0200_0000;
const GWLP_USERDATA: i32 = -21;
const WM_TIMER: u32 = 0x0113;
const WM_NCDESTROY: u32 = 0x0082;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;
const FROM_ADDRESS: u32 = 0x4;
const UNCHANGED_REFCOUNT: u32 = 0x2;
const TIMER_ID: usize = 1;

type Tick = Box<dyn Fn()>;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn class_name() -> &'static [u16] {
    static NAME: OnceLock<Vec<u16>> = OnceLock::new();
    NAME.get_or_init(|| wide("kit-editor"))
}

// This DLL's module handle: classes must belong to it, not the host.
fn module() -> *mut c_void {
    let anchor = module as fn() -> *mut c_void as *const u16;
    let mut handle = std::ptr::null_mut();
    unsafe { GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT, anchor, &mut handle) };
    handle
}

unsafe extern "system" fn wndproc(hwnd: Hwnd, msg: u32, w: usize, l: isize) -> isize {
    let tick = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut Tick;

    match msg {
        WM_TIMER if !tick.is_null() => {
            unsafe { (*tick)() };
            0
        }
        WM_NCDESTROY => {
            if !tick.is_null() {
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
                drop(unsafe { Box::from_raw(tick) });
            }
            unsafe { DefWindowProcW(hwnd, msg, w, l) }
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}

/// Registers the window class once per load of this DLL. A second
/// register fails harmlessly with "already exists".
fn register() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        let class = WndClassW {
            style: 0,
            wndproc: Some(wndproc),
            cls_extra: 0,
            wnd_extra: 0,
            instance: module(),
            icon: std::ptr::null_mut(),
            cursor: std::ptr::null_mut(),
            background: std::ptr::null_mut(),
            menu_name: std::ptr::null(),
            class_name: class_name().as_ptr(),
        };
        unsafe { RegisterClassW(&class) };
    });
}

/// Classes a DLL registers outlive it unless removed; call on unload.
pub fn unregister() {
    unsafe { UnregisterClassW(class_name().as_ptr(), module()) };
}

/// A child window of `parent` that calls `tick` every `ms`.
pub fn create(parent: Hwnd, width: i32, height: i32, ms: u32, tick: Tick) -> Option<Hwnd> {
    register();
    let style = WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN;
    let title = wide("");
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name().as_ptr(),
            title.as_ptr(),
            style,
            0,
            0,
            width,
            height,
            parent,
            std::ptr::null_mut(),
            module(),
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return None;
    }

    let tick = Box::into_raw(Box::new(tick));
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, tick as isize);
        SetTimer(hwnd, TIMER_ID, ms, std::ptr::null());
    }
    Some(hwnd)
}

pub fn resize(hwnd: Hwnd, width: i32, height: i32) {
    let flags = SWP_NOZORDER | SWP_NOMOVE | SWP_NOACTIVATE;
    unsafe { SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, width, height, flags) };
}

/// Gives keyboard focus to `hwnd`, a window of this thread.
pub fn focus(hwnd: Hwnd) {
    unsafe { SetFocus(hwnd) };
}

/// Stops the timer and destroys the window, which frees its tick.
pub fn destroy(hwnd: Hwnd) {
    unsafe {
        KillTimer(hwnd, TIMER_ID);
        DestroyWindow(hwnd);
    }
}
