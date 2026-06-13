//! MangoFetchApp core implementation conforming to the MonolithUI design system.

use crate::bridge::{CoreEvent, GuiCommand, MediaInfo, QueueItemInfo};
use crate::runtime::AppRuntime;
use crate::theme::BrandPreset;
use crate::widgets::{brand_pill, section_header, status_dot, sunken_well, surface_card};
use egui::{
    Align, Button, Color32, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, ProgressBar,
    RichText, ScrollArea, Stroke, Ui, Vec2,
};
use egui_extras::{Column, TableBuilder};

/// Active tabs in the orbital navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Queue,
    Settings,
    Logs,
    About,
}

pub struct MangoFetchApp {
    runtime: AppRuntime,
    current_tab: Tab,
    theme: BrandPreset,

    // Core states
    items: Vec<QueueItemInfo>,
    logs: Vec<String>,
    ytdlp_installed: bool,
    ffmpeg_installed: bool,

    // Inputs & Forms
    input_url: String,
    output_dir: String,
    audio_only: bool,
    selected_quality: String,
    selected_video_format: String,
    selected_audio_format: String,
    selected_audio_quality: String,

    // Media Pre-fetch
    media_info_loading: bool,
    media_info: Option<MediaInfo>,
    media_info_error: Option<String>,

    // Settings parameters
    concurrent_limit: usize,
    auto_retry: bool,
    show_persistent_logs: bool,

    // Telemetry
    sys: sysinfo::System,
    last_sys_refresh: std::time::Instant,

    // Layout
    top_nav_layout: bool,
}

impl MangoFetchApp {
    pub fn new(runtime: AppRuntime) -> Self {
        let default_output_dir = dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "C:\\Downloads".to_string());

        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        let app = Self {
            runtime,
            current_tab: Tab::Home,
            theme: BrandPreset::PlasmCore,
            items: Vec::new(),
            logs: Vec::new(),
            ytdlp_installed: false,
            ffmpeg_installed: false,
            input_url: String::new(),
            output_dir: default_output_dir,
            audio_only: false,
            selected_quality: "Best".to_string(),
            selected_video_format: "mp4".to_string(),
            selected_audio_format: "mp3".to_string(),
            selected_audio_quality: "320K".to_string(),
            media_info_loading: false,
            media_info: None,
            media_info_error: None,
            concurrent_limit: 3,
            auto_retry: true,
            show_persistent_logs: false,
            sys,
            last_sys_refresh: std::time::Instant::now(),
            top_nav_layout: false,
        };

        // Trigger initial core checks
        let _ = app.runtime.send_command(GuiCommand::CheckDependencies);
        let _ = app.runtime.send_command(GuiCommand::RefreshQueue);

