use crate::{AppState, SharedState};
use std::{
    mem,
    path::PathBuf,
    ptr,
    sync::{Mutex, OnceLock},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
            DispatchMessageW, GetCursorPos, GetMessageW, HICON, HMENU, MSG, PostQuitMessage,
            RegisterClassW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, TrackPopupMenu,
            TranslateMessage, WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP,
            WNDCLASSW, WS_OVERLAPPED,
        },
    },
};

const WM_TRAY: u32 = WM_APP + 1;
const TRAY_UID: u32 = 1;
const ID_TOGGLE_AUTO: usize = 1001;
const ID_HDR_ON: usize = 1002;
const ID_HDR_OFF: usize = 1003;
const ID_QUIT: usize = 1005;
const ID_TOGGLE_STARTUP: usize = 1006;
const ID_TARGET_ALL: usize = 1100;
const ID_TARGET_BASE: usize = 1200;

static STATE: OnceLock<SharedState> = OnceLock::new();
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static APP_ICON: OnceLock<usize> = OnceLock::new();
static TARGET_MENU_KEYS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

pub fn run(state: SharedState, config_path: PathBuf) -> Result<(), String> {
    let _ = STATE.set(state);
    let _ = CONFIG_PATH.set(config_path);

    unsafe {
        let class_name = wide("HDRNetflixTrayWindow");
        let hinstance = GetModuleHandleW(ptr::null());
        if hinstance.is_null() {
            return Err("GetModuleHandleW failed".to_string());
        }

        let icon = app_icon();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            hIcon: icon,
            lpszClassName: class_name.as_ptr(),
            ..mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 {
            return Err("RegisterClassW failed".to_string());
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("HDR Netflix").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW failed".to_string());
        }

        add_tray_icon(hwnd)?;

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                unsafe {
                    show_menu(hwnd);
                }
            }
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xffff;
            unsafe {
                handle_command(hwnd, id);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                remove_tray_icon(hwnd);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }

    let state = snapshot_state();
    let auto_label = if state.config.auto_enabled {
        "Disable auto HDR"
    } else {
        "Enable auto HDR"
    };
    let netflix_label = if state.netflix_running {
        "Netflix: running"
    } else {
        "Netflix: not running"
    };
    let status_label = match &state.last_error {
        Some(err) => format!("Last error: {err}"),
        None => "Status: OK".to_string(),
    };
    let hdr_label = hdr_status_label(&state.config.selected_targets);
    let startup_label = if crate::startup::is_enabled() {
        "Disable start with Windows"
    } else {
        "Enable start with Windows"
    };
    let target_menu = unsafe { build_target_menu(&state) };

    unsafe {
        append(menu, ID_TOGGLE_AUTO, auto_label, true);
        append(menu, ID_TOGGLE_STARTUP, startup_label, true);
        append_popup(menu, target_menu, "HDR displays");
        AppendMenuW(menu, 0x800, 0, ptr::null());
        append(menu, ID_HDR_ON, "Turn HDR on now", true);
        append(menu, ID_HDR_OFF, "Turn HDR off now", true);
        AppendMenuW(menu, 0x800, 0, ptr::null());
        append(menu, 0, netflix_label, false);
        append(menu, 0, &hdr_label, false);
        append(menu, 0, &status_label, false);
        AppendMenuW(menu, 0x800, 0, ptr::null());
        append(menu, ID_QUIT, "Quit", true);

        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            point.x,
            point.y,
            0,
            hwnd,
            ptr::null(),
        );
        DestroyMenu(menu);
    }
}

unsafe fn handle_command(hwnd: HWND, id: usize) {
    match id {
        ID_TOGGLE_AUTO => with_state(|state| {
            state.config.auto_enabled = !state.config.auto_enabled;
            if let Some(path) = CONFIG_PATH.get() {
                let _ = state.config.save(path);
            }
        }),
        ID_HDR_ON => set_manual_hdr(true),
        ID_HDR_OFF => set_manual_hdr(false),
        ID_TARGET_ALL => update_selected_targets(|targets| targets.clear()),
        ID_TARGET_BASE..=1999 => toggle_target_selection(id - ID_TARGET_BASE),
        ID_TOGGLE_STARTUP => {
            let enabled = crate::startup::is_enabled();
            match crate::startup::set_enabled(!enabled) {
                Ok(()) => with_state(|state| state.last_error = None),
                Err(err) => with_state(|state| state.last_error = Some(err)),
            }
        }
        ID_QUIT => {
            with_state(|state| state.exiting = true);
            set_manual_hdr(false);
            unsafe {
                remove_tray_icon(hwnd);
                PostQuitMessage(0);
            }
        }
        _ => {}
    }
}

