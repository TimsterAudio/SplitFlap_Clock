//! A test bench for one question, away from Split-Flap Clock itself: does
//! this computer draw a window without a title bar where it says it does?
//!
//! On one older Windows laptop, hiding the clock's title bar left a black
//! strip where it had been, and the right-click menu picked the item below
//! the one under the pointer: the picture landing a strip away from where
//! the window thinks it is. This draws a frame around the whole window, a
//! ring where it thinks the pointer is, rows and a menu to point at, and
//! reports what the window and the graphics say about themselves. It can
//! draw with OpenGL, as the clock does, or with DirectX 12, Vulkan, or
//! OpenGL by another route, to see whether any of them gets it right.
//!
//!     splitflap-graphics-test [--engine opengl|dx12|vulkan|wgpu-gl] [--frameless]

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, ViewportCommand};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Engine {
    OpenGl,
    Dx12,
    Vulkan,
    WgpuGl,
}

impl Engine {
    const ALL: [Engine; 4] = [Engine::OpenGl, Engine::Dx12, Engine::Vulkan, Engine::WgpuGl];

    fn arg(self) -> &'static str {
        match self {
            Engine::OpenGl => "opengl",
            Engine::Dx12 => "dx12",
            Engine::Vulkan => "vulkan",
            Engine::WgpuGl => "wgpu-gl",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Engine::OpenGl => "OpenGL (as the clock)",
            Engine::Dx12 => "DirectX 12",
            Engine::Vulkan => "Vulkan",
            Engine::WgpuGl => "OpenGL, the other way",
        }
    }

    fn from_arg(s: &str) -> Option<Engine> {
        Engine::ALL.into_iter().find(|e| e.arg() == s)
    }
}

/// What it was started with.
struct Args {
    engine: Engine,
    frameless: bool,
    /// An engine that wouldn't start, and why, when this is the fallback.
    failed: Option<String>,
}

fn args() -> Args {
    let mut out = Args {
        engine: Engine::OpenGl,
        frameless: false,
        failed: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--engine" => {
                if let Some(e) = it.next().as_deref().and_then(Engine::from_arg) {
                    out.engine = e;
                }
            }
            "--frameless" => out.frameless = true,
            "--failed" => out.failed = it.next(),
            _ => {}
        }
    }
    out
}

/// Starts this program again, with `engine` and a title bar or not, and
/// closes this one.
fn relaunch(engine: Engine, frameless: bool, failed: Option<&str>) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["--engine", engine.arg()]);
    if frameless {
        cmd.arg("--frameless");
    }
    if let Some(f) = failed {
        cmd.args(["--failed", f]);
    }
    let _ = cmd.spawn();
}

fn main() {
    let args = args();
    // The wgpu engines are picked by the environment, read when the
    // options are made, so set it first.
    match args.engine {
        Engine::Dx12 => std::env::set_var("WGPU_BACKEND", "dx12"),
        Engine::Vulkan => std::env::set_var("WGPU_BACKEND", "vulkan"),
        Engine::WgpuGl => std::env::set_var("WGPU_BACKEND", "gl"),
        Engine::OpenGl => {}
    }
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Split-Flap Clock graphics test")
            .with_inner_size([660.0, 600.0])
            .with_decorations(!args.frameless),
        ..Default::default()
    };
    options.renderer = match args.engine {
        Engine::OpenGl => eframe::Renderer::Glow,
        _ => eframe::Renderer::Wgpu,
    };
    let engine = args.engine;
    let frameless = args.frameless;
    let failed = args.failed.clone();
    let result = eframe::run_native(
        "Split-Flap Clock graphics test",
        options,
        Box::new(move |cc| Ok(Box::new(Bench::new(cc, engine, frameless, failed)))),
    );
    // An engine that won't start on this computer: say so, in OpenGL.
    if let Err(e) = result {
        if engine != Engine::OpenGl {
            relaunch(
                Engine::OpenGl,
                frameless,
                Some(&format!("{} wouldn't start: {e}", engine.label())),
            );
        } else {
            eprintln!("OpenGL wouldn't start either: {e}");
        }
    }
}

struct Bench {
    engine: Engine,
    frameless: bool,
    failed: Option<String>,
    /// What the graphics say they are.
    graphics: String,
    /// The last row and menu item clicked.
    clicked: Option<String>,
    /// A one-pixel nudge to the window's size under way: when to put it
    /// back.
    nudge_back: Option<f64>,
    copied: bool,
    saved: Option<String>,
}

