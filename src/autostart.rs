//! Starting the clock when the user logs in.
//!
//! The system holds the setting, not the clock's saved settings, so it is
//! read back from there: a Run entry in the registry on Windows, an XDG
//! autostart entry on Linux, and a launch agent on macOS.

use std::path::PathBuf;

/// The clock's own exe, which the entry starts.
fn exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("Can't find the clock's program file: {e}"))
}

/// If the clock starts at login, points the entry at this copy of it, in
/// case the exe has moved since the entry was made.
pub fn refresh() {
    if enabled() {
        let _ = set(true);
    }
}

#[cfg(windows)]
mod imp {
    use std::ptr::null_mut;

    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
    };

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const NAME: &str = "SplitFlapClock";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    pub fn enabled() -> bool {
        let (run, name) = (wide(RUN), wide(NAME));
        let mut len = 0u32;
        // SAFETY: the strings are NUL-terminated, and with no buffer the
        // call only reports the value's size.
        let r = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                run.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                null_mut(),
                null_mut(),
                &mut len,
            )
        };
        r == ERROR_SUCCESS
    }

    pub fn set(on: bool) -> Result<(), String> {
        let (run, name) = (wide(RUN), wide(NAME));
        let r = if on {
            let value = wide(&format!("\"{}\"", super::exe()?.display()));
            // SAFETY: `value` is a NUL-terminated UTF-16 string of the
            // length given in bytes.
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    run.as_ptr(),
                    name.as_ptr(),
                    REG_SZ,
                    value.as_ptr().cast(),
                    (value.len() * 2) as u32,
                )
            }
        } else {
            // SAFETY: the strings are NUL-terminated.
            match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, run.as_ptr(), name.as_ptr()) } {
                ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                r => r,
            }
        };
        if r == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!(
                "Windows refused to change the startup entry (error {r})"
            ))
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::PathBuf;

    fn entry() -> Option<PathBuf> {
        let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
        Some(
            PathBuf::from(home)
                .join("Library/LaunchAgents")
                .join("com.timstersaudioworks.splitflapclock.plist"),
        )
    }

    pub fn enabled() -> bool {
        entry().is_some_and(|p| p.exists())
    }

    pub fn set(on: bool) -> Result<(), String> {
        let path = entry().ok_or("Can't find the launch agents folder: HOME isn't set")?;
        let fail = |e: std::io::Error| format!("Can't change {}: {e}", path.display());
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(fail(e)),
                _ => Ok(()),
            };
        }
        let exe = super::exe()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(fail)?;
        }
        std::fs::write(&path, agent(&exe.to_string_lossy())).map_err(fail)
    }

    /// A launch agent that starts `exe` when the user logs in.
    pub fn agent(exe: &str) -> String {
        let exe = exe
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\
             \t<string>com.timstersaudioworks.splitflapclock</string>\n\
             \t<key>ProgramArguments</key>\n\
             \t<array>\n\
             \t\t<string>{exe}</string>\n\
             \t</array>\n\
             \t<key>RunAtLoad</key>\n\
             \t<true/>\n\
             </dict>\n\
             </plist>\n"
        )
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn the_agent_names_the_program() {
            let a = super::agent("/Applications/A & B.app/Contents/MacOS/splitflap-clock");
            assert!(a.contains(
                "<string>/Applications/A &amp; B.app/Contents/MacOS/splitflap-clock</string>"
            ));
            assert!(a.contains("<key>RunAtLoad</key>"));
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use std::path::PathBuf;

    fn entry() -> Option<PathBuf> {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(config.join("autostart").join("splitflap-clock.desktop"))
    }

    pub fn enabled() -> bool {
        entry().is_some_and(|p| p.exists())
    }

    pub fn set(on: bool) -> Result<(), String> {
        let path = entry().ok_or("Can't find the autostart folder: HOME isn't set")?;
        let fail = |e: std::io::Error| format!("Can't change {}: {e}", path.display());
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(fail(e)),
                _ => Ok(()),
            };
        }
        let exe = super::exe()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(fail)?;
        }
        std::fs::write(&path, desktop_entry(&exe.to_string_lossy())).map_err(fail)
    }

    pub fn desktop_entry(exe: &str) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Split-Flap Clock\n\
             Comment=A split-flap desktop clock\n\
             Exec={}\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            quote(exe)
        )
    }

    /// `path` as one argument of an Exec line. Quoting escapes `"`, `` ` ``,
    /// `$` and `\` with a backslash, and the string rules then double every
    /// backslash; `%` is doubled for field codes.
    fn quote(path: &str) -> String {
        let mut out = String::from("\"");
        for c in path.chars() {
            match c {
                '"' | '`' | '$' => {
                    out.push_str("\\\\");
                    out.push(c);
                }
                '\\' => out.push_str("\\\\\\\\"),
                '%' => out.push_str("%%"),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn exec_lines_are_quoted() {
            assert_eq!(quote("/opt/My Clock/clock"), "\"/opt/My Clock/clock\"");
            assert_eq!(quote("/a$b%c"), "\"/a\\\\$b%%c\"");
            assert_eq!(quote(r"/a\b"), r#""/a\\\\b""#);
            assert!(desktop_entry("/x").contains("\nExec=\"/x\"\n"));
        }
    }
}

pub use imp::{enabled, set};
