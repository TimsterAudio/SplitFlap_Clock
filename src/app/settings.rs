//! The settings window: tabs down the side, a live preview of the board
//! along the top, and each page's settings in cards, the basics apart from
//! the effects.

use super::*;
use egui::pos2;

/// The settings window's pages.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Page {
    #[default]
    Clock,
    Places,
    Alarms,
    Effects,
    Window,
}

impl Page {
    /// The pages under their group headings, as down the side.
    const GROUPS: [(&'static str, &'static [Page]); 3] = [
        ("THE CLOCK", &[Page::Clock, Page::Places, Page::Alarms]),
        ("THE SHOW", &[Page::Effects]),
        ("THE COMPUTER", &[Page::Window]),
    ];

    fn label(self) -> &'static str {
        match self {
            Page::Clock => "Clock",
            Page::Places => "Places",
            Page::Alarms => "Alarms",
            Page::Effects => "Effects",
            Page::Window => "Window and About",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Page::Clock => "🕓",
            Page::Places => "🌐",
            Page::Alarms => "⏰",
            Page::Effects => "✨",
            Page::Window => "🖵",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Page::Clock => "CLOCK",
            Page::Places => "PLACES",
            Page::Alarms => "ALARMS",
            Page::Effects => "EFFECTS",
            Page::Window => "WINDOW AND ABOUT",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Page::Clock => "What the main time shows, and how it looks.",
            Page::Places => {
                "Other times under the main clock. Drag a place by its handle to move it, into another region too; drag a region's heading to move the whole group."
            }
            Page::Alarms => "The board announces them with a chime. Snooze is nine minutes.",
            Page::Effects => {
                "The extras that make it feel like a real board. Start from a mood, then fine-tune it. None of them change the time."
            }
            Page::Window => "How the clock sits on your desktop, and who made it.",
        }
    }
}

/// A mood for the effects: a starting point that sets several at once.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Scene {
    Classic,
    Terminal,
    LastFlight,
    Library,
}

/// What a mood sets.
struct SceneLook {
    lighting: Mood,
    bulbs: Failures,
    replace: Replace,
    flip_speed: f32,
    refresh: Refresh,
    sound: bool,
    volume: f32,
    card_brightness: f32,
    light_board: bool,
}

impl Scene {
    const ALL: [Scene; 4] = [
        Scene::Classic,
        Scene::Terminal,
        Scene::LastFlight,
        Scene::Library,
    ];

    fn name(self) -> &'static str {
        match self {
            Scene::Classic => "Classic",
            Scene::Terminal => "Terminal 1975",
            Scene::LastFlight => "Last flight out",
            Scene::Library => "Quiet library",
        }
    }

    fn blurb(self) -> &'static str {
        match self {
            Scene::Classic => "Flat light, hourly refresh, quiet.",
            Scene::Terminal => "Warm lamps, a clatter on the hour.",
            Scene::LastFlight => "Late evening, slow flaps, bulbs failing.",
            Scene::Library => "White cards, no sound, no refresh.",
        }
    }

    fn look(self) -> SceneLook {
        let base = SceneLook {
            lighting: Mood::Off,
            bulbs: Failures::Sometimes,
            replace: Replace::Automatically,
            flip_speed: 1.0,
            refresh: Refresh::Hourly,
            sound: false,
            volume: 0.6,
            card_brightness: 0.0,
            light_board: false,
        };
        match self {
            Scene::Classic => base,
            Scene::Terminal => SceneLook {
                lighting: Mood::Warm,
                sound: true,
                card_brightness: 0.2,
                ..base
            },
            Scene::LastFlight => SceneLook {
                lighting: Mood::Evening,
                bulbs: Failures::Hourly,
                replace: Replace::ByHand,
                flip_speed: 0.7,
                refresh: Refresh::Random,
                sound: true,
                volume: 0.35,
                card_brightness: 0.45,
                ..base
            },
            Scene::Library => SceneLook {
                refresh: Refresh::Off,
                light_board: true,
                ..base
            },
        }
    }

    fn apply(self, s: &mut Saved) {
        let l = self.look();
        s.lighting = l.lighting;
        s.bulbs = l.bulbs;
        s.replace = l.replace;
        s.flip_speed = l.flip_speed;
        s.refresh = l.refresh;
        s.sound = l.sound;
        s.volume = l.volume;
        s.clock_sound = false;
        s.card_brightness = l.card_brightness;
        s.light_board = l.light_board;
    }

    /// Whether the settings are this mood's, give or take the volume.
    fn matches(self, s: &Saved) -> bool {
        let l = self.look();
        let close = |a: f32, b: f32| (a - b).abs() < 0.01;
        s.lighting == l.lighting
            && (l.lighting == Mood::Off || (s.bulbs == l.bulbs && s.replace == l.replace))
            && close(s.flip_speed, l.flip_speed)
            && s.refresh == l.refresh
            && s.sound == l.sound
            && !s.clock_sound
            && close(s.card_brightness, l.card_brightness)
            && s.light_board == l.light_board
    }

    /// The mood the settings are in, if they're in one.
    fn of(s: &Saved) -> Option<Scene> {
        Scene::ALL.into_iter().find(|m| m.matches(s))
    }
}

/// What can be dragged about on the Places page.
#[derive(Clone)]
enum Drag {
    /// A place, by its number.
    Place(u64),
    /// A region, by its key.
    Region(String),
}

/// The settings window's accent: the board's yellow, darkened on light
/// pages so it still reads.
fn accent(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        YELLOW
    } else {
        Color32::from_rgb(150, 108, 0)
    }
}

/// A card's background, a step up from the page's.
fn card_fill(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_gray(36)
    } else {
        Color32::from_gray(250)
    }
}

fn flap_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("flap".into()))
}

/// An on or off switch.
fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let size = vec2(38.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let dark = ui.visuals().dark_mode;
        let off_fill = if dark {
            Color32::from_gray(64)
        } else {
            Color32::from_gray(200)
        };
        let fill = egui::lerp(egui::Rgba::from(off_fill)..=egui::Rgba::from(YELLOW), t);
        let mut fill = Color32::from(fill);
        if !ui.is_enabled() {
            fill = fill.gamma_multiply(0.4);
        }
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, radius, fill);
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        let knob = if *on {
            Color32::from_gray(26)
        } else {
            Color32::from_gray(if dark { 200 } else { 255 })
        };
        let r = radius - 3.0 + if response.hovered() { 1.0 } else { 0.0 };
        ui.painter()
            .circle_filled(pos2(x, rect.center().y), r, knob);
    }
    response
}

/// A row of buttons, one of them lit: the choice.
fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark {
            Color32::from_gray(44)
        } else {
            Color32::from_gray(224)
        })
        .corner_radius(7)
        .inner_margin(3)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (v, label) in options {
                    let on = *value == *v;
                    let text = if on {
                        egui::RichText::new(*label).color(DARK_INK).strong()
                    } else {
                        egui::RichText::new(*label)
                    };
                    let b = egui::Button::new(text)
                        .fill(if on { YELLOW } else { Color32::TRANSPARENT })
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(5)
                        .min_size(vec2(0.0, 24.0));
                    if ui.add(b).clicked() && !on {
                        *value = *v;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// A card of settings under a small heading.
fn card<R>(ui: &mut Ui, heading: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(card_fill(ui))
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(heading)
                    .size(11.0)
                    .strong()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(2.0);
            add(ui)
        })
        .inner
}

/// The line between rows in a card.
fn divider(ui: &mut Ui) {
    ui.add(egui::Separator::default().spacing(10.0));
}

/// A setting: its name, a line saying what it does, and its controls.
fn row<R>(ui: &mut Ui, title: &str, help: &str, controls: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let w = (ui.available_width() * 0.42).clamp(150.0, 240.0);
        ui.allocate_ui_with_layout(
            vec2(w, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_width(w);
                ui.label(egui::RichText::new(title).strong());
                if !help.is_empty() {
                    ui.label(
                        egui::RichText::new(help)
                            .size(11.5)
                            .color(ui.visuals().weak_text_color()),
                    );
                }
            },
        );
        controls(ui)
    })
    .inner
}