impl Bench {
    fn new(
        cc: &eframe::CreationContext<'_>,
        engine: Engine,
        frameless: bool,
        failed: Option<String>,
    ) -> Self {
        let mut graphics = String::new();
        if let Some(gl) = cc.gl.as_ref() {
            use eframe::glow::HasContext;
            // SAFETY: plain queries of the context eframe just made current.
            let (vendor, renderer, version) = unsafe {
                (
                    gl.get_parameter_string(eframe::glow::VENDOR),
                    gl.get_parameter_string(eframe::glow::RENDERER),
                    gl.get_parameter_string(eframe::glow::VERSION),
                )
            };
            graphics = format!("OpenGL: {renderer} ({vendor}), version {version}");
        }
        if let Some(rs) = cc.wgpu_render_state.as_ref() {
            let info = rs.adapter.get_info();
            graphics = format!(
                "wgpu {:?}: {} ({:?}), driver {} {}",
                info.backend, info.name, info.device_type, info.driver, info.driver_info
            );
        }
        Self {
            engine,
            frameless,
            failed,
            graphics,
            clicked: None,
            nudge_back: None,
            copied: false,
            saved: None,
        }
    }

    /// Everything worth sending back, as text.
    fn report(&self, ctx: &egui::Context) -> String {
        let (inner, outer, ppp, native, monitor, pointer) = ctx.input(|i| {
            let v = i.viewport();
            (
                v.inner_rect,
                v.outer_rect,
                i.pixels_per_point,
                v.native_pixels_per_point,
                v.monitor_size,
                i.pointer.hover_pos(),
            )
        });
        let screen = ctx.content_rect();
        format!(
            "Split-Flap Clock graphics test {}\n\
             System: {} {}\n\
             Windows: {}\n\
             Engine: {}\n\
             Graphics: {}\n\
             Title bar: {}\n\
             Drawing area (as the program sees it): {:.0} x {:.0}\n\
             Window inside: {}\n\
             Window outside: {}\n\
             Scale: {:.2} (system {})\n\
             Screen: {}\n\
             Pointer: {}\n\
             Last clicked: {}\n\
             {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            windows_version(),
            self.engine.label(),
            if self.graphics.is_empty() {
                "unknown"
            } else {
                &self.graphics
            },
            if self.frameless { "hidden" } else { "shown" },
            screen.width(),
            screen.height(),
            fmt_rect(inner),
            fmt_rect(outer),
            ppp,
            native.map_or("unknown".into(), |n| format!("{n:.2}")),
            monitor.map_or("unknown".into(), |m| format!("{:.0} x {:.0}", m.x, m.y)),
            pointer.map_or("off the window".into(), |p| format!(
                "{:.0}, {:.0}",
                p.x, p.y
            )),
            self.clicked.as_deref().unwrap_or("nothing yet"),
            self.failed
                .as_deref()
                .map_or(String::new(), |f| format!("Note: {f}\n")),
        )
    }
}

fn fmt_rect(r: Option<Rect>) -> String {
    r.map_or("unknown".into(), |r| {
        format!(
            "{:.0} x {:.0} at {:.0}, {:.0}",
            r.width(),
            r.height(),
            r.min.x,
            r.min.y
        )
    })
}

/// Windows's own version line, from `ver`.
fn windows_version() -> String {
    if !cfg!(windows) {
        return "not Windows".into();
    }
    std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

impl eframe::App for Bench {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);
        if let Some(at) = self.nudge_back {
            if time >= at {
                self.nudge_back = None;
                let size = ctx.content_rect().size() - egui::vec2(1.0, 1.0);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
            }
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.page(ui, &ctx, time));
        });
        // A bright frame round all of what the program thinks is its
        // window, and a ring where it thinks the pointer is: drawn on top
        // of everything.
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("frame"),
        ));
        let screen = ctx.content_rect();
        painter.rect_stroke(
            screen.shrink(2.0),
            0.0,
            Stroke::new(4.0, Color32::from_rgb(255, 0, 200)),
            egui::StrokeKind::Inside,
        );
        if let Some(p) = ctx.input(|i| i.pointer.hover_pos()) {
            ring(&painter, p);
        }
        ctx.request_repaint();
    }
}

