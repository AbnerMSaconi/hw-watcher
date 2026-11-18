// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! X11 window hints (`_NET_WM_WINDOW_TYPE_UTILITY`, `_NET_WM_STATE_SKIP_TASKBAR`,
//! and compositor blur `_KDE_NET_WM_BLUR_BEHIND_REGION`) for the floating panel.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use slint::winit_030::winit::event::WindowEvent;
use slint::winit_030::winit::event_loop::EventLoop;
use slint::winit_030::winit::platform::x11::{
    EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType,
};
use slint::winit_030::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::window::WindowLevel;
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, PhysicalPosition};
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{
    AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, PropMode,
};
use x11rb::wrapper::ConnectionExt as _;

use crate::app::Panel;

const NET_WM_STATE_REMOVE: u32 = 0;
const NET_WM_STATE_ADD: u32 = 1;
const SOURCE_APPLICATION: u32 = 1;

/// Returns `true` when X11 should be forced: an X server is reachable and the user did not opt in
/// to the native Wayland backend with `HW_WATCHER_WAYLAND=1`.
fn prefer_x11() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty())
        && std::env::var_os("HW_WATCHER_WAYLAND").is_none_or(|v| v != "1")
}

/// Selects the winit backend and configures `Utility` window type when `skip_taskbar` is `true`.
pub fn configure_backend(skip_taskbar: bool) {
    let mut selector = slint::BackendSelector::new().backend_name("winit".into());
    if prefer_x11() {
        // Wayland sessions with XWayland keep every X11 hint working (blur, skip-taskbar, position).
        let mut builder = EventLoop::with_user_event();
        builder.with_x11();
        selector = selector.with_winit_event_loop_builder(builder);
    }
    let result = selector
        .with_winit_window_attributes_hook(move |attrs| {
            if skip_taskbar {
                attrs.with_x11_window_type(vec![WindowType::Utility])
            } else {
                attrs
            }
        })
        .select();
    if let Err(e) = result {
        tracing::warn!("could not configure winit backend hook: {e}");
    }
}

/// Watches `winit` window events to apply taskbar and blur hints once the X11 window maps or resizes.
pub fn install_window_hooks(
    panel: &Panel,
    top_flag: Arc<AtomicBool>,
    skip_flag: Arc<AtomicBool>,
    blur_flag: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
) {
    let weak = panel.as_weak();
    panel.window().on_winit_window_event(move |_win, ev| {
        if let Some(p) = weak.upgrade()
            && p.window().has_winit_window()
        {
            match ev {
                WindowEvent::RedrawRequested if pending.swap(false, Ordering::AcqRel) => {
                    apply_always_on_top(&p, top_flag.load(Ordering::Relaxed));
                    apply_skip_taskbar(&p, skip_flag.load(Ordering::Relaxed));
                    apply_blur_behind(&p, blur_flag.load(Ordering::Relaxed));
                }
                WindowEvent::Resized(_) if blur_flag.load(Ordering::Relaxed) => {
                    apply_blur_behind(&p, true);
                }
                _ => {}
            }
        }
        EventResult::Propagate
    });
}

/// Applies the always-on-top window level (`_NET_WM_STATE_ABOVE` on X11) to the mapped window.
///
/// The level set while the window is being created is not honored by every window manager, so it is
/// asserted again once the window is mapped. Without this the setting reads as on but has no effect
/// until it is toggled by hand.
pub fn apply_always_on_top(panel: &Panel, on: bool) {
    let level = if on {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    };
    panel
        .window()
        .with_winit_window(|w| w.set_window_level(level));
}

/// Applies `_NET_WM_STATE_SKIP_TASKBAR` and `_NET_WM_STATE_SKIP_PAGER` on X11 when available.
pub fn apply_skip_taskbar(panel: &Panel, skip: bool) {
    let Some(win) = panel.window().with_winit_window(x11_window_id).flatten() else {
        return;
    };
    if let Err(e) = send_net_wm_state(win, skip) {
        tracing::debug!("failed to update X11 skip_taskbar state: {e}");
    }
}