/// `cells` at `k` times the size.
fn scaled(cells: Vec<Cell>, k: f32) -> Vec<Cell> {
    cells
        .into_iter()
        .map(|c| match c {
            Cell::Card {
                drum,
                text,
                size,
                font,
            } => Cell::Card {
                drum,
                text,
                size: size * k,
                font,
            },
            Cell::Colon { size } => Cell::Colon { size: size * k },
            Cell::Gap(g) => Cell::Gap(g * k),
        })
        .collect()
}

/// A handle to drag something by: two columns of dots.
fn grip(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 16.0), Sense::hover());
    let c = ui.visuals().weak_text_color();
    for col in [2.5, 7.5] {
        for r in [3.0, 8.0, 13.0] {
            ui.painter()
                .circle_filled(pos2(rect.left() + col, rect.top() + r), 1.3, c);
        }
    }
}

/// A small cross to click, to remove something; red under the pointer.
fn remove_button(ui: &mut Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
    let colour = if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().widgets.hovered.weak_bg_fill);
        Color32::from_rgb(230, 110, 95)
    } else {
        ui.visuals().weak_text_color()
    };
    let c = rect.center();
    let d = 4.5;
    let stroke = egui::Stroke::new(1.6, colour);
    ui.painter()
        .line_segment([c + vec2(-d, -d), c + vec2(d, d)], stroke);
    ui.painter()
        .line_segment([c + vec2(-d, d), c + vec2(d, -d)], stroke);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A coloured dot, for a place's or an alarm's colour.
fn dot(ui: &mut Ui, colour: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 6.0, colour);
}

