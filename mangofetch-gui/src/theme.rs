//! Theming: MonolithUI → egui visuals
//! Adapta el sistema de colores de MonolithUI a egui

use egui::{Color32, Stroke};

/// Presets de brand (colores primarios)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrandPreset {
    PlasmCore,      // Cyan #22d3ee + Violet #a855f7
    OxidizedGold,   // Amber #f59e0b
    VioletReaction, // Purple #a855f7 + Crimson #e11d48
    CoolantLiquid,  // Cyan #06b6d4 + Teal #2dd4bf
    CriticalMass,   // Scarlet #ef4444 + Dark Red #991b1b
}

impl BrandPreset {
    pub fn name(&self) -> &'static str {
        match self {
            BrandPreset::PlasmCore => "Plasma Core",
            BrandPreset::OxidizedGold => "Oxidized Gold",
            BrandPreset::VioletReaction => "Violet Reaction",
            BrandPreset::CoolantLiquid => "Coolant Liquid",
            BrandPreset::CriticalMass => "Critical Mass",
        }
    }

    pub fn primary(&self) -> Color32 {
        match self {
            BrandPreset::PlasmCore => hex_to_color32("#22d3ee"),
            BrandPreset::OxidizedGold => hex_to_color32("#f59e0b"),
            BrandPreset::VioletReaction => hex_to_color32("#a855f7"),
            BrandPreset::CoolantLiquid => hex_to_color32("#06b6d4"),
            BrandPreset::CriticalMass => hex_to_color32("#ef4444"),
        }
    }

    pub fn secondary(&self) -> Color32 {
        match self {
            BrandPreset::PlasmCore => hex_to_color32("#a855f7"),
            BrandPreset::OxidizedGold => hex_to_color32("#fbbf24"),
            BrandPreset::VioletReaction => hex_to_color32("#e11d48"),
            BrandPreset::CoolantLiquid => hex_to_color32("#2dd4bf"),
            BrandPreset::CriticalMass => hex_to_color32("#991b1b"),
        }
    }
}

/// Convierte hex string a Color32 (solo #RRGGBB)
pub fn hex_to_color32(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}

/// Agrega alpha a un color
pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let arr = color.to_array();
    let r = arr[0];
    let g = arr[1];
    let b = arr[2];
    Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0) as u8)
}

/// Colores de la rampa de superficies de MonolithUI
pub struct MonolithSurfaces;
impl MonolithSurfaces {
    pub const SURFACE_0: Color32 = Color32::from_rgb(0x08, 0x08, 0x08); // deepest
    pub const SURFACE_1: Color32 = Color32::from_rgb(0x10, 0x10, 0x10); // sidebar, side-nav
    pub const SURFACE_2: Color32 = Color32::from_rgb(0x18, 0x18, 0x18); // panels
    pub const SURFACE_3: Color32 = Color32::from_rgb(0x24, 0x24, 0x24); // root canvas
    pub const SURFACE_4: Color32 = Color32::from_rgb(0x2E, 0x2E, 0x2E); // cards, controls
    pub const SURFACE_5: Color32 = Color32::from_rgb(0x38, 0x38, 0x38); // hover states
    pub const SURFACE_6: Color32 = Color32::from_rgb(0x44, 0x44, 0x44); // active hover
}

/// Aplica el tema dark de MonolithUI al contexto egui
pub fn apply_monolith_dark(ctx: &egui::Context, brand: BrandPreset) {
    let mut visuals = egui::Visuals::dark();

    // Rampa de superficie de MonolithUI
    visuals.window_fill = MonolithSurfaces::SURFACE_1;
    visuals.panel_fill = MonolithSurfaces::SURFACE_0;
    visuals.faint_bg_color = MonolithSurfaces::SURFACE_2;
    visuals.extreme_bg_color = MonolithSurfaces::SURFACE_0;

    // El color primario del brand activo
    let primary = brand.primary();

    // Widgets styling (inactive, hovered, active)
    visuals.widgets.inactive.bg_fill = MonolithSurfaces::SURFACE_3;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, hex_to_color32("#a1a1aa")); // neutral-400
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6); // --ui-r-md

    visuals.widgets.hovered.bg_fill = MonolithSurfaces::SURFACE_4;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, primary);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);

    visuals.widgets.active.bg_fill = with_alpha(primary, 0.20);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5, primary);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

    // Selection color
    visuals.selection.bg_fill = with_alpha(primary, 0.15);
    visuals.selection.stroke = Stroke::new(1.0, primary);

    // Hyperlinks
    visuals.hyperlink_color = primary;

    ctx.set_visuals(visuals);
}

/// Carga fuentes embebidas (Outfit + DM Mono)
pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Cargar Outfit-Regular
    fonts.font_data.insert(
        "outfit_regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(
            include_bytes!("../assets/Outfit-Regular.ttf").to_vec(),
        )),
    );

    // Cargar Outfit-Bold
    fonts.font_data.insert(
        "outfit_bold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(
            include_bytes!("../assets/Outfit-Bold.ttf").to_vec(),
        )),
    );

    // Cargar DM Mono
    fonts.font_data.insert(
        "dm_mono".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(
            include_bytes!("../assets/DMMono-Regular.ttf").to_vec(),
        )),
    );

    // Configurar Outfit como la fuente proporcional por defecto
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "outfit_regular".to_owned());

    // Configurar DM Mono como la fuente monoespaciada
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "dm_mono".to_owned());

    ctx.set_fonts(fonts);
    tracing::debug!("MonolithUI custom fonts (Outfit & DM Mono) loaded successfully");
}
