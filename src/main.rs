use eframe::egui;
mod file_explore;
use file_explore::FileExplorer;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Anvel"),
        ..Default::default()
    };

    eframe::run_native(
        "Anvel",
        options,
        Box::new(|_cc| Ok(Box::new(FileExplorer::default()))),
    )
}