impl ClockApp {
    pub(super) fn settings(&mut self, ctx: &egui::Context) {
        let before = self.saved.scale;
        let size = vec2(820.0, 700.0);
        if self.settings_at.is_none() {
            let (clock, screen) =
                ctx.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size));
            self.settings_at = clock.zip(screen).map(|(c, sc)| settings_spot(c, sc, size));
        }
        let mut builder = egui::ViewportBuilder::default()
            .with_title("Clock settings")
            .with_inner_size(size)
            .with_min_inner_size([640.0, 480.0]);
        // Next to the clock, where the system says where the clock is (not
        // on Wayland, which places windows itself).
        if let Some(at) = self.settings_at {
            builder = builder.with_position(at);
        }
        ctx.show_viewport_immediate(ViewportId::from_hash_of("settings"), builder, |ui, _| {
            if ui.input(|i| i.viewport().close_requested()) {
                self.settings_open = false;
            }
            self.settings_window(ui);
        });
        if self.saved.scale != before {
            if let Some(board) = self.fitted {
                self.resize(ctx, board, ctx.input(|i| i.time));
            }
        }
    }

    fn settings_window(&mut self, ui: &mut Ui) {
        let dark = ui.visuals().dark_mode;
        let page_fill = if dark {
            Color32::from_gray(27)
        } else {
            Color32::from_gray(240)
        };
        let side_fill = if dark {
            Color32::from_gray(22)
        } else {
            Color32::from_gray(230)
        };
        egui::Panel::bottom("settings foot")
            .frame(
                egui::Frame::new()
                    .fill(side_fill)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Changes apply straight away").weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(version_text()).weak());
                    });
                });
            });
        egui::Panel::left("settings pages")
            .resizable(false)
            .exact_size(196.0)
            .frame(
                egui::Frame::new()
                    .fill(side_fill)
                    .inner_margin(egui::Margin::symmetric(10, 8)),
            )
            .show(ui, |ui| self.page_tabs(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(page_fill))
            .show(ui, |ui| {
                self.preview(ui);
                egui::ScrollArea::vertical()
                    .id_salt(("settings page", self.page as u8))
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        egui::Frame::new()
                            .inner_margin(egui::Margin::symmetric(22, 16))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                self.page_ui(ui);
                            });
                    });
            });
        self.beer_modal(ui);
    }

    fn page_tabs(&mut self, ui: &mut Ui) {
        let counts = |p: Page, s: &Saved| match p {
            Page::Places => Some(s.zones.len()),
            Page::Alarms => Some(s.alarms.len()),
            _ => None,
        };
        for (group, pages) in Page::GROUPS {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(group)
                    .font(flap_font(13.0))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(2.0);
            for &p in pages {
                let on = self.page == p;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
                let painter = ui.painter();
                if on {
                    let tint = if ui.visuals().dark_mode {
                        Color32::from_rgb(46, 41, 24)
                    } else {
                        Color32::from_rgb(246, 232, 186)
                    };
                    painter.rect_filled(rect, 8.0, tint);
                } else if response.hovered() {
                    painter.rect_filled(rect, 8.0, ui.visuals().widgets.hovered.weak_bg_fill);
                }
                let colour = if on {
                    accent(ui)
                } else {
                    ui.visuals().text_color()
                };
                let font = egui::FontId::proportional(14.0);
                painter.text(
                    rect.left_center() + vec2(12.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    p.icon(),
                    font.clone(),
                    colour,
                );
                painter.text(
                    rect.left_center() + vec2(38.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    p.label(),
                    font.clone(),
                    colour,
                );
                if let Some(n) = counts(p, &self.saved) {
                    painter.text(
                        rect.right_center() - vec2(12.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        n.to_string(),
                        egui::FontId::proportional(12.0),
                        ui.visuals().weak_text_color(),
                    );
                }
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.page = p;
                }
            }
        }
    }

    /// The board as it looks with these settings: the main time, and under
    /// it the date, or the place or alarm being changed.
    fn preview(&mut self, ui: &mut Ui) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 132.0), Sense::hover());
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);
        let painter = ui.painter_at(rect);
        let s = &self.saved;
        let pal = s.palette();
        painter.rect_filled(rect, 0.0, pal.board);
        let now = chrono::Local::now();
        let ink = self.ink();
        let main_ink = s.time_colour.map_or(ink, rgb);
        let top = time_cells(
            now.hour(),
            now.minute(),
            s.seconds.then(|| now.second()),
            s.h24,
            true,
        );
        // The second line: what's being changed, or else the date.
        let open_place = (self.page == Page::Places)
            .then(|| s.zones.iter().find(|z| Some(z.id) == self.open_place))
            .flatten();
        let open_alarm = (self.page == Page::Alarms)
            .then(|| s.alarms.iter().find(|a| Some(a.id) == self.open_alarm))
            .flatten();
        let (second, second_ink) = if let Some(z) = open_place {
            let t = s.time_at(Some(z.id));
            let day = t.format("%a").to_string().to_uppercase();
            (
                zone_cells(
                    &z.label.to_uppercase(),
                    t.hour(),
                    t.minute(),
                    &day,
                    s.h24,
                    None,
                ),
                z.colour.map_or(ink, rgb),
            )
        } else if let Some(a) = open_alarm {
            let remark = match a.reason.trim() {
                "" => "NOW".to_string(),
                r => r.to_uppercase(),
            };
            let mut cells = text_cards(&a.label.to_uppercase(), a.label.chars().count().max(1));
            cells.push(Cell::Gap(14.0));
            cells.extend(time_cells(a.hour, a.minute, None, true, false));
            cells.push(Cell::Gap(14.0));
            cells.extend(text_cards(&remark, remark.chars().count()));
            (cells, a.colour.map_or(ink, rgb))
        } else if s.date {
            let date = now.format("%a %-d %b %Y").to_string().to_uppercase();
            (date_cells(&date), s.date_colour.map_or(ink, rgb))
        } else {
            (Vec::new(), ink)
        };
        let mood = s.lighting;
        let shadow = mood.shadow();
        let speed = s.flip_speed.clamp(0.25, 4.0) as f64;
        let mut look = self.look();
        let rows = [(0u8, top, main_ink, 58.0), (1, second, second_ink, 22.0)];
        let gap = 12.0;
        let heights: f32 = rows
            .iter()
            .filter(|r| !r.1.is_empty())
            .map(|r| r.3)
            .sum::<f32>()
            + gap;
        let mut y = rect.center().y - heights / 2.0;
        let mut busy = false;
        for (row, cells, row_ink, h) in rows {
            if cells.is_empty() {
                continue;
            }
            let k = h / row_height(&cells);
            let cells = scaled(cells, k);
            let w = row_width(&cells).min(rect.width() - 24.0);
            let cells = fit(cells, w);
            let mut x = rect.center().x - row_width(&cells) / 2.0;
            look.ink = row_ink;
            for (i, cell) in cells.iter().enumerate() {
                match cell {
                    Cell::Card {
                        drum,
                        text,
                        size,
                        font,
                    } => {
                        let flap = self
                            .preview_flaps
                            .entry((row, i))
                            .or_insert_with(|| Flap::new(*drum, time, 0.0, 1.0));
                        if flap.drum() != *drum {
                            *flap = Flap::new(*drum, time, 0.0, 1.0);
                        }
                        flap.set(text);
                        flap.tick(time, speed);
                        busy |= flap.busy(time);
                        let r = Rect::from_min_size(pos2(x, y + h - size.y), *size);
                        if shadow > 0.0 {
                            painter.add(
                                egui::epaint::RectShape::filled(
                                    r.translate(vec2(0.0, size.y * 0.07)),
                                    (size.y * 0.07) as u8,
                                    Color32::from_black_alpha((shadow * 170.0) as u8),
                                )
                                .with_blur_width(size.y * 0.22),
                            );
                        }
                        flap.draw(&ctx, &painter, r, &flap_font(size.y * font), &look);
                        x += size.x;
                        if let Some(Cell::Card { .. }) = cells.get(i + 1) {
                            x += size.y * SPACING;
                        }
                    }
                    Cell::Colon { size } => {
                        let c = x + size.x / 2.0;
                        let d = size.y * 0.06;
                        for dy in [-0.16, 0.16] {
                            painter.rect_filled(
                                Rect::from_center_size(
                                    pos2(c, y + h - size.y / 2.0 + dy * size.y),
                                    vec2(d * 1.6, d * 1.6),
                                ),
                                1.0,
                                row_ink,
                            );
                        }
                        x += size.x;
                    }
                    Cell::Gap(g) => x += g,
                }
            }
            y += h + gap;
        }
        self.lighting.draw(&painter, rect, mood, s.dimmer, time);
        painter.text(
            rect.right_bottom() - vec2(10.0, 6.0),
            egui::Align2::RIGHT_BOTTOM,
            "Preview",
            egui::FontId::proportional(11.0),
            pal.caption,
        );
        if busy {
            ctx.request_repaint();
        }
    }

    fn page_ui(&mut self, ui: &mut Ui) {
        let page = self.page;
        ui.label(
            egui::RichText::new(page.title())
                .font(flap_font(28.0))
                .color(accent(ui)),
        );
        ui.label(egui::RichText::new(page.blurb()).weak());
        ui.add_space(6.0);
        match page {
            Page::Clock => self.clock_page(ui),
            Page::Places => self.places_page(ui),
            Page::Alarms => self.alarms_page(ui),
            Page::Effects => self.effects_page(ui),
            Page::Window => self.window_page(ui),
        }
    }

    fn clock_page(&mut self, ui: &mut Ui) {
        let s = &mut self.saved;
        card(ui, "TIME", |ui| {
            row(ui, "Hours", "For every place on the board", |ui| {
                segmented(ui, &mut s.h24, &[(false, "12-hour"), (true, "24-hour")]);
            });
            divider(ui);
            row(
                ui,
                "Seconds",
                "Two more big cards, flipping every second",
                |ui| {
                    toggle(ui, &mut s.seconds);
                },
            );
            divider(ui);
            row(ui, "Date", "The line under the time", |ui| {
                toggle(ui, &mut s.date);
            });
        });
        card(ui, "LOOK", |ui| {
            row(ui, "Cards", "White cards have dark letters", |ui| {
                segmented(
                    ui,
                    &mut s.light_board,
                    &[(false, "Black cards"), (true, "White cards")],
                );
            });
            divider(ui);
            row(ui, "Letters", "Everything without its own colour", |ui| {
                ui.add_enabled_ui(!s.light_board, |ui| {
                    segmented(
                        ui,
                        &mut s.ink,
                        &[(Ink::Yellow, "Yellow"), (Ink::White, "White")],
                    );
                })
                .response
                .on_disabled_hover_text("White cards have dark letters");
            });
            divider(ui);
            let ink = s.default_ink();
            row(ui, "Main time colour", "Its own, or the letters'", |ui| {
                colour_choice(ui, "time", &mut s.time_colour, ink);
            });
            divider(ui);
            row(ui, "Date colour", "Its own, or the letters'", |ui| {
                colour_choice(ui, "date", &mut s.date_colour, ink);
            });
        });
    }

    fn places_page(&mut self, ui: &mut Ui) {
        self.add_place(ui);
        ui.add_space(4.0);
        let s = &mut self.saved;
        s.region_order = region_order(s);
        let board_ink = s.default_ink();
        let utc = Utc::now();
        let utc_min = (utc.hour() * 60 + utc.minute()) as i32;
        let accent = accent(ui);
        // What was dropped where: (what, onto which region, before which
        // place, or at the region's end).
        let mut dropped: Option<(Drag, String, Option<u64>)> = None;
        let mut remove = None;
        let open = &mut self.open_place;
        card(ui, "ON THE BOARD", |ui| {
            if s.zones.is_empty() {
                ui.label(egui::RichText::new("None yet. Add one above.").weak());
                return;
            }
            let regions = s.region_order.clone();
            for (n, region) in regions.iter().enumerate() {
                // The region's heading: drag it to move the group, or drop a
                // place on it to put the place in this region.
                let title = match region.as_str() {
                    "" if n == 0 => "NO REGION".to_string(),
                    "" => "OTHER (NO REGION)".to_string(),
                    r => r.to_string(),
                };
                let count = s.zones.iter().filter(|z| z.region_key() == *region).count();
                let head = ui
                    .horizontal(|ui| {
                        ui.dnd_drag_source(
                            ui.id().with(("region drag", region)),
                            Drag::Region(region.clone()),
                            |ui| {
                                grip(ui);
                                ui.label(
                                    egui::RichText::new(&title)
                                        .font(flap_font(17.0))
                                        .color(ui.visuals().strong_text_color()),
                                );
                            },
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let word = if count == 1 { "place" } else { "places" };
                            ui.label(egui::RichText::new(format!("{count} {word}")).weak());
                        });
                    })
                    .response;
                let head = ui.interact(head.rect, ui.id().with(("region", region)), Sense::hover());
                if head.dnd_hover_payload::<Drag>().is_some() {
                    ui.painter().hline(
                        head.rect.x_range(),
                        head.rect.top() - 2.0,
                        egui::Stroke::new(2.0, accent),
                    );
                }
                if let Some(d) = head.dnd_release_payload::<Drag>() {
                    let first = s
                        .zones
                        .iter()
                        .find(|z| z.region_key() == *region)
                        .map(|z| z.id);
                    dropped = Some(((*d).clone(), region.clone(), first));
                }
                let ids: Vec<u64> = s
                    .zones
                    .iter()
                    .filter(|z| z.region_key() == *region)
                    .map(|z| z.id)
                    .collect();
                for (k, id) in ids.iter().enumerate() {
                    let Some(z) = s.zones.iter_mut().find(|z| z.id == *id) else {
                        continue;
                    };
                    let is_open = *open == Some(z.id);
                    let t = {
                        let utc = Utc::now();
                        match z.offset_now() {
                            Some(off) => FixedOffset::east_opt(off * 60)
                                .map(|f| utc.with_timezone(&f).naive_local())
                                .unwrap_or_else(|| utc.naive_utc()),
                            None => utc.naive_utc(),
                        }
                    };
                    let when = if s.h24 {
                        t.format("%H:%M %a")
                    } else {
                        t.format("%-I:%M %p %a")
                    }
                    .to_string()
                    .to_uppercase();
                    let frame = egui::Frame::new()
                        .fill(if ui.visuals().dark_mode {
                            Color32::from_gray(29)
                        } else {
                            Color32::from_gray(242)
                        })
                        .stroke(if is_open {
                            egui::Stroke::new(1.0, accent.gamma_multiply(0.6))
                        } else {
                            egui::Stroke::NONE
                        })
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(10, 6));
                    let resp = frame
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.dnd_drag_source(
                                    ui.id().with(("place drag", z.id)),
                                    Drag::Place(z.id),
                                    |ui| {
                                        grip(ui);
                                        dot(ui, z.colour.map_or(board_ink, rgb));
                                        ui.allocate_ui_with_layout(
                                            vec2(120.0, 20.0),
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.set_width(120.0);
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(z.label.to_uppercase())
                                                            .font(flap_font(17.0))
                                                            .color(
                                                                ui.visuals().strong_text_color(),
                                                            ),
                                                    )
                                                    .truncate(),
                                                );
                                            },
                                        );
                                    },
                                );
                                ui.label(egui::RichText::new(&z.tz).weak().size(12.0));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let arrow = if is_open { "⏶" } else { "⏷" };
                                        if ui
                                            .add(egui::Button::new(arrow).frame(false))
                                            .on_hover_text(if is_open {
                                                "Close"
                                            } else {
                                                "Change it"
                                            })
                                            .clicked()
                                        {
                                            *open = if is_open { None } else { Some(z.id) };
                                        }
                                        ui.label(egui::RichText::new(&when).font(flap_font(16.0)));
                                    },
                                );
                            });
                            if is_open {
                                ui.add_space(6.0);
                                let mut gone = false;
                                place_editor(ui, z, board_ink, utc_min, &mut gone);
                                if gone {
                                    remove = Some(z.id);
                                }
                            }
                        })
                        .response;
                    let resp =
                        ui.interact(resp.rect, ui.id().with(("place", z.id)), Sense::hover());
                    if resp
                        .dnd_hover_payload::<Drag>()
                        .is_some_and(|d| matches!(*d, Drag::Place(_)))
                    {
                        let below = ui
                            .ctx()
                            .pointer_hover_pos()
                            .is_some_and(|p| p.y > resp.rect.center().y);
                        let y = if below {
                            resp.rect.bottom() + 2.0
                        } else {
                            resp.rect.top() - 2.0
                        };
                        ui.painter()
                            .hline(resp.rect.x_range(), y, egui::Stroke::new(2.0, accent));
                    }
                    if let Some(d) = resp.dnd_release_payload::<Drag>() {
                        let below = ui
                            .ctx()
                            .pointer_interact_pos()
                            .is_some_and(|p| p.y > resp.rect.center().y);
                        let before = if below {
                            ids.get(k + 1).copied()
                        } else {
                            Some(*id)
                        };
                        dropped = Some(((*d).clone(), region.clone(), before));
                    }
                }
                ui.add_space(4.0);
            }
        });
        if let Some(id) = remove {
            if *open == Some(id) {
                *open = None;
            }
            s.zones.retain(|z| z.id != id);
        }
        if let Some((what, region, before)) = dropped {
            self.moved_place = move_dropped(s, what, &region, before);
        }
    }

    /// Search for a city or a time zone, pick one, Add.
    fn add_place(&mut self, ui: &mut Ui) {
        let found = find_places(&self.search);
        if self
            .picked
            .as_deref()
            .is_some_and(|p| !found.iter().any(|f| f.key == p))
        {
            self.picked = None;
        }
        // The one picked from the list, or else the best match.
        let choice = match &self.picked {
            Some(p) => found.iter().find(|f| f.key == *p),
            None => found.first(),
        };
        let mut add = false;
        ui.horizontal(|ui| {
            let label = match choice {
                Some(c) => format!("Add {}", c.label),
                None => "Add".into(),
            };
            let button_w = 150.0;
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Add a place: a city, a country, or a time zone")
                    .desired_width(ui.available_width() - button_w - 8.0),
            );
            add |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let b = egui::Button::new(egui::RichText::new(label).color(DARK_INK).strong())
                .fill(YELLOW)
                .min_size(vec2(button_w, 26.0));
            add |= ui.add_enabled(choice.is_some(), b).clicked();
        });
        if let (true, Some(c)) = (add, choice) {
            self.saved.zones.push(Zone {
                label: c.label.clone(),
                at: c.at,
                ..Zone::new(c.tz)
            });
            self.saved.number();
            self.open_place = self.saved.zones.last().map(|z| z.id);
            self.search.clear();
            self.picked = None;
            return;
        }
        if self.search.trim().is_empty() {
            return;
        }
        let chosen = choice.map(|c| c.key.clone());
        egui::Frame::new()
            .fill(card_fill(ui))
            .corner_radius(8)
            .inner_margin(6)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("found")
                    .max_height(190.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for f in &found {
                            let on = chosen.as_deref() == Some(f.key.as_str());
                            let mut text = egui::text::LayoutJob::default();
                            let strong = ui.visuals().strong_text_color();
                            let weak = ui.visuals().weak_text_color();
                            let font = egui::FontId::proportional(13.5);
                            text.append(
                                &f.name,
                                0.0,
                                egui::TextFormat::simple(font.clone(), strong),
                            );
                            if !f.around.is_empty() {
                                text.append(
                                    &format!(", {}", f.around),
                                    0.0,
                                    egui::TextFormat::simple(
                                        font.clone(),
                                        ui.visuals().text_color(),
                                    ),
                                );
                            }
                            text.append(
                                &format!("   {}", f.tz),
                                0.0,
                                egui::TextFormat::simple(egui::FontId::proportional(12.0), weak),
                            );
                            if ui.selectable_label(on, text).clicked() {
                                self.picked = Some(f.key.clone());
                            }
                        }
                        if found.is_empty() {
                            ui.label(egui::RichText::new("No city or time zone matches.").weak());
                        }
                    });
            });
    }

    fn alarms_page(&mut self, ui: &mut Ui) {
        let mut test = None;
        let s = &mut self.saved;
        let places: Vec<(u64, String)> = s
            .zones
            .iter()
            .map(|z| (z.id, format!("{} ({})", z.label.to_uppercase(), z.tz)))
            .collect();
        let board_ink = s.default_ink();
        let accent = accent(ui);
        let mut remove = None;
        let mut changed = Vec::new();
        let open = &mut self.open_alarm;
        let confirm = &mut self.confirm_remove;
        let b = egui::Button::new(
            egui::RichText::new("+  Add an alarm")
                .color(DARK_INK)
                .strong(),
        )
        .fill(YELLOW)
        .min_size(vec2(150.0, 28.0));
        if ui.add(b).clicked() {
            s.alarms.push(Alarm::default());
            s.number();
            *open = s.alarms.last().map(|a| a.id);
        }
        if s.alarms.is_empty() {
            ui.label(egui::RichText::new("No alarms yet.").weak());
        }
        for (i, a) in s.alarms.iter_mut().enumerate() {
            let before = (a.hour, a.minute, a.days, a.place, a.on, a.date);
            let is_open = *open == Some(a.id);
            let colour = a.colour.map_or(board_ink, rgb);
            egui::Frame::new()
                .fill(card_fill(ui))
                .stroke(if is_open {
                    egui::Stroke::new(1.0, accent.gamma_multiply(0.6))
                } else {
                    ui.visuals().widgets.noninteractive.bg_stroke
                })
                .corner_radius(10)
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        toggle(ui, &mut a.on).on_hover_text("On or off");
                        ui.add_space(4.0);
                        let shown = if a.on {
                            colour
                        } else {
                            ui.visuals().weak_text_color()
                        };
                        ui.label(
                            egui::RichText::new(format!("{:02}:{:02}", a.hour, a.minute))
                                .font(flap_font(30.0))
                                .color(shown),
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            let name = match a.reason.trim() {
                                "" => a.label.to_uppercase(),
                                r => format!("{} · {}", a.label.to_uppercase(), r.to_uppercase()),
                            };
                            ui.label(egui::RichText::new(name).font(flap_font(17.0)).color(shown));
                            let at = places
                                .iter()
                                .find(|(id, _)| Some(*id) == a.place)
                                .map_or("your time".to_string(), |(_, n)| n.clone());
                            ui.label(
                                egui::RichText::new(format!("{} · {}", a.when_text(), at))
                                    .size(11.5)
                                    .weak(),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let arrow = if is_open { "⏶" } else { "⏷" };
                            if ui
                                .add(egui::Button::new(arrow).frame(false))
                                .on_hover_text(if is_open { "Close" } else { "Change it" })
                                .clicked()
                            {
                                *open = if is_open { None } else { Some(a.id) };
                            }
                            // Removing it from here, with a word to make sure.
                            if *confirm == Some(a.id) {
                                if ui.button("Keep it").clicked() {
                                    *confirm = None;
                                }
                                let yes = egui::Button::new(
                                    egui::RichText::new("Remove").color(Color32::WHITE).strong(),
                                )
                                .fill(Color32::from_rgb(178, 62, 48));
                                if ui.add(yes).clicked() {
                                    remove = Some(i);
                                    *confirm = None;
                                }
                                ui.label("Remove this alarm?");
                                return;
                            }
                            if remove_button(ui)
                                .on_hover_text("Remove this alarm")
                                .clicked()
                            {
                                *confirm = Some(a.id);
                            }
                            dot(ui, colour);
                            if a.date.is_none() {
                                for d in (0..7).rev() {
                                    let lit = a.days[d];
                                    let name = ["M", "T", "W", "T", "F", "S", "S"][d];
                                    let (rect, _) =
                                        ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                                    let (fill, text) = if lit {
                                        (YELLOW, DARK_INK)
                                    } else {
                                        (
                                            ui.visuals().extreme_bg_color,
                                            ui.visuals().weak_text_color(),
                                        )
                                    };
                                    ui.painter().rect_filled(rect, 4.0, fill);
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        name,
                                        egui::FontId::proportional(10.0),
                                        text,
                                    );
                                }
                            }
                        });
                    });
                    if is_open {
                        ui.add_space(8.0);
                        alarm_editor(ui, a, i, &places, board_ink, &mut remove, &mut test);
                    }
                });
            // A changed alarm can ring again today, at its new time.
            if before != (a.hour, a.minute, a.days, a.place, a.on, a.date) {
                a.rang = None;
                changed.push(a.id);
            }
        }
        // A changed alarm only rings from now on, not for a time just gone.
        for id in changed {
            self.armed.remove(&id);
        }
        if let Some(i) = remove {
            if s.alarms.get(i).map(|a| a.id) == *open {
                *open = None;
            }
            s.alarms.remove(i);
        }
        ui.add_space(6.0);
        card(ui, "CHIME", |ui| {
            row(
                ui,
                "Chime volume",
                "Alarms ring even when the clatter is off",
                |ui| {
                    ui.add(egui::Slider::new(&mut s.alarm_volume, 0.0..=1.0).show_value(false));
                },
            );
        });
        if let Some(id) = test {
            self.ring(id);
        }
    }

    fn effects_page(&mut self, ui: &mut Ui) {
        let mut blow = false;
        let s = &mut self.saved;
        let current = Scene::of(s);
        let accent = accent(ui);
        ui.columns(4, |cols| {
            for (ui, m) in cols.iter_mut().zip(Scene::ALL) {
                let on = current == Some(m);
                let r = egui::Frame::new()
                    .fill(if on {
                        if ui.visuals().dark_mode {
                            Color32::from_rgb(46, 41, 24)
                        } else {
                            Color32::from_rgb(246, 232, 186)
                        }
                    } else {
                        card_fill(ui)
                    })
                    .stroke(if on {
                        egui::Stroke::new(1.5, accent)
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke
                    })
                    .corner_radius(10)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        // A little board in the mood's colours.
                        let l = m.look();
                        let (rect, _) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
                        let (bg, fg) = if l.light_board {
                            (Color32::from_rgb(206, 203, 195), DARK_INK)
                        } else {
                            (BOARD, YELLOW)
                        };
                        ui.painter().rect_filled(rect, 6.0, bg);
                        if l.lighting != Mood::Off {
                            let glow = match l.lighting {
                                Mood::Evening => Color32::from_rgba_premultiplied(150, 88, 30, 0),
                                _ => Color32::from_rgba_premultiplied(120, 86, 46, 0),
                            };
                            for fx in [0.28, 0.72] {
                                let c = pos2(rect.left() + rect.width() * fx, rect.top());
                                ui.painter().add(crate::lighting::halo(
                                    c,
                                    rect.width() * 0.2,
                                    rect.height() * 0.45,
                                    glow,
                                ));
                            }
                        }
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "14:35",
                            flap_font(24.0),
                            if l.lighting == Mood::Evening {
                                Color32::from_rgb(224, 169, 58)
                            } else {
                                fg
                            },
                        );
                        ui.add_space(2.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.label(egui::RichText::new(m.name()).strong());
                            ui.label(egui::RichText::new(m.blurb()).size(11.0).weak());
                        });
                    })
                    .response;
                let r = ui.interact(r.rect, ui.id().with(("scene", m.name())), Sense::click());
                if r.on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Start from this mood")
                    .clicked()
                {
                    m.apply(s);
                }
            }
        });
        ui.label(
            egui::RichText::new(match current {
                Some(m) => format!(
                    "Mood: {}. Change anything below to make it your own.",
                    m.name()
                ),
                None => "Mood: Custom, your own mix. Pick one above to start again.".into(),
            })
            .size(11.5)
            .weak(),
        );
        ui.add_space(4.0);
        card(ui, "MOTION", |ui| {
            row(ui, "Flip speed", "How fast the flaps fall", |ui| {
                ui.add(
                    egui::Slider::new(&mut s.flip_speed, 0.25..=4.0)
                        .logarithmic(true)
                        .custom_formatter(|v, _| format!("{v:.2}×")),
                );
                if ui
                    .add_enabled(s.flip_speed != 1.0, egui::Button::new("Normal").small())
                    .clicked()
                {
                    s.flip_speed = 1.0;
                }
            });
            divider(ui);
            row(
                ui,
                "Refresh",
                "Every card spins round once, like a real board",
                |ui| {
                    egui::ComboBox::from_id_salt("refresh")
                        .selected_text(s.refresh.label())
                        .show_ui(ui, |ui| {
                            for r in Refresh::ALL {
                                ui.selectable_value(&mut s.refresh, r, r.label());
                            }
                        });
                },
            );
            if s.refresh == Refresh::Random {
                row(ui, "", "", |ui| {
                    let (lo, hi) = &mut s.random_minutes;
                    ui.label("Between");
                    if ui.add(egui::DragValue::new(lo).range(1..=720)).changed() {
                        *hi = (*hi).max(*lo);
                    }
                    ui.label("and");
                    if ui.add(egui::DragValue::new(hi).range(1..=720)).changed() {
                        *lo = (*lo).min(*hi);
                    }
                    ui.label("minutes apart");
                });
            }
        });
        card(ui, "SOUND", |ui| {
            row(ui, "Clatter", "When the board starts and refreshes", |ui| {
                toggle(ui, &mut s.sound);
            });
            divider(ui);
            ui.add_enabled_ui(s.sound, |ui| {
                row(ui, "Volume", "", |ui| {
                    ui.add(egui::Slider::new(&mut s.volume, 0.0..=1.0).show_value(false));
                });
                divider(ui);
                row(
                    ui,
                    "Every minute too",
                    "A click whenever the time changes",
                    |ui| {
                        toggle(ui, &mut s.clock_sound);
                    },
                );
            });
            if let Some(e) = &self.sound_error {
                ui.colored_label(ui.visuals().warn_fg_color, e);
            }
        });
        card(ui, "LIGHT", |ui| {
            row(ui, "Lamps", "Warm bulbs over the board", |ui| {
                segmented(
                    ui,
                    &mut s.lighting,
                    &[
                        (Mood::Off, "Off"),
                        (Mood::Soft, "Soft"),
                        (Mood::Warm, "Warm"),
                        (Mood::Evening, "Evening"),
                    ],
                );
            });
            divider(ui);
            ui.add_enabled_ui(s.lighting != Mood::Off, |ui| {
                row(ui, "Dimmer", "The lamps turned down, or up", |ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut s.dimmer,
                            crate::lighting::DIMMEST..=crate::lighting::BRIGHTEST,
                        )
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                    );
                    if ui
                        .add_enabled(s.dimmer != 1.0, egui::Button::new("Normal").small())
                        .clicked()
                    {
                        s.dimmer = 1.0;
                    }
                });
                divider(ui);
                row(
                    ui,
                    "Bulbs",
                    "Now and then one flickers, and may blow",
                    |ui| {
                        egui::ComboBox::from_id_salt("bulbs")
                            .selected_text(s.bulbs.label())
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for f in Failures::ALL {
                                    ui.selectable_value(&mut s.bulbs, f, f.label());
                                }
                            });
                        if ui
                            .add_enabled(
                                s.bulbs != Failures::Never,
                                egui::Button::new("Blow one now"),
                            )
                            .on_hover_text("For anyone who can't wait: a bulb flickers, then blows")
                            .clicked()
                        {
                            blow = true;
                        }
                    },
                );
                divider(ui);
                row(
                    ui,
                    "Replace by hand",
                    "A dead bulb stays out until you click it on the board",
                    |ui| {
                        let mut by_hand = s.replace == Replace::ByHand;
                        if toggle(ui, &mut by_hand).changed() {
                            s.replace = if by_hand {
                                Replace::ByHand
                            } else {
                                Replace::Automatically
                            };
                        }
                    },
                );
                divider(ui);
                row(
                    ui,
                    "Bulb sounds",
                    "The pop, and the glass landing: on its own, whether the clatter's on or not",
                    |ui| {
                        ui.add(
                            egui::Slider::new(&mut s.bulb_volume, 0.0..=1.0).custom_formatter(
                                |v, _| {
                                    if v == 0.0 {
                                        "Off".into()
                                    } else {
                                        format!("{:.0}%", v * 100.0)
                                    }
                                },
                            ),
                        );
                    },
                );
            });
        });
        if blow {
            self.lighting.blow(ui.input(|i| i.time));
            ui.ctx().request_repaint();
        }
        card(ui, "CARDS AND SKY", |ui| {
            row(
                ui,
                "Card brightness",
                "Lighter cards show the split and edges better",
                |ui| {
                    ui.add(
                        egui::Slider::new(&mut s.card_brightness, 0.0..=1.0)
                            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                    );
                    if ui
                        .add_enabled(
                            s.card_brightness != 0.0,
                            egui::Button::new("Classic").small(),
                        )
                        .clicked()
                    {
                        s.card_brightness = 0.0;
                    }
                },
            );
            divider(ui);
            row(
                ui,
                "Day and night",
                "Sun, moon, sunrise or sunset at each place",
                |ui| {
                    toggle(ui, &mut s.sky_places);
                    ui.label("Places");
                    ui.add_space(12.0);
                    toggle(ui, &mut s.sky_main);
                    ui.label("Main time");
                },
            );
        });
    }

    fn window_page(&mut self, ui: &mut Ui) {
        let s = &mut self.saved;
        card(ui, "WINDOW", |ui| {
            row(
                ui,
                "Keep on top",
                "Above other windows, like a board above a busy hall",
                |ui| {
                    toggle(ui, &mut s.on_top);
                },
            );
            divider(ui);
            row(
                ui,
                "Hide title bar",
                "A bare slab: drag it by its face",
                |ui| {
                    toggle(ui, &mut s.frameless);
                },
            );
            divider(ui);
            let help = if cfg!(windows) {
                "On Windows this hides its taskbar button"
            } else {
                "On every workspace"
            };
            row(ui, "Show on all desktops", help, |ui| {
                toggle(ui, &mut s.all_desktops);
            });
            if let Some(e) = &self.desktops_error {
                ui.colored_label(ui.visuals().warn_fg_color, e);
            }
            divider(ui);
            row(ui, "Size", "Or drag the window's edge", |ui| {
                ui.add(egui::Slider::new(&mut s.scale, 0.3..=4.0).fixed_decimals(2));
            });
            divider(ui);
            row(ui, "Menus", "This window and the right-click menu", |ui| {
                segmented(
                    ui,
                    &mut s.menus,
                    &[
                        (Theme::System, "Like my system"),
                        (Theme::Light, "Light"),
                        (Theme::Dark, "Dark"),
                    ],
                );
            });
        });
        card(ui, "START", |ui| {
            row(
                ui,
                "Start when I log in",
                "So it's there every morning",
                |ui| {
                    let mut on = self.autostart;
                    if toggle(ui, &mut on).changed() {
                        match autostart::set(on) {
                            Ok(()) => {
                                self.autostart = on;
                                self.autostart_error = None;
                            }
                            Err(e) => self.autostart_error = Some(e),
                        }
                    }
                },
            );
            if let Some(e) = &self.autostart_error {
                ui.colored_label(ui.visuals().warn_fg_color, e);
            }
        });
        card(ui, "ABOUT", |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("SPLIT-FLAP CLOCK")
                            .font(flap_font(22.0))
                            .color(ui.visuals().strong_text_color()),
                    );
                    ui.label(egui::RichText::new(version_text()).weak());
                    ui.label(egui::RichText::new("Timster's Audio Works").weak());
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let b = egui::Button::new(
                        egui::RichText::new("🍺  Beer money")
                            .color(DARK_INK)
                            .strong(),
                    )
                    .fill(YELLOW)
                    .min_size(vec2(140.0, 30.0));
                    if ui.add(b).on_hover_text("Support the developer").clicked() {
                        self.beer_open = true;
                    }
                });
            });
        });
    }

    fn beer_modal(&mut self, ui: &mut Ui) {
        if !self.beer_open {
            return;
        }
        let shown = egui::Modal::new(egui::Id::new("beer money")).show(ui.ctx(), |ui| {
            ui.set_max_width(380.0);
            ui.heading("Beer money");
            ui.add_space(6.0);
            ui.label(
                "If you like Split-Flap Clock and want to support its developer with some beer funding, send it via PayPal to:",
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(BEER_PAYPAL).monospace().strong());
                if ui.small_button("Copy").clicked() {
                    ui.ctx().copy_text(BEER_PAYPAL.into());
                }
            });
            ui.add_space(6.0);
            ui.label(egui::RichText::new(CLINK).monospace().size(11.0));
            ui.add_space(6.0);
            ui.vertical_centered(|ui| {
                if ui.button("Cheers!").clicked() {
                    self.beer_open = false;
                }
            });
        });
        if shown.should_close() {
            self.beer_open = false;
        }
    }
}

