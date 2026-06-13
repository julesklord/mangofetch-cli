fn enhanced_path() -> Option<&'static str> {
    use std::sync::OnceLock;
    static CACHED: OnceLock<Option<String>> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            let bin_dir = crate::core::paths::app_data_dir()?.join("bin");
            let sep = if cfg!(windows) { ";" } else { ":" };
            let current = std::env::var("PATH").unwrap_or_default();

            #[allow(unused_mut)]
            let mut extra_dirs: Vec<String> = vec![bin_dir.display().to_string()];

            #[cfg(target_os = "macos")]
            {
                extra_dirs.push("/opt/homebrew/bin".into());
                extra_dirs.push("/usr/local/bin".into());
            }

            #[cfg(target_os = "linux")]
            {
                if let Some(home) = dirs::home_dir() {
                    extra_dirs.push(home.join(".local").join("bin").display().to_string());
                }
                extra_dirs.push("/usr/local/bin".into());
            }

            Some(format!("{}{}{}", extra_dirs.join(sep), sep, current))
        })
        .as_deref()
}

pub fn command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    if let Some(path) = enhanced_path() {
        cmd.env("PATH", path);
    }
    cmd.env_remove("PYTHONHOME");
    cmd.env_remove("PYTHONPATH");
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
    cmd.stdin(std::process::Stdio::null());
    cmd
}

pub fn std_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    if let Some(path) = enhanced_path() {
        cmd.env("PATH", path);
    }
    cmd.env_remove("PYTHONHOME");
    cmd.env_remove("PYTHONPATH");
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
    cmd.stdin(std::process::Stdio::null());
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn test_tokio_command_construction() {
        let cmd = command("test_prog");
        let std_cmd = cmd.as_std();

        assert_eq!(std_cmd.get_program(), OsStr::new("test_prog"));

        let mut has_python_io = false;
        let mut has_python_utf8 = false;
        let mut removed_pythonhome = false;
        let mut removed_pythonpath = false;

        for (k, v) in std_cmd.get_envs() {
            if k == OsStr::new("PYTHONIOENCODING") {
                assert_eq!(v, Some(OsStr::new("utf-8")));
                has_python_io = true;
            }
            if k == OsStr::new("PYTHONUTF8") {
                assert_eq!(v, Some(OsStr::new("1")));
                has_python_utf8 = true;
            }
            if k == OsStr::new("PYTHONHOME") {
                assert_eq!(v, None);
                removed_pythonhome = true;
            }
            if k == OsStr::new("PYTHONPATH") {
                assert_eq!(v, None);
                removed_pythonpath = true;
            }
        }

        assert!(has_python_io, "PYTHONIOENCODING was not set");
        assert!(has_python_utf8, "PYTHONUTF8 was not set");
        assert!(removed_pythonhome, "PYTHONHOME was not removed");
        assert!(removed_pythonpath, "PYTHONPATH was not removed");
    }

    #[test]
    fn test_std_command_construction() {
        let std_cmd = std_command("test_prog");

        assert_eq!(std_cmd.get_program(), OsStr::new("test_prog"));

        let mut has_python_io = false;
        let mut has_python_utf8 = false;
        let mut removed_pythonhome = false;
        let mut removed_pythonpath = false;

        for (k, v) in std_cmd.get_envs() {
            if k == OsStr::new("PYTHONIOENCODING") {
                assert_eq!(v, Some(OsStr::new("utf-8")));
                has_python_io = true;
            }
            if k == OsStr::new("PYTHONUTF8") {
                assert_eq!(v, Some(OsStr::new("1")));
                has_python_utf8 = true;
            }
            if k == OsStr::new("PYTHONHOME") {
                assert_eq!(v, None);
                removed_pythonhome = true;
            }
            if k == OsStr::new("PYTHONPATH") {
                assert_eq!(v, None);
                removed_pythonpath = true;
            }
        }

        assert!(has_python_io, "PYTHONIOENCODING was not set");
        assert!(has_python_utf8, "PYTHONUTF8 was not set");
        assert!(removed_pythonhome, "PYTHONHOME was not removed");
        assert!(removed_pythonpath, "PYTHONPATH was not removed");
    }
}