/// A green ring with a cross, where the program thinks the pointer is.
fn ring(painter: &egui::Painter, p: Pos2) {
    let green = Color32::from_rgb(40, 230, 90);
    painter.circle_stroke(p, 14.0, Stroke::new(2.5, green));
    painter.line_segment(
        [p - egui::vec2(20.0, 0.0), p + egui::vec2(20.0, 0.0)],
        Stroke::new(1.5, green),
    );
    painter.line_segment(
        [p - egui::vec2(0.0, 20.0), p + egui::vec2(0.0, 20.0)],
        Stroke::new(1.5, green),
    );
}

impl Bench {
    fn page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, time: f64) {
        ui.heading("Split-Flap Clock graphics test");
        if let Some(f) = &self.failed {
            ui.colored_label(Color32::from_rgb(255, 150, 60), format!("Note: {f}"));
        }
        ui.label(format!("Drawing with: {}", self.engine.label()));
        ui.label(egui::RichText::new(&self.graphics).weak());
        ui.add_space(6.0);
        ui.label(
            "Look for two things. The pink frame should run right round the edge of the window, \
             with no black strip outside it. And the green ring should sit exactly on the tip of \
             your mouse pointer.",
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let mut shown = !self.frameless;
            if ui.checkbox(&mut shown, "Title bar").changed() {
                self.frameless = !shown;
                ctx.send_viewport_cmd(ViewportCommand::Decorations(shown));
            }
            if ui
                .button("Nudge the size")
                .on_hover_text("A pixel bigger and back, as the clock now does")
                .clicked()
            {
                let size = ctx.content_rect().size() + egui::vec2(1.0, 1.0);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                self.nudge_back = Some(time + 0.2);
            }
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        });

        ui.add_space(8.0);
        ui.label("Point at a row and click it. The one that lights up should be the one under the pointer.");
        for n in 1..=5 {
            let (rect, r) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::click());
            let fill = if r.hovered() {
                Color32::from_rgb(244, 196, 48)
            } else {
                ui.visuals().faint_bg_color
            };
            let text = if r.hovered() {
                Color32::BLACK
            } else {
                ui.visuals().text_color()
            };
            ui.painter().rect_filled(rect, 4.0, fill);
            ui.painter().text(
                rect.left_center() + egui::vec2(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("Row {n}"),
                egui::FontId::proportional(16.0),
                text,
            );
            if r.clicked() {
                self.clicked = Some(format!("Row {n}"));
            }
            r.context_menu(|ui| {
                for m in 1..=5 {
                    if ui.button(format!("Menu item {m}")).clicked() {
                        self.clicked = Some(format!("Menu item {m}"));
                        ui.close();
                    }
                }
            });
        }
        ui.label(
            egui::RichText::new("Right-click a row for a menu: the item that lights up should be the one under the pointer too.")
                .weak(),
        );
        if let Some(c) = &self.clicked {
            ui.label(format!("You clicked: {c}. Was that the one you meant?"));
        }

        ui.add_space(10.0);
        ui.label("Try again drawing another way. Each starts this program afresh:");
        ui.horizontal_wrapped(|ui| {
            for e in Engine::ALL {
                for frameless in [false, true] {
                    let label = format!(
                        "{}{}",
                        e.label(),
                        if frameless { ", no title bar" } else { "" }
                    );
                    if ui.button(label).clicked() {
                        relaunch(e, frameless, None);
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
        });

        ui.add_space(10.0);
        ui.label(
            "What this computer says. Copy it, or save it beside this program, and send it back:",
        );
        let report = self.report(ctx);
        ui.add(
            egui::TextEdit::multiline(&mut report.as_str())
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            if ui.button("Copy the report").clicked() {
                ctx.copy_text(report.clone());
                self.copied = true;
            }
            if ui.button("Save the report").clicked() {
                self.saved = Some(save_report(&report));
            }
            if self.copied {
                ui.label("Copied.");
            }
            if let Some(s) = &self.saved {
                ui.label(s);
            }
        });
    }
}

/// Adds `report` to graphics-test-report.txt beside the program, and says
/// where.
fn save_report(report: &str) -> String {
    use std::io::Write;
    let path = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("graphics-test-report.txt")))
        .unwrap_or_else(|| "graphics-test-report.txt".into());
    let written = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{report}\n----"));
    match written {
        Ok(()) => format!("Added to {}", path.display()),
        Err(e) => format!("Couldn't save it: {e}"),
    }
}
