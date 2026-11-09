//! What installing changes in Windows besides the files, the same as
//! packaging/windows/yappuccino.iss did, so either replaces the other.

use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::*;

/// Inno Setup's key for the AppId, so a Rust install takes over an Inno one.
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{19FF8D53-8D7E-4E66-91BB-E2F5BE307E9A}_is1";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
pub const UNINSTALLER: &str = "Uninstall Yappuccino.exe";
const NO_WINDOW: u32 = 0x0800_0000;

/// %LOCALAPPDATA%\Programs\Yappuccino, where Inno's per-user install went.
pub fn default_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Programs").join("Yappuccino")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

fn check(code: u32, what: &str) -> io::Result<()> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::other(format!("{what}: Windows error {code}")))
    }
}

fn set_value(key: &str, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> io::Result<()> {
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        check(
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            ),
            key,
        )?;
        let result = RegSetValueExW(
            handle,
            wide(name).as_ptr(),
            0,
            kind,
            data.as_ptr(),
            data.len() as u32,
        );
        RegCloseKey(handle);
        check(result, name)
    }
}

fn set_string(key: &str, name: &str, value: &str) -> io::Result<()> {
    let data: Vec<u8> = wide(value)
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .collect();
    set_value(key, name, REG_SZ, &data)
}

fn set_dword(key: &str, name: &str, value: u32) -> io::Result<()> {
    set_value(key, name, REG_DWORD, &value.to_le_bytes())
}

// Removing what isn't there is fine, so these ignore the result.
fn delete_value(key: &str, name: &str) {
    unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, wide(key).as_ptr(), wide(name).as_ptr()) };
}

fn delete_tree(key: &str) {
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide(key).as_ptr()) };
}

/// Deletes `key` only while it holds neither values nor subkeys.
fn delete_if_empty(key: &str) {
    unsafe {
        let mut handle: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(key).as_ptr(),
            0,
            KEY_READ,
            &mut handle,
        ) != ERROR_SUCCESS
        {
            return;
        }
        let (mut keys, mut values) = (0u32, 0u32);
        use std::ptr::null_mut as n;
        let read = RegQueryInfoKeyW(
            handle,
            n(),
            n(),
            n(),
            &mut keys,
            n(),
            n(),
            &mut values,
            n(),
            n(),
            n(),
            n(),
        );
        RegCloseKey(handle);
        if read == ERROR_SUCCESS && keys == 0 && values == 0 {
            RegDeleteKeyW(HKEY_CURRENT_USER, wide(key).as_ptr());
        }
    }
}

