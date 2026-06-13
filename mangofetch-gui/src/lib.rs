//! MangoFetch GUI — Interfaz gráfica con egui + eframe

pub mod bridge;
pub mod runtime;
pub mod theme;
pub mod widgets;
pub mod app;

pub use bridge::{CoreEvent, GuiCommand, MediaInfo, QueueItemInfo};
pub use runtime::AppRuntime;
pub use theme::BrandPreset;
pub use app::MangoFetchApp;