        app
    }

    /// Drains all incoming asynchronous events from the Tokio background engine
    fn drain_events(&mut self) {
        let events = self.runtime.drain_events();
        for event in events {
            match event {
                CoreEvent::QueueUpdated(queue_items) => {
                    self.items = queue_items;
                }
                CoreEvent::DownloadProgress {
                    id,
                    progress,
                    speed,
                    eta,
                } => {
                    if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                        item.progress = progress;
                        item.speed = speed;
                        item.eta = eta;
                    }
                }
                CoreEvent::DownloadComplete { id, title } => {
                    if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                        item.status = "Complete".to_string();
                        item.progress = 100.0;
                    }
                    self.logs
                        .push(format!("✓ [{}] Completed successfully", title));
                }
                CoreEvent::DownloadError { id, error } => {
                    if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                        item.status = "Error".to_string();
                        self.logs.push(format!("✗ [ID #{}] Error: {}", id, error));
                    }
                }
                CoreEvent::MediaInfoFetched(result) => {
                    self.media_info_loading = false;
                    match result {
                        Ok(info) => {
                            self.media_info = Some(info);
                            self.media_info_error = None;
                        }
                        Err(err) => {
                            self.media_info = None;
                            self.media_info_error = Some(err);
                        }
                    }
                }
                CoreEvent::DependencyStatus { ytdlp, ffmpeg } => {
                    self.ytdlp_installed = ytdlp;
                    self.ffmpeg_installed = ffmpeg;
                }
                CoreEvent::LogLine(line) => {
                    self.logs.push(line);
                    if self.logs.len() > 800 {
                        self.logs.remove(0);
                    }
                }
            }
        }
    }

    /// Renders the top navigation panel (horizontal)
    fn render_top_nav(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(16.0);

            // Tab navigation list
            let nav_tabs = [
                (Tab::Home, "Home"),
                (Tab::Queue, "Queue"),
                (Tab::Settings, "Settings"),
                (Tab::Logs, "Logs"),
                (Tab::About, "About"),
            ];

            for (tab_enum, label) in nav_tabs {
                let is_active = self.current_tab == tab_enum;

                let text_color = if is_active {
                    self.theme.primary()
                } else {
                    Color32::from_rgb(0x9c, 0xa3, 0xaf)
                };

                let fill_color = if is_active {
                    Color32::from_rgb(0x28, 0x28, 0x28)
                } else {
                    Color32::TRANSPARENT
                };

                let button =
                    egui::Button::new(RichText::new(label).strong().color(text_color).size(14.0))
                        .fill(fill_color)
                        .min_size(egui::vec2(0.0, 32.0));

                let response = ui.add(button);

                if response.clicked() {
                    self.current_tab = tab_enum;
                    if tab_enum == Tab::Queue {
                        let _ = self.runtime.send_command(GuiCommand::RefreshQueue);
                    }
                }

                ui.add_space(8.0);
            }
        });
    }

    /// Renders the sidebar navigation panel (left)
    fn render_sidebar(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.add_space(24.0);

            // Premium Brand Logo and name
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../../docs/assets/logo.svg"))
                        .max_width(28.0)
                        .max_height(28.0),
                );
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("MANGOFETCH")
                            .font(FontId::new(17.0, FontFamily::Proportional))
                            .strong()
                            .color(Color32::WHITE),
                    );
                });
            });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(16.0);

            // Tab navigation list
            let nav_tabs = [
                (Tab::Home, "Home"),
                (Tab::Queue, "Queue"),
                (Tab::Settings, "Settings"),
                (Tab::Logs, "Logs"),
                (Tab::About, "About"),
            ];

            for (tab_enum, label) in nav_tabs {
                let is_active = self.current_tab == tab_enum;

                ui.horizontal(|ui| {
                    ui.add_space(8.0);

                    // Physical indicator on active hover
                    if is_active {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(4.0, 32.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(2), self.theme.primary());
                        ui.add_space(4.0);
                    } else {
                        ui.add_space(8.0);
                    }

                    let text_color = if is_active {
                        self.theme.primary()
                    } else {
                        Color32::from_rgb(0x9c, 0xa3, 0xaf)
                    };

                    let fill_color = if is_active {
                        Color32::from_rgb(0x28, 0x28, 0x28)
                    } else {
                        Color32::TRANSPARENT
                    };

                    let button = egui::Button::new(
                        RichText::new(label).strong().color(text_color).size(15.0),
                    )
                    .fill(fill_color)
                    .min_size(egui::vec2(ui.available_width() - 16.0, 32.0));

                    let response = ui.add(button);

                    if response.clicked() {
                        self.current_tab = tab_enum;
                        if tab_enum == Tab::Queue {
                            let _ = self.runtime.send_command(GuiCommand::RefreshQueue);
                        }
                    }
                });

                ui.add_space(8.0);
            }

            // Push dynamic tactical radar scanner to the bottom
            let remaining_h = ui.available_height();
            if remaining_h > 40.0 {
                ui.add_space(remaining_h - 36.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let scan_chars = ["|", "/", "-", "\\"];
                    let idx = ((chrono::Local::now().timestamp_subsec_millis() / 250) % 4) as usize;
                    let spin = scan_chars[idx];
                    ui.label(
                        RichText::new(format!("{}  [RADAR: ACTIVE]", spin))
                            .font(FontId::monospace(10.0))
                            .color(self.theme.primary()),
                    );
                });
            }
        });
    }

    /// Home Tab: Entry input, options, media preview, download triggers
    fn draw_home_tab(&mut self, ui: &mut Ui) {
        section_header(ui, "Command Center");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            // LEFT COLUMN: Controls well (URL inputs, directory, options)
            let total_width = ui.available_width();
            let left_col_w = total_width * 0.45; // 45% of available width
            let right_col_w = total_width * 0.55 - 16.0; // 55% minus gap

            ui.allocate_ui(Vec2::new(left_col_w, ui.available_height()), |ui| {
                ui.vertical(|ui| {
                    // Input Card
                    surface_card(ui, |ui| {
                        ui.label(RichText::new("URL to Download").color(Color32::from_rgb(0xd1, 0xd5, 0xdb)));
                        ui.add_space(6.0);

                        // Sunken input well combining text edit and inspect button
                        sunken_well(ui, |ui| {
                            ui.horizontal(|ui| {
                                let text_edit = ui.add_sized(
                                    Vec2::new(ui.available_width() - 95.0, 24.0),
                                    egui::TextEdit::singleline(&mut self.input_url)
                                        .hint_text("Paste YouTube, Twitch, TikTok or direct link...")
                                        .frame(false) // removes egui default box borders
                                );

                                if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    self.fetch_preview();
                                }

                                if ui.add_sized(Vec2::new(80.0, 24.0), Button::new("Inspect")).clicked() {
                                    self.fetch_preview();
                                }
                            });
                        });
                        ui.add_space(4.0);
                    });

                    ui.add_space(12.0);

                    // Options Card
                    surface_card(ui, |ui| {
                        ui.label(RichText::new("Download Options").strong().color(Color32::from_rgb(0xd1, 0xd5, 0xdb)));
                        ui.add_space(10.0);

                        ui.checkbox(&mut self.audio_only, "Extract Audio Only (MP3/M4A/FLAC)");
                        ui.add_space(10.0);

                        if !self.audio_only {
                            ui.horizontal(|ui| {
                                ui.label("Video Quality:");
                                egui::ComboBox::from_id_salt("quality_combo")
                                    .selected_text(&self.selected_quality)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_quality, "Best".to_string(), "Best (Default)");
                                        ui.selectable_value(&mut self.selected_quality, "1080p".to_string(), "1080p HD");
                                        ui.selectable_value(&mut self.selected_quality, "720p".to_string(), "720p");
                                        ui.selectable_value(&mut self.selected_quality, "480p".to_string(), "480p");
                                    });
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Video Format:");
                                egui::ComboBox::from_id_salt("video_format_combo")
                                    .selected_text(&self.selected_video_format)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_video_format, "mp4".to_string(), "MP4");
                                        ui.selectable_value(&mut self.selected_video_format, "mkv".to_string(), "MKV");
                                        ui.selectable_value(&mut self.selected_video_format, "webm".to_string(), "WEBM");
                                    });
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label("Audio Format:");
                                egui::ComboBox::from_id_salt("audio_format_combo")
                                    .selected_text(&self.selected_audio_format)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_audio_format, "mp3".to_string(), "MP3");
                                        ui.selectable_value(&mut self.selected_audio_format, "m4a".to_string(), "M4A");
                                        ui.selectable_value(&mut self.selected_audio_format, "flac".to_string(), "FLAC");
                                        ui.selectable_value(&mut self.selected_audio_format, "wav".to_string(), "WAV");
                                        ui.selectable_value(&mut self.selected_audio_format, "opus".to_string(), "OPUS");
                                    });
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Audio Quality:");
                                egui::ComboBox::from_id_salt("audio_quality_combo")
                                    .selected_text(&self.selected_audio_quality)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.selected_audio_quality, "320K".to_string(), "320K (High)");
                                        ui.selectable_value(&mut self.selected_audio_quality, "256K".to_string(), "256K");
                                        ui.selectable_value(&mut self.selected_audio_quality, "192K".to_string(), "192K (Medium)");
                                        ui.selectable_value(&mut self.selected_audio_quality, "128K".to_string(), "128K (Low)");
                                        ui.selectable_value(&mut self.selected_audio_quality, "0".to_string(), "0 (Best possible)");
                                    });
                            });
                        }

                        ui.add_space(12.0);
                        ui.label("Output Directory:");
                        ui.add_space(4.0);

                        // Sunken output well for directory browse
                        sunken_well(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    Vec2::new(ui.available_width() - 85.0, 24.0),
                                    egui::TextEdit::singleline(&mut self.output_dir)
                                        .frame(false)
                                );

                                if ui.add_sized(Vec2::new(75.0, 24.0), Button::new("Browse...")).clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        self.output_dir = path.to_string_lossy().to_string();
                                    }
                                }
                            });
                        });

                        ui.add_space(16.0);

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let start_btn = ui.add_sized(
                                Vec2::new(160.0, 32.0),
                                Button::new(RichText::new("Enqueue Download").strong().color(Color32::BLACK))
                                    .fill(self.theme.primary())
                            );

                            if start_btn.clicked() && !self.input_url.is_empty() {
                                let cmd = GuiCommand::StartDownload {
                                    url: self.input_url.clone(),
                                    output_dir: self.output_dir.clone(),
                                    quality: Some(self.selected_quality.clone()),
                                    video_format: Some(self.selected_video_format.clone()),
                                    audio_format: Some(self.selected_audio_format.clone()),
                                    audio_quality: Some(self.selected_audio_quality.clone()),
                                    audio_only: self.audio_only,
                                };
                                let _ = self.runtime.send_command(cmd);

                                self.logs.push(format!("Enqueued download: {}", self.input_url));
                                self.input_url.clear();
                                self.media_info = None;
                                self.current_tab = Tab::Queue;
                            }
                        });
                    });
                });
            });

            ui.add_space(16.0); // Column separator gap

            // RIGHT COLUMN: Operational Manual & Pre-fetch metadata preview
            ui.allocate_ui(Vec2::new(right_col_w, ui.available_height()), |ui| {
                ui.vertical(|ui| {
                    if self.media_info_loading {
                        surface_card(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(RichText::new("Analyzing stream metadata...").italics().color(Color32::from_rgb(0x9c, 0xa3, 0xaf)));
                                });
                            });
                        });
                    } else if let Some(ref info) = self.media_info {
                        surface_card(ui, |ui| {
                            ui.label(RichText::new("Media Metadata Inspector").strong().color(self.theme.primary()));
                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                ui.label("Title:");
                                ui.label(RichText::new(&info.title).strong().color(Color32::WHITE));
                            });
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Duration:");
                                if let Some(sec) = info.duration {
                                    let min = sec / 60;
                                    let s = sec % 60;
                                    ui.label(RichText::new(format!("{:02}:{:02}", min, s)).color(Color32::WHITE));
                                } else {
                                    ui.label("Live Stream / Unknown");
                                }
                            });
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Platform detected:");
                                crate::widgets::platform_pill(ui, &info.platform);
                            });

                            ui.add_space(10.0);
                        });
                    } else if let Some(ref err) = self.media_info_error {
                        Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(242, 139, 130, 15))
                            .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(242, 139, 130, 60)))
                            .inner_margin(Margin::same(12))
                            .corner_radius(CornerRadius::same(6))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("Metadata check failed:\n{}", err)).color(Color32::from_rgb(0xf2, 0x8b, 0x82)));
                            });
                    } else {
                        // Render standard Quick Start Guide
                        surface_card(ui, |ui| {
                            ui.label(RichText::new("QUICK START").strong().color(self.theme.primary()));
                            ui.add_space(14.0);

                            ui.label(
                                RichText::new("MangoFetch is a fast, multi-source download manager built for efficiency.")
                                    .color(Color32::from_rgb(0xd1, 0xd5, 0xdb))
                            );
                            ui.add_space(12.0);

                            ui.label(RichText::new("GETTING STARTED:").strong().color(Color32::WHITE));
                            ui.label("1. Paste a media link inside the [URL TO DOWNLOAD] sunken well.");
                            ui.label("2. Click 'Inspect' or press Enter to analyze the stream metadata.");
                            ui.label("3. Choose custom options (audio-extraction, quality profiles).");
                            ui.label("4. Click 'Enqueue Download' to dispatch to the async thread pool.");

                            ui.add_space(16.0);
                            ui.separator();
                            ui.add_space(12.0);

                            ui.label(RichText::new("INTEGRATED PIPELINES:").strong().color(self.theme.secondary()));
                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                crate::widgets::platform_pill(ui, "YouTube");
                                ui.add_space(4.0);
                                crate::widgets::platform_pill(ui, "Instagram");
                                ui.add_space(4.0);
                                crate::widgets::platform_pill(ui, "TikTok");
                                ui.add_space(4.0);
                                crate::widgets::platform_pill(ui, "Twitch");
                                ui.add_space(4.0);
                                crate::widgets::platform_pill(ui, "Torrent");
                            });
                            ui.add_space(4.0);
                        });
                    }
                });
            });
        });
    }

    /// Queue Tab: interactive grid with progress bars
    fn draw_queue_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            section_header(ui, "Active Download Queue");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Refresh Queue").clicked() {
                    let _ = self.runtime.send_command(GuiCommand::RefreshQueue);
                }
            });
        });
        ui.add_space(8.0);

        if self.items.is_empty() {
            sunken_well(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("No active or completed downloads in the queue.")
                            .color(Color32::from_rgb(0x9c, 0xa3, 0xaf)),
                    );
                });
            });
            return;
        }

        // Render queue in a rich table
        ScrollArea::vertical().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(40.0)) // ID
                .column(Column::exact(110.0)) // Platform
                .column(Column::remainder()) // Title
                .column(Column::exact(110.0)) // Status
                .column(Column::exact(150.0)) // Progress
                .column(Column::exact(90.0)) // Actions
                .header(26.0, |mut header| {
                    header.col(|ui| {
                        ui.label(RichText::new("# ID").strong().color(self.theme.primary()));
                    });
                    header.col(|ui| {
                        ui.label(
                            RichText::new("PLATFORM")
                                .strong()
                                .color(self.theme.primary()),
                        );
                    });
                    header.col(|ui| {
                        ui.label(
                            RichText::new("MEDIA TITLE")
                                .strong()
                                .color(self.theme.primary()),
                        );
                    });
                    header.col(|ui| {
                        ui.label(RichText::new("STATUS").strong().color(self.theme.primary()));
                    });
                    header.col(|ui| {
                        ui.label(
                            RichText::new("PROGRESS")
                                .strong()
                                .color(self.theme.primary()),
                        );
                    });
                    header.col(|ui| {
                        ui.label(
                            RichText::new("CONTROLS")
                                .strong()
                                .color(self.theme.primary()),
                        );
                    });
                })
                .body(|body| {
                    let items_clone = self.items.clone();
                    body.rows(34.0, items_clone.len(), |mut row| {
                        let item = &items_clone[row.index()];

                        // ID
                        row.col(|ui| {
                            ui.label(
                                RichText::new(format!("{:02}", item.id))
                                    .font(FontId::monospace(11.0))
                                    .color(Color32::from_rgb(0x6b, 0x72, 0x80)),
                            );
                        });

                        // Platform (official branding)
                        row.col(|ui| {
                            crate::widgets::platform_pill(ui, &item.platform);
                        });

                        // Title
                        row.col(|ui| {
                            ui.label(RichText::new(&item.title).strong().color(Color32::WHITE));
                        });

                        // Status & Dot
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                status_dot(ui, &item.status);
                                ui.add_space(2.0);
                                ui.label(&item.status);
                            });
                        });

                        // Progress Bar & Speed
                        row.col(|ui| {
                            ui.vertical(|ui| {
                                ui.add_space(2.0);
                                let p = item.progress / 100.0;
                                ui.add(
                                    ProgressBar::new(p)
                                        .show_percentage()
                                        .fill(self.theme.primary()),
                                );

                                if item.status == "Active" && item.speed > 0.0 {
                                    let speed_str = format!("{:.1} MB/s", item.speed / 1_048_576.0);
                                    ui.label(
                                        RichText::new(speed_str)
                                            .font(FontId::monospace(9.0))
                                            .color(self.theme.secondary()),
                                    );
                                }
                            });
                        });

                        // Action button controls (tactile switch panel)
                        row.col(|ui| {
                            ui.horizontal(|ui| {
                                if item.status == "Active" {
                                    let btn = egui::Button::new(
                                        RichText::new("⏸").color(self.theme.primary()),
                                    );
                                    if ui.add(btn).clicked() {
                                        let _ =
                                            self.runtime.send_command(GuiCommand::PauseDownload {
                                                id: item.id,
                                            });
                                    }
                                } else if item.status == "Paused" {
                                    let btn = egui::Button::new(
                                        RichText::new("▶")
                                            .color(Color32::from_rgb(0x34, 0xa8, 0x53)),
                                    );
                                    if ui.add(btn).clicked() {
                                        let _ =
                                            self.runtime.send_command(GuiCommand::ResumeDownload {
                                                id: item.id,
                                            });
                                    }
                                }

                                let del_btn = egui::Button::new(
                                    RichText::new("❌").color(Color32::from_rgb(0xf2, 0x8b, 0x82)),
                                );
                                if ui.add(del_btn).clicked() {
                                    let _ = self
                                        .runtime
                                        .send_command(GuiCommand::RemoveDownload { id: item.id });
                                }
                            });
                        });
                    });
                });
        });
    }

    /// Settings Tab: engine config
    fn draw_settings_tab(&mut self, ui: &mut Ui) {
        section_header(ui, "Preferences");
        ui.add_space(8.0);

        ScrollArea::vertical().show(ui, |ui| {
            surface_card(ui, |ui| {
                ui.label(
                    RichText::new("Application Layout & Behavior")
                        .strong()
                        .color(self.theme.primary()),
                );
                ui.add_space(12.0);

                ui.checkbox(
                    &mut self.top_nav_layout,
                    "Use Top Navigation Bar instead of Sidebar (Hotkey: L)",
                );
                ui.add_space(8.0);

                ui.checkbox(
                    &mut self.show_persistent_logs,
                    "Show Persistent Engine Output Terminal (TUI Mode)",
                );
            });

            ui.add_space(16.0);

            surface_card(ui, |ui| {
                ui.label(
                    RichText::new("Concurrency & Limits")
                        .strong()
                        .color(self.theme.primary()),
                );
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label("Max Concurrent Downloads:");
                    ui.add(egui::Slider::new(&mut self.concurrent_limit, 1..=8));
                });

                ui.add_space(8.0);
                ui.checkbox(&mut self.auto_retry, "Automatically retry failed downloads");
            });

            ui.add_space(16.0);

            surface_card(ui, |ui| {
                ui.label(
                    RichText::new("Graphical Customization")
                        .strong()
                        .color(self.theme.primary()),
                );
                ui.add_space(12.0);

                ui.label("Active Brand Preset Theme:");
                ui.add_space(8.0);

                let presets = [
                    BrandPreset::PlasmCore,
                    BrandPreset::OxidizedGold,
                    BrandPreset::VioletReaction,
                    BrandPreset::CoolantLiquid,
                    BrandPreset::CriticalMass,
                ];

                ui.horizontal_wrapped(|ui| {
                    for preset in presets {
                        let active = self.theme == preset;

                        let fill = if active {
                            crate::theme::MonolithSurfaces::SURFACE_5
                        } else {
                            crate::theme::MonolithSurfaces::SURFACE_3
                        };
                        let border_stroke = if active {
                            Stroke::new(1.5, preset.primary())
                        } else {
                            Stroke::new(1.0, crate::theme::MonolithSurfaces::SURFACE_6)
                        };

                        let response = Frame::NONE
                            .fill(fill)
                            .stroke(border_stroke)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Circular color sweeps indicators
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(18.0, 10.0),
                                        egui::Sense::hover(),
                                    );

                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_max(
                                            rect.min,
                                            egui::pos2(rect.center().x, rect.max.y),
                                        ),
                                        CornerRadius::same(2),
                                        preset.primary(),
                                    );
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_max(
                                            egui::pos2(rect.center().x, rect.min.y),
                                            rect.max,
                                        ),
                                        CornerRadius::same(2),
                                        preset.secondary(),
                                    );
                                    ui.add_space(4.0);

                                    let text_color = if active {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0x9c, 0xa3, 0xaf)
                                    };
                                    ui.label(
                                        RichText::new(preset.name()).strong().color(text_color),
                                    );
                                });
                            })
                            .response;

                        let response =
                            ui.interact(response.rect, response.id, egui::Sense::click());
                        if response.clicked() {
                            self.theme = preset;
                            crate::theme::apply_monolith_dark(ui.ctx(), preset);
                        }

                        ui.add_space(6.0);
                    }
                });
            });

            ui.add_space(16.0);

            // Engine status checks
            surface_card(ui, |ui| {
                ui.label(
                    RichText::new("External Dependencies")
                        .strong()
                        .color(self.theme.primary()),
                );
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label("yt-dlp Core Downloader:");
                    if self.ytdlp_installed {
                        brand_pill(ui, "INSTALLED", Color32::from_rgb(0x34, 0xa8, 0x53));
                    } else {
                        brand_pill(
                            ui,
                            "MISSING / RECOVERY",
                            Color32::from_rgb(0xf2, 0x8b, 0x82),
                        );
                    }
                });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label("ffmpeg Converter Suite:");
                    if self.ffmpeg_installed {
                        brand_pill(ui, "INSTALLED", Color32::from_rgb(0x34, 0xa8, 0x53));
                    } else {
                        brand_pill(ui, "MISSING", Color32::from_rgb(0xf2, 0x8b, 0x82));
                    }
                });

                ui.add_space(12.0);
                if ui.button("Force Re-Check Dependencies").clicked() {
                    let _ = self.runtime.send_command(GuiCommand::CheckDependencies);
                }
            });
        });
    }

    fn draw_logs_content(&self, ui: &mut Ui) {
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.logs.is_empty() {
                    ui.label(
                        RichText::new("[SYSTEM] Idle - Listening for download tasks...")
                            .font(FontId::monospace(11.0))
                            .color(Color32::from_rgb(0x9c, 0xa3, 0xaf)),
                    );
                } else {
                    for line in &self.logs {
                        let text_color = if line.starts_with('✓') {
                            Color32::from_rgb(0x81, 0xc9, 0x95) // Success green
                        } else if line.starts_with('✗') {
                            Color32::from_rgb(0xf2, 0x8b, 0x82) // Danger red
                        } else if line.starts_with('⚙') {
                            self.theme.primary()
                        } else {
                            Color32::from_rgb(0xe5, 0xe7, 0xeb) // Neutral
                        };

                        ui.label(
                            RichText::new(line)
                                .font(FontId::monospace(11.0))
                                .color(text_color),
                        );
                    }
                }
            });
    }

    /// Logs Tab: scrollable terminal mockup
    fn draw_logs_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            section_header(ui, "Engine Activity Shell");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Clear Buffer").clicked() {
                    self.logs.clear();
                }
            });
        });
        ui.add_space(6.0);

        sunken_well(ui, |ui| {
            self.draw_logs_content(ui);
        });
    }

    /// About Tab: information block
    fn draw_about_tab(&mut self, ui: &mut Ui) {
        section_header(ui, "About MangoFetch");
        ui.add_space(8.0);

        ScrollArea::vertical().show(ui, |ui| {
            surface_card(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("🥭").font(FontId::new(48.0, FontFamily::Proportional)));
                });
                ui.add_space(12.0);

                ui.label(
                    RichText::new("MangoFetch v0.7.2")
                        .font(FontId::new(20.0, FontFamily::Proportional))
                        .strong()
                        .color(self.theme.primary()),
                );

                ui.label("concurrent media downloading utility.");
                ui.add_space(12.0);

                ui.label("Credits & Contributors:");
                ui.label(
                    RichText::new("• Core Architecture & GUI: Jules Martins")
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label("• Framework: egui + eframe (Immediate mode Desktop Suite)");
                ui.label("• Async Engine: Tokio multi-threaded runtime");

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new("LICENSE AND LEGAL")
                        .font(FontId::new(12.0, FontFamily::Monospace))
                        .strong()
                        .color(self.theme.secondary()),
                );
                ui.add_space(4.0);
                ui.label("This software is licensed under the GPL-3.0-or-later License.");
            });
        });
    }

    /// Triggers url inspections
    fn fetch_preview(&mut self) {
        if !self.input_url.is_empty() {
            self.media_info_loading = true;
            self.media_info = None;
            self.media_info_error = None;
            let _ = self.runtime.send_command(GuiCommand::FetchMediaInfo {
                url: self.input_url.clone(),
            });
        }
    }
}

