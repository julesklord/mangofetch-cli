//! Bridge entre egui (UI thread) y tokio runtime (async)
//! Define los tipos de mensajes bidireccionales

use std::fmt;

/// Comandos enviados desde el GUI hacia el core
#[derive(Debug, Clone)]
pub enum GuiCommand {
    StartDownload {
        url: String,
        output_dir: String,
        quality: Option<String>,
        video_format: Option<String>,
        audio_format: Option<String>,
        audio_quality: Option<String>,
        audio_only: bool,
    },
    PauseDownload {
        id: u64,
    },
    ResumeDownload {
        id: u64,
    },
    RemoveDownload {
        id: u64,
    },
    RefreshQueue,
    CheckDependencies,
    FetchMediaInfo {
        url: String,
    },
    Shutdown,
}

impl fmt::Display for GuiCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiCommand::StartDownload { url, .. } => write!(f, "StartDownload({})", url),
            GuiCommand::PauseDownload { id } => write!(f, "PauseDownload({})", id),
            GuiCommand::ResumeDownload { id } => write!(f, "ResumeDownload({})", id),
            GuiCommand::RemoveDownload { id } => write!(f, "RemoveDownload({})", id),
            GuiCommand::RefreshQueue => write!(f, "RefreshQueue"),
            GuiCommand::CheckDependencies => write!(f, "CheckDependencies"),
            GuiCommand::FetchMediaInfo { url } => write!(f, "FetchMediaInfo({})", url),
            GuiCommand::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// Eventos emitidos desde el core hacia el GUI
#[derive(Debug, Clone)]
pub enum CoreEvent {
    QueueUpdated(Vec<QueueItemInfo>),
    DownloadProgress {
        id: u64,
        progress: f32,
        speed: f64,
        eta: Option<u64>,
    },
    DownloadComplete {
        id: u64,
        title: String,
    },
    DownloadError {
        id: u64,
        error: String,
    },
    MediaInfoFetched(Result<MediaInfo, String>),
    DependencyStatus {
        ytdlp: bool,
        ffmpeg: bool,
    },
    LogLine(String),
}

/// Información de un item en la cola de descargas
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueItemInfo {
    pub id: u64,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub progress: f32,
    pub speed: f64,
    pub eta: Option<u64>,
}

/// Información de un medio (título, duración, etc)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaInfo {
    pub title: String,
    pub duration: Option<u64>,
    pub platform: String,
    pub available_formats: Vec<String>,
}
