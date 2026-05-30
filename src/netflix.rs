use std::{ffi::OsString, mem, os::windows::ffi::OsStringExt};
use windows_sys::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        System::Threading::GetCurrentProcessId,
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
        },
    },
    core::BOOL,
};

pub fn is_netflix_running(process_names: &[String], detect_window_titles: bool) -> bool {
    process_match(process_names) || (detect_window_titles && window_title_match())
}

pub fn detection_reasons(process_names: &[String], detect_window_titles: bool) -> Vec<String> {
    let mut reasons = process_reasons(process_names);
    if detect_window_titles {
        reasons.extend(window_title_reasons());
    }
    reasons
}

fn process_match(process_names: &[String]) -> bool {
    !process_reasons(process_names).is_empty()
}

fn process_reasons(process_names: &[String]) -> Vec<String> {
    let wanted: Vec<String> = process_names
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect();
    let mut reasons = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() {
            return reasons;
        }

        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        let current_process_id = GetCurrentProcessId();

        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let exe = wide_array_to_string(&entry.szExeFile).to_ascii_lowercase();
                if entry.th32ProcessID != current_process_id && wanted.contains(&exe) {
                    reasons.push(format!("process:{} pid={}", exe, entry.th32ProcessID));
                }

                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    reasons
}

fn window_title_match() -> bool {
    !window_title_reasons().is_empty()
}

fn window_title_reasons() -> Vec<String> {
    let mut reasons = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            &mut reasons as *mut Vec<String> as LPARAM,
        );
    }
    reasons
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }

    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return 1;
    }

    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    if copied <= 0 {
        return 1;
    }

    let title = String::from_utf16_lossy(&buffer[..copied as usize]);
    if is_netflix_window_title(&title) {
        let reasons = lparam as *mut Vec<String>;
        unsafe {
            (*reasons).push(format!("window:{title}"));
        }
    }

    1
}

fn is_netflix_window_title(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    if lower.contains("hdr-netflix") || lower.contains("hdr_netflix") {
        return false;
    }

    let normalized = lower.replace(['_', '.'], " ");
    normalized == "netflix"
        || normalized.starts_with("netflix -")
        || normalized.contains(" - netflix")
}

fn wide_array_to_string(raw: &[u16]) -> String {
    let len = raw.iter().position(|ch| *ch == 0).unwrap_or(raw.len());
    OsString::from_wide(&raw[..len])
        .to_string_lossy()
        .to_string()
}
