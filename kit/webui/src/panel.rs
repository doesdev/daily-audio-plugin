//! The page in a webview, in a child window of a given parent. A 30 Hz
//! tick on the parent's UI thread sends the page settings and meters
//! when they change; the page's actions go to whoever opened it.

use crate::bridge::{self, Action};
use crate::window::{self, Hwnd};
use kit::Spec;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::num::NonZeroIsize;
use std::path::PathBuf;
use std::rc::Rc;
use wry::dpi::{PhysicalPosition, PhysicalSize};
use wry::http::{Response, StatusCode, header};
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use wry::{Rect, WebContext, WebView, WebViewBuilder};

/// Settings, then meters, as normalized values; the owner keeps them
/// current and the panel shows them.
pub type Values = Rc<[Cell<f64>]>;
/// Where the page's actions go. `Ready` and `Blur` are the panel's own
/// business.
pub type OnAction = Rc<dyn Fn(Action)>;
/// Called before each tick reads the values, to bring them up to date.
pub type Pull = Box<dyn Fn()>;

// The page, served from the binary over its own scheme so its ES
// modules can import one another.
const SCHEME: &str = "kit";
const START: &str = "kit://localhost/index.html";
const TICK_MS: u32 = 33;

// What the IPC handler and the tick share.
struct Link {
    spec: Spec,
    // The window the host gave us, which gets the keyboard back.
    parent: Hwnd,
    values: Values,
    on_action: OnAction,
    pull: Pull,
    view: RefCell<Option<WebView>>,
    want_hello: Cell<bool>,
    sent: RefCell<Vec<f64>>,
}

impl Link {
    fn act(&self, action: Action) {
        match action {
            Action::Ready => self.want_hello.set(true),
            Action::Blur => window::focus(self.parent),
            other => (self.on_action)(other),
        }
    }

    fn send(&self, msg: &str) {
        if let Some(view) = self.view.borrow().as_ref() {
            let _ = view.evaluate_script(&format!("window.kit && window.kit.receive({msg})"));
        }
    }

    fn tick(&self) {
        (self.pull)();
        let now: Vec<f64> = self.values.iter().map(Cell::get).collect();
        let count = self.spec.params.len().min(now.len());
        let mut sent = self.sent.borrow_mut();

        if self.want_hello.replace(false) {
            self.send(&bridge::hello(&self.spec, &now));
        } else {
            if now[..count] != sent[..count] {
                self.send(&bridge::state(&self.spec, &now[..count]));
            }
            if now[count..] != sent[count..] {
                self.send(&bridge::meters(&self.spec, &now[count..]));
            }
        }
        *sent = now;
    }
}

/// The asset at `path`, or 404.
fn serve(spec: &Spec, path: &str) -> Response<Cow<'static, [u8]>> {
    let (status, kind, body) = match bridge::asset(spec, path) {
        Some((kind, Cow::Borrowed(body))) => (StatusCode::OK, kind, body),
        _ => (StatusCode::NOT_FOUND, "text/plain", "not found"),
    };

    // Never cached: a new build of the plugin must show its own page,
    // not one the webview kept from the last.
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, kind)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Cow::Borrowed(body.as_bytes()))
        .unwrap_or_default()
}

struct Parent(Hwnd);

impl HasWindowHandle for Parent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let hwnd = NonZeroIsize::new(self.0 as isize).ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(hwnd));
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

// WebView2 keeps a profile; by default beside the host's exe, which
// may sit in Program Files and not be writable. One per host program:
// a second process on the same profile fails with ERROR_INVALID_STATE
// unless its options match exactly, which two hosts cannot promise.
fn data_dir(spec: &Spec) -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map_or_else(std::env::temp_dir, PathBuf::from);
    let host = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "host".into());
    base.join(spec.info.vendor).join("webview2").join(host)
}

fn bounds((w, h): (i32, i32)) -> Rect {
    Rect {
        position: PhysicalPosition::new(0, 0).into(),
        size: PhysicalSize::new(w.max(0) as u32, h.max(0) as u32).into(),
    }
}

/// The page, open in a child window of some parent. Closing it is
/// dropping it.
pub struct Panel {
    link: Rc<Link>,
    hwnd: Hwnd,
    // Kept alive as long as the webview.
    _context: WebContext,
}

impl Panel {
    pub fn open(
        spec: Spec,
        parent: Hwnd,
        size: (i32, i32),
        values: Values,
        on_action: OnAction,
        pull: Pull,
    ) -> Option<Self> {
        let link = Rc::new(Link {
            spec,
            parent,
            sent: RefCell::new(vec![f64::NAN; values.len()]),
            values,
            on_action,
            pull,
            view: RefCell::new(None),
            want_hello: Cell::new(false),
        });

        // Each tick holds its own count on the link: a host closing the
        // editor during a tick frees this closure while it runs.
        let ticker = link.clone();
        let hwnd = window::create(
            parent,
            size.0,
            size.1,
            TICK_MS,
            Box::new(move || Rc::clone(&ticker).tick()),
        )?;
        let ipc = Rc::downgrade(&link);
        let mut context = WebContext::new(Some(data_dir(&spec)));
        let built = WebViewBuilder::new_with_web_context(&mut context)
            .with_custom_protocol(SCHEME.into(), move |_, req| serve(&spec, req.uri().path()))
            .with_url(START)
            .with_bounds(bounds(size))
            .with_ipc_handler(move |req| {
                let Some(link) = ipc.upgrade() else {
                    return;
                };
                if let Some(action) = bridge::parse(&link.spec, req.body()) {
                    link.act(action);
                }
            })
            .build_as_child(&Parent(hwnd));
        let Ok(view) = built else {
            window::destroy(hwnd);
            return None;
        };

        *link.view.borrow_mut() = Some(view);
        Some(Self {
            link,
            hwnd,
            _context: context,
        })
    }

    pub fn resize(&self, size: (i32, i32)) {
        window::resize(self.hwnd, size.0, size.1);
        if let Some(view) = self.link.view.borrow().as_ref() {
            let _ = view.set_bounds(bounds(size));
        }
    }

    /// Runs `js` in the page; `done` gets the result as JSON. For tests
    /// and tools: the page itself talks through the bridge.
    pub fn eval(&self, js: &str, done: impl Fn(String) + Send + 'static) -> bool {
        self.link
            .view
            .borrow()
            .as_ref()
            .is_some_and(|view| view.evaluate_script_with_callback(js, done).is_ok())
    }
}

impl Drop for Panel {
    // The webview goes first, then its window; the context after both.
    fn drop(&mut self) {
        self.link.view.borrow_mut().take();
        window::destroy(self.hwnd);
    }
}