/// A place found by a search: a city, or a time zone by its own name.
struct Found {
    /// What the board calls it.
    label: String,
    name: String,
    /// Where it is: region and country.
    around: String,
    tz: &'static str,
    at: Option<(f64, f64)>,
    /// Which one it is, for picking it from the list.
    key: String,
}

/// `text` lower case and without accents, as the cities' names are kept
/// for matching: "São Paulo" is "sao paulo".
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        let base = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
            'ç' | 'ć' | 'č' | 'ĉ' | 'ċ' => "c",
            'ď' | 'đ' => "d",
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
            'ğ' | 'ĝ' | 'ġ' | 'ģ' => "g",
            'ĥ' | 'ħ' => "h",
            'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
            'ĵ' => "j",
            'ķ' => "k",
            'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
            'ñ' | 'ń' | 'ņ' | 'ň' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
            'ŕ' | 'ŗ' | 'ř' => "r",
            'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => "s",
            'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
            'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
            'ŵ' => "w",
            'ý' | 'ÿ' | 'ŷ' => "y",
            'ź' | 'ż' | 'ž' => "z",
            'ß' => "ss",
            'æ' => "ae",
            'œ' => "oe",
            'þ' => "th",
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push_str(base);
    }
    out
}

/// The most places a search lists.
const MOST_FOUND: usize = 80;

