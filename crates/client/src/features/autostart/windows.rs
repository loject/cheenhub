//! Windows-реализация автоматического запуска через пользовательский реестр.

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "CheenHub";
const STARTUP_HIDDEN_ARG: &str = "--startup-hidden";

/// Сообщает, доступно ли управление автоматическим запуском.
pub(crate) const fn is_supported() -> bool {
    true
}

/// Проверяет, указывает ли запись автозапуска на текущий исполняемый файл.
pub(crate) fn is_enabled() -> Result<bool, String> {
    let expected = command_for_executable(&std::env::current_exe().map_err(|error| {
        format!("Не удалось определить путь CheenHub для проверки автозапуска: {error}")
    })?);
    Ok(read_run_value()?.is_some_and(|value| value == expected))
}

/// Создаёт или удаляет пользовательскую запись автоматического запуска.
pub(crate) fn set_enabled(enabled: bool) -> Result<(), String> {
    if enabled {
        let executable = std::env::current_exe().map_err(|error| {
            format!("Не удалось определить путь CheenHub для включения автозапуска: {error}")
        })?;
        write_run_value(&command_for_executable(&executable))
    } else {
        delete_run_value()
    }
}

/// Возвращает, передан ли аргумент скрытого запуска вместе с Windows.
pub(crate) fn started_hidden() -> bool {
    std::env::args_os().any(|argument| argument == STARTUP_HIDDEN_ARG)
}

fn command_for_executable(executable: &Path) -> OsString {
    let mut command = OsString::from("\"");
    command.push(executable.as_os_str());
    command.push(format!("\" {STARTUP_HIDDEN_ARG}"));
    command
}

fn read_run_value() -> Result<Option<OsString>, String> {
    let key_path = wide_null(RUN_KEY);
    let mut key = null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    check_status(status, "открыть раздел автозапуска Windows")?;
    let key = RegistryKey(key);

    let value_name = wide_null(VALUE_NAME);
    let mut value_type = 0;
    let mut byte_len = 0;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            null(),
            &mut value_type,
            null_mut(),
            &mut byte_len,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    check_status(status, "прочитать размер записи автозапуска CheenHub")?;
    if value_type != REG_SZ {
        return Ok(None);
    }

    let mut value = vec![0_u16; byte_len.div_ceil(2) as usize];
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            value_name.as_ptr(),
            null(),
            &mut value_type,
            value.as_mut_ptr().cast(),
            &mut byte_len,
        )
    };
    check_status(status, "прочитать запись автозапуска CheenHub")?;
    while value.last() == Some(&0) {
        value.pop();
    }
    Ok(Some(OsString::from_wide(&value)))
}

fn write_run_value(command: &OsStr) -> Result<(), String> {
    let key_path = wide_null(RUN_KEY);
    let mut key = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    check_status(status, "открыть раздел автозапуска Windows для записи")?;
    let key = RegistryKey(key);
    let value_name = wide_null(VALUE_NAME);
    let value = wide_null(command);
    let status = unsafe {
        RegSetValueExW(
            key.0,
            value_name.as_ptr(),
            0,
            REG_SZ,
            value.as_ptr().cast(),
            (value.len() * size_of::<u16>()) as u32,
        )
    };
    check_status(status, "сохранить запись автозапуска CheenHub")
}

fn delete_run_value() -> Result<(), String> {
    let key_path = wide_null(RUN_KEY);
    let mut key = null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check_status(status, "открыть раздел автозапуска Windows для удаления")?;
    let key = RegistryKey(key);
    let value_name = wide_null(VALUE_NAME);
    let status = unsafe { RegDeleteValueW(key.0, value_name.as_ptr()) };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    check_status(status, "удалить запись автозапуска CheenHub")
}

fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

fn check_status(status: WIN32_ERROR, action: &str) -> Result<(), String> {
    if status == ERROR_SUCCESS {
        return Ok(());
    }
    Err(format!(
        "Не удалось {action}: {}",
        std::io::Error::from_raw_os_error(status as i32)
    ))
}

struct RegistryKey(HKEY);

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use super::command_for_executable;

    #[test]
    fn startup_command_quotes_executable_path() {
        assert_eq!(
            command_for_executable(Path::new(r"C:\Program Files\CheenHub\cheen_hub.exe")),
            OsString::from(r#""C:\Program Files\CheenHub\cheen_hub.exe" --startup-hidden"#)
        );
    }
}
