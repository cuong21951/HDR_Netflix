use std::{collections::HashSet, mem, ptr};
use windows_sys::Win32::{
    Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
        DisplayConfigSetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
        QueryDisplayConfig,
    },
    Foundation::LUID,
};

const ERROR_SUCCESS: u32 = 0;

#[derive(Clone)]
pub struct DisplayTarget {
    pub key: String,
    pub name: String,
    adapter_id: LUID,
    target_id: u32,
}

#[derive(Clone)]
pub struct AdvancedColorStatus {
    pub target: DisplayTarget,
    pub supported: bool,
    pub enabled: bool,
    pub force_disabled: bool,
}

#[repr(C)]
struct DeviceInfoHeader {
    r#type: i32,
    size: u32,
    adapter_id: LUID,
    id: u32,
}

#[repr(C)]
struct GetAdvancedColorInfo {
    header: DeviceInfoHeader,
    value: u32,
    color_encoding: u32,
    bits_per_color_channel: u32,
}

#[repr(C)]
struct SetAdvancedColorState {
    header: DeviceInfoHeader,
    value: u32,
}

pub fn list_targets() -> Result<Vec<DisplayTarget>, String> {
    unsafe {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        check_win32_u32(
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count),
            "GetDisplayConfigBufferSizes",
        )?;

        let mut paths = zeroed_vec::<DISPLAYCONFIG_PATH_INFO>(path_count as usize);
        let mut modes = zeroed_vec::<DISPLAYCONFIG_MODE_INFO>(mode_count as usize);

        check_win32_u32(
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                ptr::null_mut(),
            ),
            "QueryDisplayConfig",
        )?;

        let mut seen = HashSet::new();
        let mut targets = Vec::new();
        for path in paths.into_iter().take(path_count as usize) {
            let target = path.targetInfo;
            let key = target_key(target.adapterId, target.id);
            if seen.insert(key.clone()) {
                let name = get_target_name(target.adapterId, target.id)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("Display {}", targets.len() + 1));
                targets.push(DisplayTarget {
                    key,
                    name,
                    adapter_id: target.adapterId,
                    target_id: target.id,
                });
            }
        }

        Ok(targets)
    }
}

pub fn get_statuses(selected_targets: &[String]) -> Result<Vec<AdvancedColorStatus>, String> {
    let selected = selected_set(selected_targets);
    let mut statuses = Vec::new();
    for target in list_targets()? {
        if !selected.is_empty() && !selected.contains(&target.key) {
            continue;
        }
        statuses.push(get_status(target)?);
    }
    Ok(statuses)
}

pub fn set_hdr_for_targets(enable: bool, selected_targets: &[String]) -> Result<(), String> {
    let selected = selected_set(selected_targets);
    let mut errors = Vec::new();

    for target in list_targets()? {
        if !selected.is_empty() && !selected.contains(&target.key) {
            continue;
        }

        match get_status(target.clone()) {
            Ok(status) if !status.supported => continue,
            Ok(_) => {
                if let Err(err) = set_hdr(&target, enable) {
                    errors.push(format!("{}: {}", target.key, err));
                }
            }
            Err(err) => errors.push(format!("{}: {}", target.key, err)),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn get_status(target: DisplayTarget) -> Result<AdvancedColorStatus, String> {
    let mut info = GetAdvancedColorInfo {
        header: DeviceInfoHeader {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
            size: mem::size_of::<GetAdvancedColorInfo>() as u32,
            adapter_id: target.adapter_id,
            id: target.target_id,
        },
        value: 0,
        color_encoding: 0,
        bits_per_color_channel: 0,
    };

    unsafe {
        check_win32_i32(
            DisplayConfigGetDeviceInfo(
                &mut info.header as *mut DeviceInfoHeader as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER,
            ),
            "DisplayConfigGetDeviceInfo(GET_ADVANCED_COLOR_INFO)",
        )?;
    }

    Ok(AdvancedColorStatus {
        target,
        supported: (info.value & 0x1) != 0,
        enabled: (info.value & 0x2) != 0,
        force_disabled: (info.value & 0x8) != 0,
    })
}

fn get_target_name(adapter_id: LUID, target_id: u32) -> Option<String> {
    let mut name = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size: mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
            adapterId: adapter_id,
            id: target_id,
        },
        ..Default::default()
    };

    let code = unsafe { DisplayConfigGetDeviceInfo(&mut name.header) };
    if code != 0 {
        return None;
    }

    Some(wide_array_to_string(&name.monitorFriendlyDeviceName))
}

fn set_hdr(target: &DisplayTarget, enable: bool) -> Result<(), String> {
    let mut state = SetAdvancedColorState {
        header: DeviceInfoHeader {
            r#type: DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
            size: mem::size_of::<SetAdvancedColorState>() as u32,
            adapter_id: target.adapter_id,
            id: target.target_id,
        },
        value: u32::from(enable),
    };

    unsafe {
        check_win32_i32(
            DisplayConfigSetDeviceInfo(
                &mut state.header as *mut DeviceInfoHeader
                    as *const DISPLAYCONFIG_DEVICE_INFO_HEADER,
            ),
            "DisplayConfigSetDeviceInfo(SET_ADVANCED_COLOR_STATE)",
        )
    }
}

fn target_key(adapter_id: LUID, target_id: u32) -> String {
    format!(
        "{}:{}:{}",
        adapter_id.HighPart, adapter_id.LowPart, target_id
    )
}

fn selected_set(selected_targets: &[String]) -> HashSet<String> {
    selected_targets
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn wide_array_to_string(raw: &[u16]) -> String {
    let len = raw.iter().position(|ch| *ch == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..len])
}

fn check_win32_u32(code: u32, name: &str) -> Result<(), String> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("{name} failed with code {code}"))
    }
}

fn check_win32_i32(code: i32, name: &str) -> Result<(), String> {
    check_win32_u32(code as u32, name)
}

unsafe fn zeroed_vec<T>(len: usize) -> Vec<T> {
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        values.push(unsafe { mem::zeroed() });
    }
    values
}