/// The cities and time zones that `query` finds, best first: cities whose
/// name starts with it, then whose name has a word starting with it, then
/// with it anywhere in the name, then in that region or country, biggest
/// first in each; then time zones with it in their name. "Paris, France"
/// looks for Paris in France.
fn find_places(query: &str) -> Vec<Found> {
    let q = plain(query.trim());
    if q.is_empty() {
        return Vec::new();
    }
    let (city_q, where_q) = match q.split_once(',') {
        Some((c, w)) => (c.trim().to_string(), w.trim().to_string()),
        None => (q.clone(), String::new()),
    };
    let word = format!(" {city_q}");
    let mut hits: Vec<(u8, &crate::cities::City)> = crate::cities::CITIES
        .iter()
        .filter(|c| where_q.is_empty() || c.around.contains(&where_q))
        .filter_map(|c| {
            let score = if c.words.starts_with(&city_q) {
                0
            } else if c.words.contains(&word) {
                1
            } else if c.words.contains(&city_q) {
                2
            } else if where_q.is_empty() && c.around.contains(&city_q) {
                3
            } else {
                return None;
            };
            Some((score, c))
        })
        .collect();
    // The list is biggest first already; a stable sort keeps that.
    hits.sort_by_key(|(score, _)| *score);
    let mut out: Vec<Found> = hits
        .into_iter()
        .take(MOST_FOUND)
        .map(|(_, c)| {
            let around = [c.region, c.country]
                .into_iter()
                .filter(|p| !p.is_empty() && *p != c.name)
                .collect::<Vec<_>>()
                .join(", ");
            Found {
                label: c.ascii.to_uppercase().chars().take(NAME_CARDS).collect(),
                name: c.name.to_string(),
                around,
                tz: c.tz,
                at: Some((c.lat, c.lon)),
                key: format!("{}|{}|{}|{}", c.name, c.region, c.country, c.tz),
            }
        })
        .collect();
    if where_q.is_empty() && out.len() < MOST_FOUND {
        let zq = q.replace(' ', "_");
        for tz in chrono_tz::TZ_VARIANTS.iter().map(|t| t.name()) {
            if out.len() >= MOST_FOUND {
                break;
            }
            if tz.to_lowercase().contains(&zq) {
                out.push(Found {
                    label: default_label(tz),
                    name: tz.to_string(),
                    around: "time zone".into(),
                    tz,
                    at: None,
                    key: tz.to_string(),
                });
            }
        }
    }
    out
}

