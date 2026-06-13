use std::path::PathBuf;
use tracing_appender::non_blocking;
use tracing_subscriber::{fmt, prelude::*, Registry};

pub fn init_logging(verbose: bool) {
    init_logging_ext(verbose, true);
}

pub fn init_logging_ext(verbose: bool, use_stdout: bool) {
    let log_dir = crate::core::paths::app_data_dir()
        .map(|d| d.join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "mangofetch.log");
    let (non_blocking_appender, _guard) = non_blocking(file_appender);

    // Keep the guard alive as long as the program runs.
    Box::leak(Box::new(_guard));

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_appender);

    if use_stdout {
        let stdout_layer = fmt::layer()
            .with_ansi(true)
            .with_target(verbose)
            .with_filter(if verbose {
                tracing_subscriber::filter::LevelFilter::DEBUG
            } else {
                tracing_subscriber::filter::LevelFilter::WARN
            });

        let subscriber = Registry::default().with(file_layer).with(stdout_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    } else {
        let subscriber = Registry::default().with(file_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }
}

const DEFAULT_MAX_LOG_AGE_DAYS: u64 = 7;
const DEFAULT_MAX_LOG_SIZE_MB: u64 = 100;

pub fn clean_old_logs(max_age_days: u64, max_size_mb: u64) -> std::io::Result<u64> {
    let log_dir = crate::core::paths::app_data_dir()
        .map(|d| d.join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    if !log_dir.exists() {
        return Ok(0);
    }

    let mut removed_count = 0u64;
    let mut total_size: u64 = 0;
    let mut file_times: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    for entry in std::fs::read_dir(&log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
                if let Ok(modified) = metadata.modified() {
                    file_times.push((path.clone(), modified));
                }
            }
        }
    }

    let max_size_bytes = max_size_mb * 1024 * 1024;
    let now = std::time::SystemTime::now();

    for (path, modified) in file_times {
        let age_days = if let Ok(duration) = now.duration_since(modified) {
            duration.as_secs() / 86400
        } else {
            0
        };

        let should_remove = age_days > max_age_days || total_size > max_size_bytes;

        if should_remove && std::fs::remove_file(&path).is_ok() {
            removed_count += 1;
            if let Ok(metadata) = std::fs::metadata(&path) {
                total_size = total_size.saturating_sub(metadata.len());
            }
        }
    }

    Ok(removed_count)
}

pub fn clean_logs() -> std::io::Result<u64> {
    clean_old_logs(DEFAULT_MAX_LOG_AGE_DAYS, DEFAULT_MAX_LOG_SIZE_MB)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process::Command;
    use uuid::Uuid;

    #[test]
    fn test_init_logging_ext_subprocess() {
        if env::var("TEST_INIT_LOGGING_EXT").is_ok() {
            let temp_dir = env::temp_dir().join(Uuid::new_v4().to_string());
            env::set_var("MANGOFETCH_DATA_DIR", temp_dir.to_str().unwrap());

            init_logging_ext(true, true);

            let log_dir = temp_dir.join("logs");
            assert!(log_dir.exists());

            std::process::exit(0);
        }

        let exe = env::current_exe().unwrap();
        let output = Command::new(exe)
            .env("TEST_INIT_LOGGING_EXT", "1")
            .arg("core::logger::tests::test_init_logging_ext_subprocess")
            .arg("--exact")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "init_logging_ext failed: {:?}",
            output
        );
    }

    #[test]
    fn test_init_logging_ext_no_stdout_subprocess() {
        if env::var("TEST_INIT_LOGGING_EXT_NO_STDOUT").is_ok() {
            let temp_dir = env::temp_dir().join(Uuid::new_v4().to_string());
            env::set_var("MANGOFETCH_DATA_DIR", temp_dir.to_str().unwrap());

            init_logging_ext(false, false);

            let log_dir = temp_dir.join("logs");
            assert!(log_dir.exists());

            std::process::exit(0);
        }

        let exe = env::current_exe().unwrap();
        let output = Command::new(exe)
            .env("TEST_INIT_LOGGING_EXT_NO_STDOUT", "1")
            .arg("core::logger::tests::test_init_logging_ext_no_stdout_subprocess")
            .arg("--exact")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "init_logging_ext failed: {:?}",
            output
        );
    }

    #[test]
    fn test_init_logging() {
        if env::var("TEST_INIT_LOGGING").is_ok() {
            let temp_dir = env::temp_dir().join(Uuid::new_v4().to_string());
            env::set_var("MANGOFETCH_DATA_DIR", temp_dir.to_str().unwrap());

            init_logging(false);

            let log_dir = temp_dir.join("logs");
            assert!(log_dir.exists());

            std::process::exit(0);
        }

        let exe = env::current_exe().unwrap();
        let output = Command::new(exe)
            .env("TEST_INIT_LOGGING", "1")
            .arg("core::logger::tests::test_init_logging")
            .arg("--exact")
            .output()
            .unwrap();

        assert!(output.status.success(), "init_logging failed: {:?}", output);
    }
}
