use eframe::egui;
use mangofetch_gui::{AppRuntime, BrandPreset, MangoFetchApp};

fn main() -> Result<(), eframe::Error> {
    // Inicializar logging de tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Lanzar el runtime asíncrono de Tokio en thread separado
    let app_runtime = AppRuntime::start();

    // Configurar native options con viewport optimizado para una distribución premium
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 720.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("🥭 MangoFetch — Multi-source Download Station"),
        ..Default::default()
    };

    eframe::run_native(
        "MangoFetch",
        options,
        Box::new(move |cc| {
            // Install image loaders
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // Cargar fuentes personalizadas (Outfit + DM Mono) antes del renderizado
            mangofetch_gui::theme::load_fonts(&cc.egui_ctx);
            // Aplicar el tema oscuro de la estación industrial de MonolithUI
            mangofetch_gui::theme::apply_monolith_dark(&cc.egui_ctx, BrandPreset::PlasmCore);

            Ok(Box::new(MangoFetchApp::new(app_runtime)))
        }),
    )
}