/// A place opened up: its name, region, colour and time, and Remove.
fn place_editor(ui: &mut Ui, z: &mut Zone, board_ink: Color32, utc_min: i32, remove: &mut bool) {
    row(ui, "Name", "What the board says, up to 12 cards", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut z.label)
                .char_limit(NAME_CARDS)
                .desired_width(160.0),
        );
    });
    row(ui, "Region", "The group it's shown in", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut z.region)
                .char_limit(REGION_CHARS)
                .desired_width(160.0)
                .hint_text("e.g. Europe"),
        );
    });
    row(ui, "Colour", "Its own, or the letters'", |ui| {
        colour_choice(ui, ("place", z.id), &mut z.colour, board_ink);
    });
    row(
        ui,
        "Time",
        "Its time zone's, summer time and all, or set by hand",
        |ui| {
            let mut by_hand = z.offset.is_some();
            if segmented(
                ui,
                &mut by_hand,
                &[(false, "From its time zone"), (true, "Set by hand")],
            ) {
                z.offset = if by_hand {
                    // Start from the time it is there now, so nothing jumps.
                    Some(z.offset_now().unwrap_or(0))
                } else {
                    None
                };
            }
        },
    );
    if let Some(off) = z.offset {
        row(ui, "", "", |ui| {
            let local = (utc_min + off).rem_euclid(24 * 60);
            let (mut h, mut m) = (local / 60, local % 60);
            let dh = ui.add(
                egui::DragValue::new(&mut h)
                    .range(0..=23)
                    .custom_formatter(|v, _| format!("{v:02}")),
            );
            ui.label(":");
            let dm = ui.add(
                egui::DragValue::new(&mut m)
                    .range(0..=59)
                    .custom_formatter(|v, _| format!("{v:02}")),
            );
            if dh.changed() || dm.changed() {
                z.offset = Some(offset_for(h * 60 + m, utc_min, off));
            }
            let off = z.offset.unwrap_or(off);
            let sign = if off < 0 { '-' } else { '+' };
            ui.label(
                egui::RichText::new(format!(
                    "UTC{sign}{:02}:{:02}, no summer time",
                    off.abs() / 60,
                    off.abs() % 60
                ))
                .weak(),
            );
        });
    }
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("Remove").color(Color32::from_rgb(230, 120, 105)))
                .clicked()
            {
                *remove = true;
            }
        });
    });
}