/// Yappuccino keeps running in the tray with its window closed, so setup
/// ends it rather than asking, as Inno's CloseApplications=force did.
pub fn close_app() {
    let ended = Command::new("taskkill")
        .args(["/F", "/IM", "yappuccino.exe"])
        .creation_flags(NO_WINDOW)
        .status();
    if ended.is_ok_and(|status| status.success()) {
        // Windows lets go of the files a moment after the process ends.
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

/// A PowerShell string, quoted.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// Start menu and, if asked, desktop shortcuts, or `None` to remove both.
/// PowerShell asks Windows where those folders are, since OneDrive may have
/// moved the desktop.
pub fn shortcuts(target: Option<(&Path, bool)>) -> io::Result<()> {
    let folder = |name: &str| {
        format!("(Join-Path ([Environment]::GetFolderPath('{name}')) 'Yappuccino.lnk')")
    };
    let script = match target {
        Some((exe, desktop)) => {
            let make = |name: &str| {
                format!(
                    "$s=(New-Object -ComObject WScript.Shell).CreateShortcut({});$s.TargetPath={};$s.WorkingDirectory={};$s.Save();",
                    folder(name),
                    quoted(&exe.to_string_lossy()),
                    quoted(&exe.parent().unwrap().to_string_lossy())
                )
            };
            let mut script = make("Programs");
            if desktop {
                script += &make("Desktop");
            } else {
                script += &format!(
                    "Remove-Item -LiteralPath {} -ErrorAction SilentlyContinue;",
                    folder("Desktop")
                );
            }
            script
        }
        None => format!(
            "Remove-Item -LiteralPath {},{} -ErrorAction SilentlyContinue",
            folder("Programs"),
            folder("Desktop")
        ),
    };
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .creation_flags(NO_WINDOW)
        .status()?;
    if target.is_some() && !status.success() {
        return Err(io::Error::other("couldn't make the Start menu shortcut"));
    }
    Ok(())
}

/// Registers the install: Apps & features, the yappuccino:// links and,
/// if chosen, starting at sign-in.
pub fn register(dir: &Path, version: &str, size_bytes: u64, autostart: bool) -> io::Result<()> {
    let exe = dir.join("yappuccino.exe");
    let exe = exe.to_string_lossy();
    let dir_text = dir.to_string_lossy();
    let uninstaller = dir.join(UNINSTALLER);
    set_string(UNINSTALL_KEY, "DisplayName", "Yappuccino")?;
    set_string(UNINSTALL_KEY, "DisplayVersion", version)?;
    set_string(UNINSTALL_KEY, "Publisher", "Yappuccino")?;
    set_string(UNINSTALL_KEY, "URLInfoAbout", "https://yapfast.com")?;
    set_string(UNINSTALL_KEY, "DisplayIcon", &exe)?;
    set_string(UNINSTALL_KEY, "InstallLocation", &dir_text)?;
    set_string(
        UNINSTALL_KEY,
        "UninstallString",
        &format!("\"{}\" --uninstall", uninstaller.display()),
    )?;
    set_string(
        UNINSTALL_KEY,
        "QuietUninstallString",
        &format!("\"{}\" --uninstall --quiet", uninstaller.display()),
    )?;
    set_dword(UNINSTALL_KEY, "EstimatedSize", (size_bytes / 1024) as u32)?;
    set_dword(UNINSTALL_KEY, "NoModify", 1)?;
    set_dword(UNINSTALL_KEY, "NoRepair", 1)?;
    // Yappuccino's own links open here; claiming Discord's stays the person's choice.
    set_string(
        r"Software\Classes\yappuccino",
        "",
        "URL:Yappuccino Protocol",
    )?;
    set_string(r"Software\Classes\yappuccino", "URL Protocol", "")?;
    set_string(
        r"Software\Classes\yappuccino\DefaultIcon",
        "",
        &format!("\"{exe}\",0"),
    )?;
    set_string(
        r"Software\Classes\yappuccino\shell\open\command",
        "",
        &format!("\"{exe}\" --open-link \"%1\""),
    )?;
    // The same value Settings > Advanced > "Start when you sign in" writes (src/autostart.rs).
    if autostart {
        set_string(RUN_KEY, "Yappuccino", &format!("\"{exe}\" --background"))?;
        // Chosen here, it starts even if Task Manager had turned it off before.
        delete_value(APPROVED_KEY, "Yappuccino");
    } else {
        delete_value(RUN_KEY, "Yappuccino");
    }
    Ok(())
}

/// Undoes `register`, and what the app itself wrote that uninstalling drops.
pub fn unregister(dir: &Path) {
    // Before the files go: discord:// links open in the app that had them again.
    let exe = dir.join("yappuccino.exe");
    if exe.is_file() {
        let _ = Command::new(&exe)
            .arg("--release-discord-links")
            .creation_flags(NO_WINDOW)
            .status();
    }
    delete_tree(UNINSTALL_KEY);
    delete_tree(r"Software\Classes\yappuccino");
    delete_tree(r"Software\Yappuccino\DiscordProtocol");
    delete_if_empty(r"Software\Yappuccino");
    delete_tree(r"Software\Classes\AppUserModelId\io.github.yappuccino.Yappuccino");
    delete_value(RUN_KEY, "Yappuccino");
    delete_value(APPROVED_KEY, "Yappuccino");
}

/// Removes the install folder. The uninstaller runs from it, so a hidden
/// cmd removes the folder once this process has ended.
pub fn remove_folder(dir: &Path) -> io::Result<()> {
    let me = std::env::current_exe()?;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path == me {
            continue;
        }
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?
        } else {
            std::fs::remove_file(&path)?
        }
    }
    Command::new("cmd")
        .raw_arg(format!(
            "/C ping 127.0.0.1 -n 3 >nul & rd /s /q \"{}\"",
            dir.display()
        ))
        .creation_flags(NO_WINDOW)
        .spawn()?;
    Ok(())
}

pub fn launch(dir: &Path) -> io::Result<()> {
    Command::new(dir.join("yappuccino.exe"))
        .current_dir(dir)
        .spawn()
        .map(drop)
}