impl eframe::App for MangoFetchApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Drain pending core events
        self.drain_events();

        // Refresh system metrics every 2 seconds (not every frame)
        if self.last_sys_refresh.elapsed() >= std::time::Duration::from_secs(2) {
            self.sys.refresh_cpu();
            self.sys.refresh_memory();
            self.last_sys_refresh = std::time::Instant::now();
        }

        // Check for 'l' key to toggle layout
        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            self.top_nav_layout = !self.top_nav_layout;
        }

        // Separator painter — draws crisp 1px chrome borders between panels (native desktop feel)
        let sep_color = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let sep_stroke = Stroke::new(1.0, sep_color);
        let sep = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("panel_separators"),
        ));

        // 2. Top Toolbar — fixed 36px, deepest chrome tone
        let top = egui::TopBottomPanel::top("command_bar")
            .exact_height(54.0)
            .frame(Frame::NONE.fill(Color32::from_rgb(0x10, 0x10, 0x10)))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("mangofetch")
                            .font(FontId::new(11.0, FontFamily::Monospace))
                            .color(Color32::from_rgb(0x4a, 0x54, 0x68)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(14.0);

                        // Pulsing connection dot glow
                        let is_even =
                            (chrono::Local::now().timestamp_subsec_millis() / 500) % 2 == 0;
                        let status_lbl = if is_even { "Active" } else { "Online" };
                        status_dot(ui, status_lbl);

                        ui.label(
                            RichText::new("CONNECTED")
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(Color32::from_rgb(0x34, 0xa8, 0x53)),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("│")
                                .color(Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
                        );
                        ui.add_space(6.0);

                        let active_cnt = self.items.iter().filter(|i| i.status == "Active").count();
                        ui.label(
                            RichText::new(format!("THREAD POOL: {} ACTIVE", active_cnt))
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(self.theme.primary()),
                        );
                    });
                });
            });
        // 1px border beneath toolbar
        sep.hline(
            top.response.rect.left()..=top.response.rect.right(),
            top.response.rect.bottom(),
            sep_stroke,
        );

        // 3. Bottom Status Bar — fixed 26px, same deep chrome tone
        let bottom = egui::TopBottomPanel::bottom("status_bar")
            .exact_height(28.0)
            .frame(Frame::NONE.fill(Color32::from_rgb(0x0C, 0x0C, 0x0C)))
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_space(14.0);

                    let cpu_usage = self.sys.global_cpu_info().cpu_usage();
                    let cpu_bar_width = 8usize;
                    let cpu_filled = (((cpu_usage / 100.0) * cpu_bar_width as f32).round()
                        as usize)
                        .min(cpu_bar_width);
                    let cpu_bar = format!(
                        "{}{}",
                        "█".repeat(cpu_filled),
                        "░".repeat(cpu_bar_width.saturating_sub(cpu_filled))
                    );

                    let total_mem = self.sys.total_memory() / 1_048_576;
                    let used_mem = self.sys.used_memory() / 1_048_576;
                    let ram_pct = (used_mem as f32 / total_mem as f32 * 100.0).clamp(0.0, 100.0);
                    let ram_bar_width = 8usize;
                    let ram_filled = (((ram_pct / 100.0) * ram_bar_width as f32).round() as usize)
                        .min(ram_bar_width);
                    let ram_bar = format!(
                        "{}{}",
                        "█".repeat(ram_filled),
                        "░".repeat(ram_bar_width.saturating_sub(ram_filled))
                    );

                    ui.label(
                        RichText::new(format!(
                            "CPU [{}] {:.1}%    RAM [{}] {}/{} MB",
                            cpu_bar, cpu_usage, ram_bar, used_mem, total_mem
                        ))
                        .font(FontId::new(10.0, FontFamily::Monospace))
                        .color(Color32::from_rgb(0x55, 0x5f, 0x72)),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(format!(
                                "{}  ·  v{}",
                                self.theme.name().to_uppercase(),
                                env!("CARGO_PKG_VERSION")
                            ))
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(Color32::from_rgb(0x4a, 0x54, 0x68)),
                        );
                    });
                });
            });
        // 1px border above status bar
        sep.hline(
            bottom.response.rect.left()..=bottom.response.rect.right(),
            bottom.response.rect.top(),
            sep_stroke,
        );

        if self.show_persistent_logs {
            let log_panel = egui::TopBottomPanel::bottom("persistent_logs_panel")
                .resizable(true)
                .min_height(120.0)
                .frame(Frame::NONE.fill(Color32::from_rgb(0x18, 0x18, 0x18)))
                .show(ctx, |ui| {
                    Frame::NONE.inner_margin(Margin::same(8)).show(ui, |ui| {
                        self.draw_logs_content(ui);
                    });
                });
            sep.hline(
                log_panel.response.rect.left()..=log_panel.response.rect.right(),
                log_panel.response.rect.top(),
                sep_stroke,
            );
        }

        if self.top_nav_layout {
            let top_nav = egui::TopBottomPanel::top("top_nav_panel")
                .exact_height(48.0)
                .frame(Frame::NONE.fill(Color32::from_rgb(0x15, 0x15, 0x15)))
                .show(ctx, |ui| {
                    self.render_top_nav(ui);
                });
            sep.hline(
                top_nav.response.rect.left()..=top_nav.response.rect.right(),
                top_nav.response.rect.bottom(),
                sep_stroke,
            );
        } else {
            // 4. Left Sidebar — slightly lighter than toolbar chrome, clean nav panel
            let sidebar = egui::SidePanel::left("left_sidebar")
                .resizable(false)
                .exact_width(200.0)
                .frame(Frame::NONE.fill(Color32::from_rgb(0x15, 0x15, 0x15)))
                .show(ctx, |ui| {
                    self.render_sidebar(ui);
                });
            // 1px border on the right edge of sidebar
            sep.vline(
                sidebar.response.rect.right(),
                sidebar.response.rect.top()..=sidebar.response.rect.bottom(),
                sep_stroke,
            );
        }

        // 5. Central Content Panel — clean surface, no decorative pattern
        // Slightly warmer/lighter than chrome to visually anchor content
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(0x1C, 0x1C, 0x1C)))
            .show(ctx, |ui| {
                Frame::NONE
                    .inner_margin(Margin::same(20))
                    .show(ui, |ui| match self.current_tab {
                        Tab::Home => self.draw_home_tab(ui),
                        Tab::Queue => self.draw_queue_tab(ui),
                        Tab::Settings => self.draw_settings_tab(ui),
                        Tab::Logs => self.draw_logs_tab(ui),
                        Tab::About => self.draw_about_tab(ui),
                    });
            });

        // Repaint every 250ms for telemetry and queue state
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