unsafe fn build_target_menu(state: &AppState) -> HMENU {
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return menu;
    }

    let selected = &state.config.selected_targets;
    let all_checked = selected.is_empty();
    unsafe {
        append_checked(menu, ID_TARGET_ALL, "All displays", all_checked, true);
        AppendMenuW(menu, 0x800, 0, ptr::null());
    }

    let mut keys = Vec::new();
    match crate::hdr::get_statuses(&[]) {
        Ok(statuses) if statuses.is_empty() => unsafe {
            append(menu, 0, "No active displays", false);
        },
        Ok(statuses) => {
            for (index, status) in statuses.iter().enumerate() {
                let checked = !all_checked && selected.iter().any(|key| key == &status.target.key);
                let suffix = if status.supported {
                    if status.enabled { "HDR on" } else { "HDR off" }
                } else {
                    "HDR unsupported"
                };
                let label = format!("{} ({suffix})", status.target.name);
                unsafe {
                    append_checked(
                        menu,
                        ID_TARGET_BASE + index,
                        &label,
                        checked,
                        status.supported,
                    );
                }
                keys.push(status.target.key.clone());
            }
        }
        Err(err) => unsafe {
            append(menu, 0, &format!("Display error: {err}"), false);
        },
    }

    if let Ok(mut guard) = target_menu_keys().lock() {
        *guard = keys;
    }

    menu
}

fn toggle_target_selection(index: usize) {
    let Some(target_key) = target_menu_keys()
        .lock()
        .ok()
        .and_then(|keys| keys.get(index).cloned())
    else {
        return;
    };

    update_selected_targets(|targets| {
        if targets.is_empty() {
            targets.push(target_key);
            return;
        }

        if let Some(position) = targets.iter().position(|key| key == &target_key) {
            targets.remove(position);
        } else {
            targets.push(target_key);
        }
    });
}

fn update_selected_targets(update: impl FnOnce(&mut Vec<String>)) {
    with_state(|state| {
        update(&mut state.config.selected_targets);
        state.config.selected_targets.sort();
        state.config.selected_targets.dedup();
        if let Some(path) = CONFIG_PATH.get() {
            match state.config.save(path) {
                Ok(()) => state.last_error = None,
                Err(err) => state.last_error = Some(format!("Save config failed: {err}")),
            }
        }
    });
}

fn set_manual_hdr(enable: bool) {
    let selected_targets =
        with_state_return(|state| state.config.selected_targets.clone()).unwrap_or_default();
    match crate::hdr::set_hdr_for_targets(enable, &selected_targets) {
        Ok(()) => with_state(|state| {
            state.hdr_active_by_app = enable;
            state.last_error = None;
        }),
        Err(err) => with_state(|state| state.last_error = Some(err)),
    }
}

fn hdr_status_label(selected_targets: &[String]) -> String {
    match crate::hdr::get_statuses(selected_targets) {
        Ok(statuses) if statuses.is_empty() => "HDR: no active displays".to_string(),
        Ok(statuses) => {
            let supported = statuses.iter().filter(|status| status.supported).count();
            let enabled = statuses
                .iter()
                .filter(|status| status.supported && status.enabled)
                .count();
            format!("HDR: {enabled}/{supported} displays on")
        }
        Err(err) => format!("HDR: {err}"),
    }
}

unsafe fn add_tray_icon(hwnd: HWND) -> Result<(), String> {
    let mut data: NOTIFYICONDATAW = unsafe { mem::zeroed() };
    data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = app_icon();
    copy_tip(&mut data.szTip, "HDR Netflix");

    if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
        Err("Shell_NotifyIconW(NIM_ADD) failed".to_string())
    } else {
        Ok(())
    }
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = unsafe { mem::zeroed() };
    data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

unsafe fn append(menu: HMENU, id: usize, label: &str, enabled: bool) {
    let flags = if enabled { 0x0000 } else { 0x0002 };
    let wide_label = wide(label);
    unsafe {
        AppendMenuW(menu, flags, id, wide_label.as_ptr());
    }
}

unsafe fn append_checked(menu: HMENU, id: usize, label: &str, checked: bool, enabled: bool) {
    let mut flags = 0x0000;
    if checked {
        flags |= 0x0008;
    }
    if !enabled {
        flags |= 0x0002;
    }
    let wide_label = wide(label);
    unsafe {
        AppendMenuW(menu, flags, id, wide_label.as_ptr());
    }
}

unsafe fn append_popup(menu: HMENU, submenu: HMENU, label: &str) {
    let wide_label = wide(label);
    unsafe {
        AppendMenuW(menu, 0x0010, submenu as usize, wide_label.as_ptr());
    }
}

fn snapshot_state() -> AppState {
    with_state_return(|state| state.clone()).unwrap_or_default()
}

fn with_state(action: impl FnOnce(&mut AppState)) {
    if let Some(state) = STATE.get()
        && let Ok(mut guard) = state.lock()
    {
        action(&mut guard);
    }
}

fn with_state_return<T>(action: impl FnOnce(&AppState) -> T) -> Option<T> {
    if let Some(state) = STATE.get()
        && let Ok(guard) = state.lock()
    {
        return Some(action(&guard));
    }
    None
}

fn copy_tip(target: &mut [u16], value: &str) {
    let wide_value = wide(value);
    let len = wide_value.len().saturating_sub(1).min(target.len());
    target[..len].copy_from_slice(&wide_value[..len]);
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn app_icon() -> HICON {
    *APP_ICON.get_or_init(|| crate::icon::create_app_icon() as usize) as HICON
}

fn target_menu_keys() -> &'static Mutex<Vec<String>> {
    TARGET_MENU_KEYS.get_or_init(|| Mutex::new(Vec::new()))
}