/// Requests native compositor background blur via `_KDE_NET_WM_BLUR_BEHIND_REGION` on X11.
pub fn apply_blur_behind(panel: &Panel, enabled: bool) {
    let Some((win, width, height)) = panel
        .window()
        .with_winit_window(|w| {
            let id = x11_window_id(w)?;
            let sz = w.inner_size();
            Some((id, sz.width.max(1), sz.height.max(1)))
        })
        .flatten()
    else {
        return;
    };
    if let Err(e) = send_blur_hint(win, enabled, width, height) {
        tracing::debug!("failed to update X11 blur hint: {e}");
    }
}

/// Nudges the window by one pixel and back so the compositor re-evaluates its blur region.
///
/// Moving the window is what makes `KWin` drop a stale blur after the hint is removed; the
/// software renderer alone only repaints dirty rectangles and would leave it in place.
pub fn refresh_compositor(panel: &Panel) {
    let pos = panel.window().position();
    panel
        .window()
        .set_position(PhysicalPosition::new(pos.x + 1, pos.y));
    panel.window().set_position(pos);
    panel.window().request_redraw();
}

/// Returns `true` when the panel runs as a native Wayland window (no global coordinates).
pub fn is_wayland(panel: &Panel) -> bool {
    panel
        .window()
        .with_winit_window(|w| {
            w.window_handle()
                .is_ok_and(|h| matches!(h.as_raw(), RawWindowHandle::Wayland(_)))
        })
        .unwrap_or(false)
}

/// Starts a compositor-driven window move (the only way to move a Wayland toplevel).
pub fn start_wayland_drag(panel: &Panel) {
    panel.window().with_winit_window(|w| {
        let _ = w.drag_window();
    });
}

fn x11_window_id(window: &slint::winit_030::winit::window::Window) -> Option<u32> {
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Xlib(h) => u32::try_from(h.window).ok().filter(|&id| id != 0),
        RawWindowHandle::Xcb(h) => Some(h.window.get()),
        _ => None,
    }
}

fn send_net_wm_state(win: u32, skip: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;

    let wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let skip_taskbar = conn
        .intern_atom(false, b"_NET_WM_STATE_SKIP_TASKBAR")?
        .reply()?
        .atom;
    let skip_pager = conn
        .intern_atom(false, b"_NET_WM_STATE_SKIP_PAGER")?
        .reply()?
        .atom;

    if skip {
        conn.change_property32(
            PropMode::APPEND,
            win,
            wm_state,
            AtomEnum::ATOM,
            &[skip_taskbar, skip_pager],
        )?
        .check()?;
    }

    let action = if skip {
        NET_WM_STATE_ADD
    } else {
        NET_WM_STATE_REMOVE
    };
    let ev = ClientMessageEvent::new(
        32,
        win,
        wm_state,
        [action, skip_taskbar, skip_pager, SOURCE_APPLICATION, 0],
    );
    let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
    conn.send_event(false, root, mask, ev)?.check()?;
    conn.flush()?;
    tracing::info!(win, skip, skip_taskbar, skip_pager, "sent _NET_WM_STATE");
    Ok(())
}

fn send_blur_hint(
    win: u32,
    enabled: bool,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, _) = x11rb::connect(None)?;
    let blur_atom = conn
        .intern_atom(false, b"_KDE_NET_WM_BLUR_BEHIND_REGION")?
        .reply()?
        .atom;
    if enabled {
        conn.change_property32(
            PropMode::REPLACE,
            win,
            blur_atom,
            AtomEnum::CARDINAL,
            &[0, 0, width, height],
        )?
        .check()?;
    } else {
        conn.delete_property(win, blur_atom)?.check()?;
    }
    conn.flush()?;
    tracing::info!(
        win,
        enabled,
        width,
        height,
        "updated _KDE_NET_WM_BLUR_BEHIND_REGION"
    );
    Ok(())
}
