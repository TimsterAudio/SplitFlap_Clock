// No console window on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod alarm;
mod app;
mod autostart;
mod cities;
mod desktops;
mod flap;
mod icon;
mod lighting;
mod places;
mod rng;
mod sky;
mod sound;

fn main() -> eframe::Result {
    autostart::refresh();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Split-Flap Clock")
            .with_app_id(app::APP_ID)
            .with_decorations(!app::starts_frameless())
            .with_inner_size([520.0, 260.0])
            .with_min_inner_size([160.0, 80.0])
            .with_icon(egui::IconData {
                rgba: icon::pixels(),
                width: icon::SIZE,
                height: icon::SIZE,
            }),
        event_loop_builder: event_loop_builder(),
        ..Default::default()
    };
    eframe::run_native(
        "Split-Flap Clock",
        options,
        Box::new(|cc| Ok(Box::new(app::ClockApp::new(cc)))),
    )
}

/// On Linux the clock runs as an X11 window wherever X11 is there, XWayland
/// included: Wayland doesn't let apps keep themselves on top or on every
/// workspace.
#[cfg(target_os = "linux")]
fn event_loop_builder() -> Option<eframe::EventLoopBuilderHook> {
    use winit::platform::x11::EventLoopBuilderExtX11;
    let x11 = std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty());
    x11.then(|| {
        Box::new(|b: &mut eframe::EventLoopBuilder<_>| {
            b.with_x11();
        }) as eframe::EventLoopBuilderHook
    })
}

#[cfg(not(target_os = "linux"))]
fn event_loop_builder() -> Option<eframe::EventLoopBuilderHook> {
    None
}