/// An alarm opened up: everything about it, Test and Remove.
fn alarm_editor(
    ui: &mut Ui,
    a: &mut Alarm,
    i: usize,
    places: &[(u64, String)],
    board_ink: Color32,
    remove: &mut Option<usize>,
    test: &mut Option<u64>,
) {
    row(ui, "Time", "Hours and minutes: type, or drag", |ui| {
        ui.add(
            egui::DragValue::new(&mut a.hour)
                .range(0..=23)
                .custom_formatter(|v, _| format!("{v:02}")),
        );
        ui.label(":");
        ui.add(
            egui::DragValue::new(&mut a.minute)
                .range(0..=59)
                .custom_formatter(|v, _| format!("{v:02}")),
        );
    });
    row(ui, "Name", "On the board when it rings", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut a.label)
                .char_limit(NAME_CARDS)
                .desired_width(160.0),
        );
    });
    row(ui, "Reason", "Shown instead of NOW", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut a.reason)
                .char_limit(NAME_CARDS)
                .desired_width(160.0)
                .hint_text("e.g. Dentist"),
        );
    });
    row(ui, "Colour", "Its row and buttons when it rings", |ui| {
        colour_choice(ui, ("alarm", a.id), &mut a.colour, board_ink);
    });
    row(ui, "Repeat", a.when_text().as_str(), |ui| {
        // The days it rings on stand out: lit yellow, like a board's lamps.
        let dated = a.date.is_some();
        for (d, name) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
            let on = a.days[d] && !dated;
            let (fill, text) = if on {
                (YELLOW, DARK_INK)
            } else {
                (
                    ui.visuals().extreme_bg_color,
                    ui.visuals().weak_text_color(),
                )
            };
            let day = egui::Button::new(egui::RichText::new(*name).strong().color(text))
                .fill(fill)
                .min_size(vec2(28.0, 26.0));
            if ui.add_enabled(!dated, day).clicked() {
                a.days[d] = !a.days[d];
            }
        }
    });
    row(ui, "On a date", "Once, on that day, instead", |ui| {
        let mut dated = a.date.is_some();
        if toggle(ui, &mut dated).changed() {
            a.date = dated.then(|| {
                let t = chrono::Local::now().date_naive() + chrono::Duration::days(1);
                (t.year(), t.month(), t.day())
            });
        }
        if let Some((y, m, d)) = &mut a.date {
            ui.add(egui::DragValue::new(d).range(1..=31).prefix("day "));
            ui.add(egui::DragValue::new(m).range(1..=12).prefix("month "));
            ui.add(egui::DragValue::new(y).range(2000..=2100));
        }
    });
    row(ui, "Time at", "Whose clock it goes by", |ui| {
        let current = places
            .iter()
            .find(|(id, _)| Some(*id) == a.place)
            .map_or("Your time", |(_, name)| name.as_str());
        egui::ComboBox::from_id_salt(("alarm place", a.id))
            .selected_text(current)
            .width(220.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut a.place, None, "Your time");
                for (id, name) in places {
                    ui.selectable_value(&mut a.place, Some(*id), name);
                }
            });
    });
    ui.horizontal(|ui| {
        if ui
            .button("▶  Test")
            .on_hover_text("Ring it now, to hear it")
            .clicked()
        {
            *test = Some(a.id);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(egui::RichText::new("Remove").color(Color32::from_rgb(230, 120, 105)))
                .clicked()
            {
                *remove = Some(i);
            }
        });
    });
}

