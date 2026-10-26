// No console window on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod flap;
mod icon;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Split-Flap Clock")
            .with_app_id("splitflap-clock")
            .with_inner_size([520.0, 260.0])
            .with_min_inner_size([160.0, 80.0])
            .with_icon(egui::IconData {
                rgba: icon::pixels(),
                width: icon::SIZE,
                height: icon::SIZE,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Split-Flap Clock",
        options,
        Box::new(|cc| Ok(Box::new(app::ClockApp::new(cc)))),
    )
}
