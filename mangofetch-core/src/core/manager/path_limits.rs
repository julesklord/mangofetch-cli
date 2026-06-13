#[cfg(target_os = "windows")]
pub const MAX_PATH_LEN: usize = 259;
#[cfg(not(target_os = "windows"))]
pub const MAX_PATH_LEN: usize = 4095;

pub const MIN_FILENAME_RESERVE: usize = 80;

pub const SEPARATOR_RESERVE: usize = 1;

#[derive(Debug, Clone, Copy)]
pub struct PathLimitError {
    pub limit: usize,
    pub current: usize,
    pub reserve: usize,
}

impl std::fmt::Display for PathLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "output path too long for OS limit (path uses {} of {} chars, need {} reserved for filename)",
            self.current, self.limit, self.reserve
        )
    }
}

impl std::error::Error for PathLimitError {}

pub fn validate_output_dir(output_dir: &str) -> Result<(), PathLimitError> {
    let current = output_dir.chars().count() + SEPARATOR_RESERVE;
    let reserve = MIN_FILENAME_RESERVE;
    if current + reserve > MAX_PATH_LEN {
        return Err(PathLimitError {
            limit: MAX_PATH_LEN,
            current,
            reserve,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_output_dir_short_path() {
        assert!(validate_output_dir("C:\\short\\path").is_ok());
    }

    #[test]
    fn test_validate_output_dir_exact_limit() {
        let exact_len = MAX_PATH_LEN - MIN_FILENAME_RESERVE - SEPARATOR_RESERVE;
        let path = "a".repeat(exact_len);
        assert!(validate_output_dir(&path).is_ok());
    }

    #[test]
    fn test_validate_output_dir_over_limit() {
        let over_len = MAX_PATH_LEN - MIN_FILENAME_RESERVE - SEPARATOR_RESERVE + 1;
        let path = "a".repeat(over_len);
        let result = validate_output_dir(&path);

        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.limit, MAX_PATH_LEN);
            assert_eq!(e.current, over_len + SEPARATOR_RESERVE);
            assert_eq!(e.reserve, MIN_FILENAME_RESERVE);
        }
    }
}