/// Moves what was dragged: a place to before `before` (or the end of the
/// region), taking on `region`; or a region to before `region`. Says which
/// place is now in the first row that changed, from which the board
/// refreshes, if anything moved at all: where it went, for a move up the
/// list, and where it left, for a move down.
fn move_dropped(s: &mut Saved, what: Drag, region: &str, before: Option<u64>) -> Option<u64> {
    let rows = |s: &Saved| -> Vec<(u64, String)> {
        grouped(s)
            .iter()
            .map(|(_, z)| (z.id, z.region_key()))
            .collect()
    };
    let was = rows(s);
    match what {
        Drag::Place(id) => {
            let from = s.zones.iter().position(|z| z.id == id)?;
            let mut z = s.zones.remove(from);
            if z.region_key() != region {
                // Take the region's name as its other places spell it.
                z.region = s
                    .zones
                    .iter()
                    .find(|o| o.region_key() == region)
                    .map_or(region.to_string(), |o| o.region.clone());
            }
            let at = match before {
                // Dropped on itself: only its region can change.
                Some(b) if b == id => from.min(s.zones.len()),
                Some(b) => s
                    .zones
                    .iter()
                    .position(|o| o.id == b)
                    .unwrap_or(s.zones.len()),
                // After the region's last place, or at the end.
                None => s
                    .zones
                    .iter()
                    .rposition(|o| o.region_key() == region)
                    .map_or(s.zones.len(), |p| p + 1),
            };
            s.zones.insert(at, z);
        }
        Drag::Region(r) => {
            if r == region {
                return None;
            }
            let mut order = region_order(s);
            let from = order.iter().position(|x| *x == r)?;
            let moving = order.remove(from);
            let to = order
                .iter()
                .position(|x| x == region)
                .unwrap_or(order.len());
            order.insert(to, moving);
            s.region_order = order;
        }
    }
    let now = rows(s);
    let first = now.iter().zip(&was).position(|(a, b)| a != b)?;
    Some(now[first].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(tz: &str, region: &str, id: u64) -> Zone {
        Zone {
            region: region.into(),
            id,
            ..Zone::new(tz)
        }
    }

    fn labels(s: &Saved) -> Vec<String> {
        grouped(s).iter().map(|(_, z)| z.label.clone()).collect()
    }

    #[test]
    fn dragging_a_place_moves_it_and_can_change_its_region() {
        let mut s = Saved {
            zones: vec![
                place("Europe/London", "Europe", 1),
                place("Europe/Paris", "Europe", 2),
                place("Asia/Tokyo", "Asia", 3),
            ],
            ..Saved::default()
        };
        // Paris up before London: the board refreshes from Paris down.
        assert_eq!(
            move_dropped(&mut s, Drag::Place(2), "EUROPE", Some(1)),
            Some(2)
        );
        // And back down again: from where it left, London's row now.
        assert_eq!(
            move_dropped(&mut s, Drag::Place(2), "EUROPE", None),
            Some(1)
        );
        move_dropped(&mut s, Drag::Place(2), "EUROPE", Some(1));
        assert_eq!(labels(&s), ["PARIS", "LONDON", "TOKYO"]);
        // Tokyo into Europe, at its end.
        assert_eq!(
            move_dropped(&mut s, Drag::Place(3), "EUROPE", None),
            Some(3)
        );
        assert_eq!(labels(&s), ["PARIS", "LONDON", "TOKYO"]);
        assert_eq!(s.zones.iter().find(|z| z.id == 3).unwrap().region, "Europe");
        // London on itself, in a new spot of no region.
        move_dropped(&mut s, Drag::Place(1), "", Some(1));
        assert_eq!(s.zones.iter().find(|z| z.id == 1).unwrap().region, "");
    }

    #[test]
    fn dragging_a_region_moves_the_group() {
        let mut s = Saved {
            zones: vec![
                place("Europe/London", "Europe", 1),
                place("Asia/Tokyo", "Asia", 2),
                place("America/New_York", "", 3),
            ],
            ..Saved::default()
        };
        // Asia above Europe: from Tokyo, its first place, down.
        assert_eq!(
            move_dropped(&mut s, Drag::Region("ASIA".into()), "EUROPE", None),
            Some(2)
        );
        // Dropped where it already is: nothing moves, nothing refreshes.
        assert_eq!(
            move_dropped(&mut s, Drag::Place(1), "EUROPE", Some(1)),
            None
        );
        assert_eq!(labels(&s), ["TOKYO", "LONDON", "NEW YORK"]);
        move_dropped(&mut s, Drag::Region("".into()), "ASIA", None);
        assert_eq!(labels(&s), ["NEW YORK", "TOKYO", "LONDON"]);
    }

    #[test]
    fn searches_find_the_city_you_mean_first() {
        let first = |q: &str| {
            let f = find_places(q);
            let c = &f[0];
            (c.label.clone(), c.around.clone(), c.tz)
        };
        let (label, around, tz) = first("sydney");
        assert_eq!((label.as_str(), tz), ("SYDNEY", "Australia/Sydney"));
        assert!(around.contains("Australia"));
        assert_eq!(first("Manchester").2, "Europe/London");
        assert_eq!(first("sao paulo").2, "America/Sao_Paulo");
        assert_eq!(first("são paulo").2, "America/Sao_Paulo");
        assert_eq!(first("paris, france").2, "Europe/Paris");
        assert!(find_places("paris, texas")
            .iter()
            .all(|f| f.around.contains("Texas")));
        // A country lists its cities, biggest first.
        assert_eq!(first("new zealand").0, "AUCKLAND");
        // Time zones by their own names still turn up.
        assert!(find_places("europe/lond")
            .iter()
            .any(|f| f.tz == "Europe/London" && f.at.is_none()));
        assert!(find_places("").is_empty());
        assert!(find_places("qqqqzz").is_empty());
        // The board's name fits its cards.
        assert!(find_places("ciudad")
            .iter()
            .all(|f| f.label.chars().count() <= NAME_CARDS));
    }

    #[test]
    fn every_city_is_in_a_time_zone_the_clock_knows() {
        for c in crate::cities::CITIES {
            assert!(Tz::from_str(c.tz).is_ok(), "{} {}", c.name, c.tz);
            assert!(c.lat.abs() <= 90.0 && c.lon.abs() <= 180.0, "{}", c.name);
        }
    }

    #[test]
    fn moods_set_and_recognise_themselves() {
        let mut s = Saved::default();
        assert_eq!(Scene::of(&s), Some(Scene::Classic));
        for m in Scene::ALL {
            m.apply(&mut s);
            assert_eq!(Scene::of(&s), Some(m), "{}", m.name());
        }
        s.flip_speed = 2.0;
        assert_eq!(Scene::of(&s), None);
    }
}
