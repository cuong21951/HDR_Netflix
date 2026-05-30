use std::{env, fs, mem, path::PathBuf, ptr};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
        RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
    },
};

const APP_NAME: &str = "HDR Netflix";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn is_enabled() -> bool {
    registry_value_exists().unwrap_or(false) || legacy_startup_shortcut().exists()
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    remove_legacy_startup_shortcut();
    if enabled {
        enable_registry_startup()
    } else {
        disable_registry_startup()
    }
}

fn registry_value_exists() -> Result<bool, String> {
    let mut value_type = 0u32;
    let code = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide(RUN_KEY).as_ptr(),
            wide(APP_NAME).as_ptr(),
            RRF_RT_REG_SZ,
            &mut value_type,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    match code {
        ERROR_SUCCESS => Ok(value_type == REG_SZ),
        ERROR_FILE_NOT_FOUND => Ok(false),
        _ => Err(format!("RegGetValueW failed with code {code}")),
    }
}

fn enable_registry_startup() -> Result<(), String> {
    let exe = env::current_exe().map_err(|err| format!("current_exe failed: {err}"))?;
    let command = format!("\"{}\"", exe.display());
    let value = wide(&command);
    let bytes = (value.len() * mem::size_of::<u16>()) as u32;

    let key = RunKey::open(KEY_SET_VALUE)?;
    let code = unsafe {
        RegSetValueExW(
            key.raw,
            wide(APP_NAME).as_ptr(),
            0,
            REG_SZ,
            value.as_ptr() as *const u8,
            bytes,
        )
    };

    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("RegSetValueExW failed with code {code}"))
    }
}

fn disable_registry_startup() -> Result<(), String> {
    let key = RunKey::open(KEY_SET_VALUE | KEY_QUERY_VALUE)?;
    let code = unsafe { RegDeleteValueW(key.raw, wide(APP_NAME).as_ptr()) };

    match code {
        ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
        _ => Err(format!("RegDeleteValueW failed with code {code}")),
    }
}

fn legacy_startup_shortcut() -> PathBuf {
    let appdata = env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    appdata
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("HDR Netflix.lnk")
}

fn remove_legacy_startup_shortcut() {
    let path = legacy_startup_shortcut();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

struct RunKey {
    raw: HKEY,
}

impl RunKey {
    fn open(access: u32) -> Result<Self, String> {
        let mut key = ptr::null_mut();
        let code = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(RUN_KEY).as_ptr(),
                0,
                access,
                &mut key,
            )
        };

        if code == ERROR_SUCCESS {
            Ok(Self { raw: key })
        } else {
            Err(format!("RegOpenKeyExW failed with code {code}"))
        }
    }
}

impl Drop for RunKey {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.raw);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
