use thiserror::Error;

#[derive(Error, Debug)]
pub enum MangoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Dependency missing: {0}")]
    DependencyMissing(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Authentication required: {0}")]
    AuthRequired(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Content restricted: {0}")]
    Restricted(String),

    #[error("Content not found: {0}")]
    NotFound(String),

    #[error("FFmpeg error: {0}")]
    FFmpeg(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("{0}")]
    Custom(String),
}

pub type MangoResult<T> = Result<T, MangoError>;

pub fn classify_download_error(error: &str) -> (&str, &str) {
    let lower = error.to_lowercase();

    if lower.contains("cookie")
        || lower.contains("login")
        || lower.contains("sign in")
        || lower.contains("authentication")
        || lower.contains("403")
    {
        return ("auth_required", "This content requires login. Install the browser extension and visit the site while logged in.");
    }

    if lower.contains("captcha")
        || lower.contains("blocking")
        || lower.contains("rate limit")
        || lower.contains("429")
        || lower.contains("too many")
    {
        return (
            "rate_limited",
            "Too many requests. Try again in a few minutes.",
        );
    }

    if lower.contains("private") || lower.contains("restricted") || lower.contains("age") {
        return ("restricted", "This content is private or age-restricted.");
    }

    if lower.contains("downloaded file") && lower.contains("not found") {
        return (
            "file_missing",
            "Downloaded file could not be located in the output folder.",
        );
    }

    if lower.contains("not found")
        || lower.contains("404")
        || lower.contains("unavailable")
        || lower.contains("deleted")
    {
        return ("not_found", "Content not found or has been deleted.");
    }

    if lower.contains("ffmpeg") || lower.contains("mux") || lower.contains("merge") {
        return (
            "ffmpeg_needed",
            "FFmpeg is required for this download. Install it from Settings.",
        );
    }

    if lower.contains("yt-dlp") || lower.contains("ytdlp") || lower.contains("no downloader") {
        return (
            "ytdlp_needed",
            "yt-dlp is required. Install it from Settings.",
        );
    }

    if lower.contains("nsig") || lower.contains("signature") || lower.contains("cipher") {
        return (
            "ytdlp_outdated",
            "yt-dlp needs updating. Restart the app to auto-update.",
        );
    }

    ("unknown", error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_download_error_auth_required() {
        let (code, msg) = classify_download_error("Sign in to confirm you're not a bot");
        assert_eq!(code, "auth_required");
        assert!(msg.contains("requires login"));

        let (code, _) = classify_download_error("ERROR: 403 Forbidden");
        assert_eq!(code, "auth_required");
    }

    #[test]
    fn test_classify_download_error_rate_limited() {
        let (code, msg) = classify_download_error("HTTP Error 429: Too Many Requests");
        assert_eq!(code, "rate_limited");
        assert!(msg.contains("Too many requests"));

        let (code, _) = classify_download_error("captcha challenge failed");
        assert_eq!(code, "rate_limited");
    }

    #[test]
    fn test_classify_download_error_restricted() {
        let (code, msg) = classify_download_error("Video is private");
        assert_eq!(code, "restricted");
        assert!(msg.contains("private or age-restricted"));

        let (code, _) = classify_download_error("age-restricted content");
        assert_eq!(code, "restricted");
    }

    #[test]
    fn test_classify_download_error_file_missing() {
        let (code, msg) = classify_download_error("Downloaded file not found");
        assert_eq!(code, "file_missing");
        assert!(msg.contains("could not be located"));
    }

    #[test]
    fn test_classify_download_error_not_found() {
        let (code, msg) = classify_download_error("Video not found");
        assert_eq!(code, "not_found");
        assert!(msg.contains("Content not found"));

        let (code, _) = classify_download_error("ERROR: 404 Not Found");
        assert_eq!(code, "not_found");

        let (code, _) = classify_download_error("Video unavailable");
        assert_eq!(code, "not_found");
    }

    #[test]
    fn test_classify_download_error_ffmpeg_needed() {
        let (code, msg) = classify_download_error("ffmpeg is not installed");
        assert_eq!(code, "ffmpeg_needed");
        assert!(msg.contains("FFmpeg is required"));

        let (code, _) = classify_download_error("Failed to merge formats");
        assert_eq!(code, "ffmpeg_needed");
    }

    #[test]
    fn test_classify_download_error_ytdlp_needed() {
        let (code, msg) = classify_download_error("yt-dlp missing");
        assert_eq!(code, "ytdlp_needed");
        assert!(msg.contains("yt-dlp is required"));
    }

    #[test]
    fn test_classify_download_error_ytdlp_outdated() {
        let (code, msg) = classify_download_error("Cannot extract nsig");
        assert_eq!(code, "ytdlp_outdated");
        assert!(msg.contains("needs updating"));

        let (code, _) = classify_download_error("Unable to extract signature");
        assert_eq!(code, "ytdlp_outdated");
    }

    #[test]
    fn test_classify_download_error_unknown() {
        let original_error = "Something completely different went wrong";
        let (code, msg) = classify_download_error(original_error);
        assert_eq!(code, "unknown");
        assert_eq!(msg, original_error);
    }
}
