//! Showing the clock on every virtual desktop.
//!
//! Neither egui nor winit can do this, so it is done to the native window:
//! on X11 the window is made sticky, which every window manager that has
//! workspaces honours (XWayland included). Windows shows tool windows on
//! every virtual desktop; a tool window has no taskbar button. On macOS it
//! is the Dock's to choose: Options, Assign To, All Desktops.

#[cfg(not(target_os = "macos"))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(not(target_os = "macos"))]
fn handle(frame: &eframe::Frame) -> Result<RawWindowHandle, String> {
    frame
        .window_handle()
        .map(|h| h.as_raw())
        .map_err(|e| format!("No window to change: {e}"))
}

/// Puts the window on all desktops, or back on the current one. `changed`
/// says whether the setting has just changed; on Windows the style is kept
/// up every frame as well, as winit rewrites it on its own changes.
#[cfg(windows)]
pub fn apply(frame: &eframe::Frame, on: bool, changed: bool) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNOACTIVATE,
        WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
    };
    if !on && !changed {
        return Ok(());
    }
    let RawWindowHandle::Win32(h) = handle(frame)? else {
        return Err("Not a Windows window".into());
    };
    let hwnd = h.hwnd.get() as *mut std::ffi::c_void;
    let (tool, app) = (WS_EX_TOOLWINDOW as isize, WS_EX_APPWINDOW as isize);
    // SAFETY: `hwnd` is this app's own live window.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let want = if on {
            (ex | tool) & !app
        } else {
            (ex & !tool) | app
        };
        if want != ex {
            // The taskbar only notices the change while the window is hidden.
            ShowWindow(hwnd, SW_HIDE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn apply(_frame: &eframe::Frame, on: bool, changed: bool) -> Result<(), String> {
    if changed && on {
        return Err(
            "On a Mac, right-click the clock in the Dock: Options, Assign To, All Desktops".into(),
        );
    }
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn apply(frame: &eframe::Frame, on: bool, changed: bool) -> Result<(), String> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ClientMessageEvent, ConnectionExt, EventMask};

    if !changed {
        return Ok(());
    }
    let window = match handle(frame)? {
        RawWindowHandle::Xlib(h) => h.window as u32,
        RawWindowHandle::Xcb(h) => h.window.get(),
        RawWindowHandle::Wayland(_) => {
            return Err("Wayland doesn't let apps choose this; use the window menu".into())
        }
        _ => return Err("Not an X11 window".into()),
    };
    let fail = |e: &dyn std::fmt::Display| format!("X11: {e}");
    let (conn, screen) = x11rb::connect(None).map_err(|e| fail(&e))?;
    let root = conn.setup().roots[screen].root;
    let atom = |name: &[u8]| -> Result<u32, String> {
        let reply = conn.intern_atom(false, name).map_err(|e| fail(&e))?;
        Ok(reply.reply().map_err(|e| fail(&e))?.atom)
    };
    let (wm_desktop, wm_state, sticky) = (
        atom(b"_NET_WM_DESKTOP")?,
        atom(b"_NET_WM_STATE")?,
        atom(b"_NET_WM_STATE_STICKY")?,
    );
    // 0xFFFFFFFF is every desktop; going back, the clock stays where the
    // user is now.
    let desktop = if on {
        u32::MAX
    } else {
        let current = atom(b"_NET_CURRENT_DESKTOP")?;
        conn.get_property(false, root, current, AtomEnum::CARDINAL, 0, 1)
            .ok()
            .and_then(|c| c.reply().ok())
            .and_then(|r| r.value32().and_then(|mut v| v.next()))
            .unwrap_or(0)
    };
    // Messages to the window manager, as the EWMH spec has them: the last
    // field of each says they come from an ordinary application.
    let messages = [
        (wm_state, [u32::from(on), sticky, 0, 1, 0]),
        (wm_desktop, [desktop, 1, 0, 0, 0]),
    ];
    for (kind, data) in messages {
        let event = ClientMessageEvent::new(32, window, kind, data);
        conn.send_event(
            false,
            root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .map_err(|e| fail(&e))?;
    }
    conn.flush().map_err(|e| fail(&e))?;
    Ok(())
}
