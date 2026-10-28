//! The clock window: the board, its layout, and the settings window.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{Datelike, FixedOffset, Timelike, Utc};
use chrono_tz::Tz;
use egui::{
    vec2, Align2, Color32, FontFamily, FontId, Rect, Sense, Ui, Vec2, ViewportCommand, ViewportId,
};
use serde::{Deserialize, Serialize};

use crate::flap::{Drum, Flap, Look};
use crate::rng::Rng;
use crate::sound::{Click, Sound};
use crate::{autostart, desktops};

/// The longest zone name a row has cards for.
pub const NAME_CARDS: usize = 12;
/// The longest region name.
const REGION_CHARS: usize = 24;

const BOARD: Color32 = Color32::from_rgb(10, 10, 10);
const RULE: Color32 = Color32::from_rgb(38, 38, 36);
const CAPTION: Color32 = Color32::from_rgb(130, 128, 118);
const REGION: Color32 = Color32::from_rgb(200, 198, 188);
const YELLOW: Color32 = Color32::from_rgb(244, 196, 48);
const WHITE: Color32 = Color32::from_rgb(236, 234, 226);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Ink {
    #[default]
    Yellow,
    White,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    /// The IANA name, such as `Europe/London`.
    pub tz: String,
    /// What the row says, up to [`NAME_CARDS`] characters.
    pub label: String,
    /// The group the city is shown in on the board; empty for none.
    #[serde(default)]
    pub region: String,
    /// Its own colour, or the board's when `None`.
    #[serde(default)]
    pub colour: Option<[u8; 3]>,
    /// Minutes ahead of UTC, when its time is set by hand rather than
    /// taken from its time zone.
    #[serde(default)]
    pub offset: Option<i32>,
}

impl Zone {
    pub fn new(tz: &str) -> Self {
        Self {
            tz: tz.to_string(),
            label: default_label(tz),
            region: String::new(),
            colour: None,
            offset: None,
        }
    }

    /// The region as the board shows it, and as zones are grouped by.
    fn region_key(&self) -> String {
        self.region.trim().to_uppercase()
    }

    /// Its offset from UTC now, in minutes: set by hand, or its time zone's.
    fn offset_now(&self) -> Option<i32> {
        use chrono::Offset;
        self.offset.or_else(|| {
            let tz = Tz::from_str(&self.tz).ok()?;
            Some(
                Utc::now()
                    .with_timezone(&tz)
                    .offset()
                    .fix()
                    .local_minus_utc()
                    / 60,
            )
        })
    }
}

/// The regions in the order the board shows them: the saved order first,
/// then any new ones in the order their first city comes. Empty is the
/// cities with no region.
fn region_order(saved: &Saved) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let used: Vec<String> = saved.zones.iter().map(Zone::region_key).collect();
    for r in saved.region_order.iter().chain(&used) {
        if used.contains(r) && !out.contains(r) {
            out.push(r.clone());
        }
    }
    out
}

/// The zones grouped by region, in region order and then list order, each
/// with the region heading it starts, if it starts one.
fn grouped(saved: &Saved) -> Vec<(Option<String>, &Zone)> {
    let mut out = Vec::new();
    for (n, region) in region_order(saved).into_iter().enumerate() {
        // Places with no region need a heading too, unless they come first,
        // or they would look part of the region above.
        let title = match region.as_str() {
            "" if n == 0 => None,
            "" => Some("OTHER".to_string()),
            r => Some(r.to_string()),
        };
        let mut first = true;
        for z in saved.zones.iter().filter(|z| z.region_key() == region) {
            let heading = if first { title.clone() } else { None };
            out.push((heading, z));
            first = false;
        }
    }
    out
}

/// The offset from UTC, in minutes, that makes the time there `local`
/// (minutes past midnight) when it is `utc` in UTC. Of the two offsets a
/// day apart that do, the one nearer `near` wins, so changing the time by
/// a little doesn't change the day.
fn offset_for(local: i32, utc: i32, near: i32) -> i32 {
    let d = (local - utc).rem_euclid(24 * 60);
    let later = d;
    let earlier = d - 24 * 60;
    let pick = if (later - near).abs() <= (earlier - near).abs() {
        later
    } else {
        earlier
    };
    pick.clamp(-(24 * 60 - 1), 24 * 60 - 1)
}

/// How often every card on the board spins round once, as a real board
/// does when it refreshes.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Refresh {
    Off,
    Quarter,
    Half,
    #[default]
    Hourly,
    /// At a random time, within the range in [`Saved::random_minutes`].
    Random,
}

impl Refresh {
    const ALL: [Refresh; 5] = [
        Refresh::Off,
        Refresh::Quarter,
        Refresh::Half,
        Refresh::Hourly,
        Refresh::Random,
    ];

    fn minutes(self) -> Option<i64> {
        match self {
            Refresh::Off => None,
            Refresh::Quarter => Some(15),
            Refresh::Half => Some(30),
            Refresh::Hourly => Some(60),
            Refresh::Random => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Refresh::Off => "Never",
            Refresh::Quarter => "Every 15 minutes",
            Refresh::Half => "Every 30 minutes",
            Refresh::Hourly => "Every hour",
            Refresh::Random => "At random",
        }
    }
}

/// What the clock remembers between runs. New fields need a default so
/// older saves still load.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub ink: Ink,
    pub h24: bool,
    pub seconds: bool,
    pub date: bool,
    pub on_top: bool,
    pub frameless: bool,
    pub scale: f32,
    pub zones: Vec<Zone>,
    pub refresh: Refresh,
    pub all_desktops: bool,
    /// The shortest and longest wait, in minutes, between random refreshes.
    pub random_minutes: (u32, u32),
    /// The clatter of the cards at start-up and on refreshes.
    pub sound: bool,
    pub volume: f32,
    /// Clatter when the time changes too, not just for shows.
    pub clock_sound: bool,
    /// Their own colours for the main time and the date, or the board's.
    pub time_colour: Option<[u8; 3]>,
    pub date_colour: Option<[u8; 3]>,
    /// The order of the regions down the board.
    pub region_order: Vec<String>,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            ink: Ink::Yellow,
            h24: true,
            seconds: false,
            date: true,
            on_top: false,
            frameless: false,
            scale: 1.0,
            zones: Vec::new(),
            refresh: Refresh::Hourly,
            all_desktops: false,
            random_minutes: (10, 60),
            sound: false,
            volume: 0.6,
            clock_sound: false,
            time_colour: None,
            date_colour: None,
            region_order: Vec::new(),
        }
    }
}

/// The name a zone gets when it is added: `America/New_York` is NEW YORK.
pub fn default_label(tz: &str) -> String {
    let city = tz.rsplit('/').next().unwrap_or(tz).replace('_', " ");
    city.to_uppercase().chars().take(NAME_CARDS).collect()
}

/// One thing in a row of the board, in board units.
#[derive(Clone)]
enum Cell {
    Card {
        drum: Drum,
        text: String,
        size: Vec2,
        font: f32,
    },
    Colon {
        size: Vec2,
    },
    Gap(f32),
}

impl Cell {
    fn width(&self) -> f32 {
        match self {
            Cell::Card { size, .. } | Cell::Colon { size } => size.x,
            Cell::Gap(w) => *w,
        }
    }
}

// Sizes in board units; the board is scaled to fit the window.
const BIG: Vec2 = vec2(66.0, 100.0);
const BIG_AMPM: Vec2 = vec2(58.0, 44.0);
const SMALL: Vec2 = vec2(22.0, 32.0);
const SMALL_AMPM: Vec2 = vec2(34.0, 32.0);
const SPACING: f32 = 0.07; // between cards, as a share of their height
const PAD: f32 = 26.0;

struct Line {
    key: String,
    cells: Vec<Cell>,
    ink: Color32,
    /// A region heading over the line, for the first city of a region.
    heading: Option<String>,
}

/// The room a region heading takes above its first city.
const HEADING: f32 = 26.0;

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn time_cells(h: u32, m: u32, s: Option<u32>, h24: bool, big: bool) -> Vec<Cell> {
    let (size, ampm_size, colon) = if big {
        (BIG, BIG_AMPM, vec2(22.0, BIG.y))
    } else {
        (SMALL, SMALL_AMPM, vec2(9.0, SMALL.y))
    };
    let card = |drum, text: String| Cell::Card {
        drum,
        text,
        size,
        font: if big { 0.94 } else { 0.86 },
    };
    let mut out = Vec::new();
    if h24 {
        out.push(card(Drum::Tens3, (h / 10).to_string()));
        out.push(card(Drum::Digit, (h % 10).to_string()));
    } else {
        let hh = match h % 12 {
            0 => 12,
            x => x,
        };
        out.push(card(Drum::Tens12, if hh >= 10 { "1" } else { " " }.into()));
        out.push(card(Drum::Digit, (hh % 10).to_string()));
    }
    for v in [Some(m), s].into_iter().flatten() {
        out.push(Cell::Colon { size: colon });
        out.push(card(Drum::Tens6, (v / 10).to_string()));
        out.push(card(Drum::Digit, (v % 10).to_string()));
    }
    if !h24 {
        out.push(Cell::Gap(size.y * 0.12));
        out.push(Cell::Card {
            drum: Drum::AmPm,
            text: if h < 12 { "AM" } else { "PM" }.into(),
            size: ampm_size,
            font: if big { 0.62 } else { 0.5 },
        });
    }
    out
}

fn text_cards(text: &str, n: usize) -> Vec<Cell> {
    let mut chars: Vec<String> = text.chars().take(n).map(|c| c.to_string()).collect();
    chars.resize(n, " ".into());
    chars
        .into_iter()
        .map(|text| Cell::Card {
            drum: Drum::Alpha,
            text,
            size: SMALL,
            font: 0.86,
        })
        .collect()
}

fn date_cells(date: &str) -> Vec<Cell> {
    date.chars()
        .map(|c| match c {
            ' ' => Cell::Gap(SMALL.x * 0.6),
            c => Cell::Card {
                drum: Drum::Alpha,
                text: c.to_string(),
                size: SMALL,
                font: 0.86,
            },
        })
        .collect()
}

fn zone_cells(label: &str, h: u32, m: u32, day: &str, h24: bool) -> Vec<Cell> {
    let mut cells = text_cards(label, NAME_CARDS);
    cells.push(Cell::Gap(18.0));
    cells.extend(time_cells(h, m, None, h24, false));
    cells.push(Cell::Gap(18.0));
    cells.extend(text_cards(day, 3));
    cells
}

fn row_width(cells: &[Cell]) -> f32 {
    let mut w = 0.0;
    for (i, c) in cells.iter().enumerate() {
        w += c.width();
        if let (Cell::Card { size, .. }, Some(Cell::Card { .. })) = (c, cells.get(i + 1)) {
            w += size.y * SPACING;
        }
    }
    w
}

fn row_height(cells: &[Cell]) -> f32 {
    cells
        .iter()
        .map(|c| match c {
            Cell::Card { size, .. } | Cell::Colon { size } => size.y,
            Cell::Gap(_) => 0.0,
        })
        .fold(0.0, f32::max)
}

pub struct ClockApp {
    saved: Saved,
    /// The cards on the board, by row.
    flaps: HashMap<String, Vec<Option<Flap>>>,
    settings_open: bool,
    search: String,
    /// The time zone picked from the search results.
    picked: Option<String>,
    /// The window state last asked of the system.
    applied_on_top: Option<bool>,
    applied_frameless: Option<bool>,
    applied_all_desktops: bool,
    desktops_error: Option<String>,
    /// Whether the clock starts at login, as the system has it.
    autostart: bool,
    autostart_error: Option<String>,
    /// The refresh interval and the slot of it the clock is in.
    refresh_slot: Option<(i64, i64)>,
    /// When the next random refresh is due, and the range it was drawn from.
    next_random: Option<(f64, (u32, u32))>,
    refresh_now: bool,
    rng: Rng,
    sound: Option<Sound>,
    sound_error: Option<String>,
    /// Whether the sound was last turned on or off.
    sound_applied: bool,
    /// When to try the sound output again after it wouldn't open.
    sound_retry: f64,
    /// The board size the window was last fitted to.
    fitted: Option<Vec2>,
    last_window: Vec2,
    resized_at: f64,
}

impl ClockApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let saved = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        Self {
            saved,
            flaps: HashMap::new(),
            settings_open: false,
            search: String::new(),
            picked: None,
            applied_on_top: None,
            applied_frameless: None,
            applied_all_desktops: false,
            desktops_error: None,
            autostart: autostart::enabled(),
            autostart_error: None,
            refresh_slot: None,
            next_random: None,
            refresh_now: false,
            rng: Rng::seeded(),
            sound: None,
            sound_error: None,
            sound_applied: false,
            sound_retry: 0.0,
            fitted: None,
            last_window: Vec2::ZERO,
            resized_at: 0.0,
        }
    }

    fn lines(&self) -> Vec<Line> {
        let now = chrono::Local::now();
        let s = &self.saved;
        let ink = self.ink();
        let mut lines = vec![Line {
            key: "main".into(),
            cells: time_cells(
                now.hour(),
                now.minute(),
                s.seconds.then(|| now.second()),
                s.h24,
                true,
            ),
            ink: s.time_colour.map_or(ink, rgb),
            heading: None,
        }];
        if s.date {
            let date = now.format("%a %d %b %Y").to_string().to_uppercase();
            lines.push(Line {
                key: "date".into(),
                cells: date_cells(&date),
                ink: s.date_colour.map_or(ink, rgb),
                heading: None,
            });
        }
        let utc = Utc::now();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (heading, z) in grouped(s) {
            // The time there: by hand, or from its time zone.
            let (h, m, day) = if let Some(off) = z.offset {
                let Some(fixed) = FixedOffset::east_opt(off * 60) else {
                    continue;
                };
                let t = utc.with_timezone(&fixed);
                (t.hour(), t.minute(), t.weekday())
            } else {
                let Ok(tz) = Tz::from_str(&z.tz) else {
                    continue;
                };
                let t = utc.with_timezone(&tz);
                (t.hour(), t.minute(), t.weekday())
            };
            let n = seen.entry(&z.tz).or_default();
            *n += 1;
            let day = day.to_string().to_uppercase();
            lines.push(Line {
                key: format!("zone:{}:{n}", z.tz),
                cells: zone_cells(&z.label.to_uppercase(), h, m, &day, s.h24),
                ink: z.colour.map_or(ink, rgb),
                heading,
            });
        }
        lines
    }

    /// The board's size in board units.
    fn board_size(lines: &[Line]) -> Vec2 {
        let w = lines
            .iter()
            .map(|l| row_width(&l.cells))
            .fold(0.0, f32::max);
        let mut h = PAD * 2.0;
        for (i, l) in lines.iter().enumerate() {
            h += row_height(&l.cells);
            if i > 0 {
                h += Self::gap_before(lines, i);
            }
        }
        vec2(w + PAD * 2.0, h)
    }

    /// The space above line `i`, with room for the rule and the column
    /// captions above the first zone.
    fn gap_before(lines: &[Line], i: usize) -> f32 {
        let heading = if lines[i].heading.is_some() {
            HEADING
        } else {
            0.0
        };
        heading + Self::rows_gap(lines, i)
    }

    fn rows_gap(lines: &[Line], i: usize) -> f32 {
        let zone = lines[i].key.starts_with("zone");
        if zone && !lines[i - 1].key.starts_with("zone") {
            56.0
        } else if zone {
            9.0
        } else {
            18.0
        }
    }

    /// The board's own colour, which anything without its own uses.
    fn ink(&self) -> Color32 {
        match self.saved.ink {
            Ink::Yellow => YELLOW,
            Ink::White => WHITE,
        }
    }

    fn look(&self) -> Look {
        Look {
            top: Color32::from_rgb(36, 36, 35),
            bottom: Color32::from_rgb(29, 29, 28),
            split: Color32::from_rgb(4, 4, 4),
            hinge: Color32::from_rgb(70, 70, 68),
            ink: self.ink(),
        }
    }

    /// Opens the sound output when the sound is turned on, and closes it
    /// when it is turned off. If it won't open, as when the clock starts at
    /// login before the sound system is up, it tries again now and then.
    fn apply_sound(&mut self, time: f64) {
        if self.saved.sound != self.sound_applied {
            self.sound_applied = self.saved.sound;
            self.sound = None;
            self.sound_error = None;
            self.sound_retry = 0.0;
        }
        if self.saved.sound && self.sound.is_none() && time >= self.sound_retry {
            match Sound::open(self.saved.volume) {
                Ok(s) => {
                    self.sound = Some(s);
                    self.sound_error = None;
                }
                Err(e) => {
                    self.sound_error = Some(e);
                    self.sound_retry = time + 15.0;
                }
            }
        }
        if let Some(s) = &self.sound {
            s.set_volume(self.saved.volume);
        }
    }

    fn apply_window(&mut self, ctx: &egui::Context) {
        if self.applied_on_top != Some(self.saved.on_top) {
            self.applied_on_top = Some(self.saved.on_top);
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if self.saved.on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        }
        if self.applied_frameless != Some(self.saved.frameless) {
            self.applied_frameless = Some(self.saved.frameless);
            ctx.send_viewport_cmd(ViewportCommand::Decorations(!self.saved.frameless));
        }
    }

    /// Fits the window to the board when the board changes shape, and
    /// remembers the size the user drags it to.
    fn fit_window(&mut self, ctx: &egui::Context, board: Vec2, window: Vec2, time: f64) {
        if self.fitted != Some(board) {
            self.fitted = Some(board);
            self.resize(ctx, board, time);
        } else if window != self.last_window && time - self.resized_at > 0.5 {
            let fit = (window.x / board.x).min(window.y / board.y);
            self.saved.scale = fit.clamp(0.3, 4.0);
        }
        self.last_window = window;
    }

    fn resize(&mut self, ctx: &egui::Context, board: Vec2, time: f64) {
        let mut size = board * self.saved.scale;
        // A long list of cities mustn't make the window taller than the
        // screen; the board shrinks to fit instead.
        if let Some(screen) = ctx.input(|i| i.viewport().monitor_size) {
            size *= (screen.x * 0.95 / size.x)
                .min(screen.y * 0.9 / size.y)
                .min(1.0);
        }
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size.round()));
        self.resized_at = time;
    }

    /// Whether the board is due a refresh: on the hour for an hourly one,
    /// on the quarter hours for every 15 minutes, by the local clock, or
    /// after a random wait.
    fn refresh_due(&mut self, time: f64) -> bool {
        if self.saved.refresh == Refresh::Random {
            self.refresh_slot = None;
            let (lo, hi) = self.saved.random_minutes;
            let range = (lo.min(hi).max(1), hi.max(lo).max(1));
            let due = match self.next_random {
                Some((at, r)) if r == range => time >= at,
                // Nothing drawn yet, or the range has changed: draw afresh.
                _ => false,
            };
            if due || self.next_random.is_none_or(|(_, r)| r != range) {
                let wait = 60.0 * self.rng.between(range.0 as f64, range.1 as f64);
                self.next_random = Some((time + wait, range));
            }
            return due;
        }
        self.next_random = None;
        let Some(every) = self.saved.refresh.minutes() else {
            self.refresh_slot = None;
            return false;
        };
        let now = chrono::Local::now();
        let minute = (now.timestamp() + i64::from(now.offset().local_minus_utc())).div_euclid(60);
        let slot = (every, minute.div_euclid(every));
        let due = self
            .refresh_slot
            .is_some_and(|(e, n)| e == every && n != slot.1);
        self.refresh_slot = Some(slot);
        due
    }

    fn board(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);
        let lines = self.lines();
        let board = Self::board_size(&lines);
        let full = ui.max_rect();
        self.fit_window(&ctx, board, full.size(), time);

        let painter = ui.painter_at(full);
        painter.rect_filled(full, 0.0, BOARD);

        let scale = (full.width() / board.x).min(full.height() / board.y);
        let origin = full.center() - board * scale / 2.0;
        let u = |x: f32, y: f32| origin + vec2(x, y) * scale;

        let base = self.look();
        let refresh = self.refresh_due(time) | std::mem::take(&mut self.refresh_now);
        let mut busy = false;
        let mut y = PAD;
        let mut first_zone = true;
        let caption = FontId::new(11.0 * scale, FontFamily::Name("flap".into()));
        let region_font = FontId::new(15.0 * scale, FontFamily::Name("flap".into()));
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                y += Self::gap_before(&lines, i);
            }
            let w = row_width(&line.cells);
            let h = row_height(&line.cells);
            let mut x = (board.x - w) / 2.0;
            let look = Look {
                ink: line.ink,
                ..base
            };
            // The rule and the column captions sit above any region heading.
            let above = if line.heading.is_some() { HEADING } else { 0.0 };
            if let Some(heading) = &line.heading {
                painter.text(
                    u(x, y - 7.0),
                    Align2::LEFT_BOTTOM,
                    heading,
                    region_font.clone(),
                    REGION,
                );
            }

            if line.key.starts_with("zone") && first_zone {
                first_zone = false;
                let rule_y = y - 40.0 - above;
                painter.hline(
                    u(PAD, rule_y).x..=u(board.x - PAD, rule_y).x,
                    u(0.0, rule_y).y,
                    (1.0_f32.max(scale), RULE),
                );
                let time_x = x + row_width(&line.cells[..NAME_CARDS + 1]) + SMALL.y * SPACING;
                let day_x = x + w - row_width(&line.cells[line.cells.len() - 3..]);
                for (cx, text) in [(x, "CITY"), (time_x, "TIME"), (day_x, "DAY")] {
                    painter.text(
                        u(cx, y - 8.0 - above),
                        Align2::LEFT_BOTTOM,
                        text,
                        caption.clone(),
                        CAPTION,
                    );
                }
            }

            // Make sure the line has a card for every cell, of the right kind.
            let want: Vec<Option<Drum>> = line
                .cells
                .iter()
                .map(|c| match c {
                    Cell::Card { drum, .. } => Some(*drum),
                    _ => None,
                })
                .collect();
            let flaps = self.flaps.entry(line.key.clone()).or_default();
            let have: Vec<Option<Drum>> =
                flaps.iter().map(|f| f.as_ref().map(Flap::drum)).collect();
            if have != want {
                *flaps = want
                    .iter()
                    .map(|d| {
                        d.map(|d| {
                            let (delay, pace) = self.rng.scatter();
                            Flap::new(d, time, delay, pace)
                        })
                    })
                    .collect();
            }

            for (k, cell) in line.cells.iter().enumerate() {
                match cell {
                    Cell::Card {
                        text, size, font, ..
                    } => {
                        let flap = flaps[k].as_mut().expect("a card for every card cell");
                        if refresh {
                            let (delay, pace) = self.rng.scatter();
                            flap.turn(text, time, delay, pace);
                        } else {
                            flap.set(text);
                        }
                        if let Some(fall) = flap.tick(time) {
                            if let Some(sound) = &mut self.sound {
                                if fall.show || self.saved.clock_sound {
                                    sound.click(Click {
                                        pan: (x + size.x / 2.0) / board.x * 2.0 - 1.0,
                                        size: size.y,
                                        land: fall.land,
                                    });
                                }
                            }
                        }
                        busy |= flap.busy(time);
                        // Cards sit on the line's baseline.
                        let r = Rect::from_min_size(u(x, y + h - size.y), *size * scale);
                        let font =
                            FontId::new(size.y * font * scale, FontFamily::Name("flap".into()));
                        flap.draw(&ctx, &painter, r, &font, &look);
                        x += size.x;
                        if let Some(Cell::Card { .. }) = line.cells.get(k + 1) {
                            x += size.y * SPACING;
                        }
                    }
                    Cell::Colon { size } => {
                        let d = size.x * 0.36;
                        for f in [0.36, 0.64] {
                            let c = u(x + size.x / 2.0, y + h - size.y + size.y * f);
                            painter.rect_filled(
                                Rect::from_center_size(c, Vec2::splat(d * scale)),
                                1.0,
                                look.ink,
                            );
                        }
                        x += size.x;
                    }
                    Cell::Gap(g) => x += g,
                }
            }
            y += h;
        }
        // Forget the cards of zones that have gone.
        self.flaps.retain(|k, _| lines.iter().any(|l| &l.key == k));

        // Drag the clock about by its face; right-click for the menu.
        let response = ui.interact(full, ui.id().with("board"), Sense::click_and_drag());
        if response.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if response.double_clicked() {
            self.settings_open = true;
        }
        response.context_menu(|ui| {
            if ui.button("Settings…").clicked() {
                self.settings_open = true;
                ui.close();
            }
            if ui.checkbox(&mut self.saved.on_top, "Keep on top").changed() {
                ui.close();
            }
            if ui
                .checkbox(&mut self.saved.frameless, "Hide title bar")
                .changed()
            {
                ui.close();
            }
            if ui
                .checkbox(&mut self.saved.all_desktops, "Show on all desktops")
                .changed()
            {
                ui.close();
            }
            if ui.checkbox(&mut self.saved.sound, "Sound").changed() {
                ui.close();
            }
            if ui.button("Refresh the board").clicked() {
                self.refresh_now = true;
                ui.close();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        });

        // A settings button shows in the corner while the pointer is over
        // the clock.
        if ui.rect_contains_pointer(full) && !self.settings_open {
            let b = Rect::from_min_size(full.right_top() + vec2(-30.0, 6.0), vec2(24.0, 24.0));
            let r = ui.put(
                b,
                egui::Button::new(egui::RichText::new("⚙").size(15.0)).frame(false),
            );
            if r.on_hover_text("Settings (or right-click the clock)")
                .clicked()
            {
                self.settings_open = true;
            }
        }

        if busy {
            ctx.request_repaint();
        } else {
            // Wake for the next second.
            let ms = 1000 - Utc::now().timestamp_subsec_millis().min(999);
            ctx.request_repaint_after(std::time::Duration::from_millis(ms as u64 + 5));
        }
    }

    fn settings(&mut self, ctx: &egui::Context) {
        let before = self.saved.scale;
        let builder = egui::ViewportBuilder::default()
            .with_title("Clock settings")
            .with_inner_size([560.0, 680.0])
            .with_min_inner_size([320.0, 380.0]);
        ctx.show_viewport_immediate(ViewportId::from_hash_of("settings"), builder, |ui, _| {
            if ui.input(|i| i.viewport().close_requested()) {
                self.settings_open = false;
            }
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.settings_ui(ui));
            });
        });
        if self.saved.scale != before {
            if let Some(board) = self.fitted {
                self.resize(ctx, board, ctx.input(|i| i.time));
            }
        }
    }

    fn settings_ui(&mut self, ui: &mut Ui) {
        let s = &mut self.saved;
        ui.heading("Display");
        egui::Grid::new("display")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Digits");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.ink, Ink::Yellow, "Yellow");
                    ui.radio_value(&mut s.ink, Ink::White, "White");
                });
                ui.end_row();
                ui.label("Colours");
                ui.horizontal(|ui| {
                    let ink = match s.ink {
                        Ink::Yellow => YELLOW,
                        Ink::White => WHITE,
                    };
                    ui.label("Time");
                    colour_choice(ui, "time", &mut s.time_colour, ink);
                    ui.add_space(8.0);
                    ui.label("Date");
                    colour_choice(ui, "date", &mut s.date_colour, ink);
                });
                ui.end_row();
                ui.label("Hours");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.h24, false, "12-hour");
                    ui.radio_value(&mut s.h24, true, "24-hour");
                });
                ui.end_row();
                ui.label("Show");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.seconds, "Seconds");
                    ui.checkbox(&mut s.date, "Date");
                });
                ui.end_row();
                ui.label("Refresh");
                egui::ComboBox::from_id_salt("refresh")
                    .selected_text(s.refresh.label())
                    .show_ui(ui, |ui| {
                        for r in Refresh::ALL {
                            ui.selectable_value(&mut s.refresh, r, r.label());
                        }
                    })
                    .response
                    .on_hover_text("Every card spins round once, like a real board refreshing");
                ui.end_row();
                if s.refresh == Refresh::Random {
                    ui.label("");
                    ui.horizontal(|ui| {
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
                    ui.end_row();
                }
                ui.label("Sound");
                ui.vertical(|ui| {
                    ui.checkbox(&mut s.sound, "Clatter when the board starts and refreshes");
                    ui.add_enabled_ui(s.sound, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Volume");
                            ui.add(egui::Slider::new(&mut s.volume, 0.0..=1.0).show_value(false));
                        });
                        ui.checkbox(&mut s.clock_sound, "Also when the time changes");
                    });
                    if let Some(e) = &self.sound_error {
                        ui.colored_label(ui.visuals().warn_fg_color, e);
                    }
                });
                ui.end_row();
                ui.label("Window");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut s.on_top, "Keep on top");
                    ui.checkbox(&mut s.frameless, "Hide title bar");
                });
                ui.end_row();
                ui.label("");
                ui.vertical(|ui| {
                    ui.checkbox(&mut s.all_desktops, "Show on all desktops");
                    if cfg!(windows) && s.all_desktops {
                        ui.label(
                            egui::RichText::new(
                                "The clock has no taskbar button while this is on.",
                            )
                            .weak()
                            .small(),
                        );
                    }
                    if let Some(e) = &self.desktops_error {
                        ui.colored_label(ui.visuals().warn_fg_color, e);
                    }
                });
                ui.end_row();
                ui.label("Start");
                ui.vertical(|ui| {
                    let mut on = self.autostart;
                    if ui.checkbox(&mut on, "Start when I log in").changed() {
                        match autostart::set(on) {
                            Ok(()) => {
                                self.autostart = on;
                                self.autostart_error = None;
                            }
                            Err(e) => self.autostart_error = Some(e),
                        }
                    }
                    if let Some(e) = &self.autostart_error {
                        ui.colored_label(ui.visuals().warn_fg_color, e);
                    }
                });
                ui.end_row();
                ui.label("Size");
                ui.add(egui::Slider::new(&mut s.scale, 0.3..=4.0).fixed_decimals(2));
                ui.end_row();
            });

        // Adding a place: search, pick, Add.
        ui.add_space(12.0);
        ui.heading("Add a place");
        let q = self.search.trim().to_lowercase().replace(' ', "_");
        let found: Vec<&'static str> = if q.is_empty() {
            Vec::new()
        } else {
            chrono_tz::TZ_VARIANTS
                .iter()
                .map(|tz| tz.name())
                .filter(|name| name.to_lowercase().contains(&q))
                .collect()
        };
        if self.picked.as_deref().is_some_and(|p| !found.contains(&p)) {
            self.picked = None;
        }
        // The one picked from the list, or else the best match.
        let choice = self.picked.clone().or(found.first().map(|f| f.to_string()));
        let mut add = false;
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search, e.g. Tokyo or New York")
                    .desired_width(240.0),
            );
            add |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let label = match &choice {
                Some(c) => format!("Add {}", default_label(c)),
                None => "Add".into(),
            };
            add |= ui
                .add_enabled(choice.is_some(), egui::Button::new(label))
                .clicked();
        });
        if let (true, Some(c)) = (add, &choice) {
            s.zones.push(Zone::new(c));
            self.search.clear();
            self.picked = None;
        }
        if !q.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("found")
                .max_height(180.0)
                .show(ui, |ui| {
                    for name in &found {
                        let on = choice.as_deref() == Some(*name);
                        if ui.selectable_label(on, *name).clicked() {
                            self.picked = Some(name.to_string());
                        }
                    }
                    if found.is_empty() {
                        ui.label(egui::RichText::new("No time zone matches.").weak());
                    }
                });
        }

        ui.add_space(12.0);
        ui.heading("Other places");
        ui.label("Listed under the main clock, grouped by region, in this order.");
        ui.add_space(4.0);
        let board_ink = match s.ink {
            Ink::Yellow => YELLOW,
            Ink::White => WHITE,
        };
        let utc = Utc::now();
        let utc_min = (utc.hour() * 60 + utc.minute()) as i32;
        let mut remove = None;
        let mut swap = None;
        let n = s.zones.len();
        for (i, z) in s.zones.iter_mut().enumerate() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut z.label)
                        .char_limit(NAME_CARDS)
                        .desired_width(120.0);
                    ui.add(edit).on_hover_text("The name shown on the board");
                    ui.label(egui::RichText::new(&z.tz).weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                        if ui
                            .add_enabled(i + 1 < n, egui::Button::new("⏷").small())
                            .on_hover_text("Move down")
                            .clicked()
                        {
                            swap = Some(i);
                        }
                        if ui
                            .add_enabled(i > 0, egui::Button::new("⏶").small())
                            .on_hover_text("Move up")
                            .clicked()
                        {
                            swap = Some(i - 1);
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Region");
                    ui.add(
                        egui::TextEdit::singleline(&mut z.region)
                            .char_limit(REGION_CHARS)
                            .desired_width(120.0)
                            .hint_text("e.g. Europe"),
                    );
                    ui.add_space(8.0);
                    ui.label("Colour");
                    colour_choice(ui, ("place", i), &mut z.colour, board_ink);
                });
                ui.horizontal(|ui| {
                    ui.label("Time");
                    let mut by_hand = z.offset.is_some();
                    let from_zone = ui.radio_value(&mut by_hand, false, "From its time zone");
                    let manual = ui.radio_value(&mut by_hand, true, "Set by hand");
                    if manual.changed() && by_hand {
                        // Start from the time it is there now, so nothing jumps.
                        z.offset = Some(z.offset_now().unwrap_or(0));
                    }
                    if from_zone.changed() && !by_hand {
                        z.offset = None;
                    }
                });
                if let Some(off) = z.offset {
                    ui.horizontal(|ui| {
                        ui.add_space(36.0);
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
            });
        }
        if let Some(i) = swap {
            s.zones.swap(i, i + 1);
        }
        if let Some(i) = remove {
            s.zones.remove(i);
        }
        if s.zones.is_empty() {
            ui.label(egui::RichText::new("None yet.").weak());
        }

        // Regions, in the order they go down the board.
        s.region_order = region_order(s);
        if s.region_order.iter().any(|r| !r.is_empty()) {
            ui.add_space(12.0);
            ui.heading("Regions");
            ui.label("Their order down the board.");
            let mut swap = None;
            let n = s.region_order.len();
            egui::Grid::new("regions")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (i, r) in s.region_order.iter().enumerate() {
                        if r.is_empty() {
                            ui.label(egui::RichText::new("Places with no region").italics())
                                .on_hover_text("Shown under OTHER, unless they come first");
                        } else {
                            ui.label(r);
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(i > 0, egui::Button::new("⏶").small())
                                .clicked()
                            {
                                swap = Some(i - 1);
                            }
                            if ui
                                .add_enabled(i + 1 < n, egui::Button::new("⏷").small())
                                .clicked()
                            {
                                swap = Some(i);
                            }
                        });
                        ui.end_row();
                    }
                });
            if let Some(i) = swap {
                s.region_order.swap(i, i + 1);
            }
        }
    }
}

impl eframe::App for ClockApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_window(&ctx);
        self.apply_sound(ctx.input(|i| i.time));
        // Window managers only take these requests once the window is up.
        let on = self.saved.all_desktops;
        if ctx.input(|i| i.time) > 1.0 {
            let changed = self.applied_all_desktops != on;
            self.applied_all_desktops = on;
            if let Err(e) = desktops::apply(frame, on, changed) {
                self.desktops_error = Some(e);
            } else if changed {
                self.desktops_error = None;
            }
        }
        self.board(ui);
        if self.settings_open {
            self.settings(&ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.saved);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        BOARD.to_normalized_gamma_f32()
    }
}

/// A colour button for something that can have its own colour, with a box
/// to type or paste a colour into, and a way back to the board's colour.
/// The picker's own copy button gives "R, G, B", which the box takes too.
fn colour_choice(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    value: &mut Option<[u8; 3]>,
    board: Color32,
) {
    let mut c = value.unwrap_or([board.r(), board.g(), board.b()]);
    if egui::color_picker::color_edit_button_srgb(ui, &mut c).changed() {
        *value = Some(c);
    }
    // While the box is being typed in, it keeps what was typed; otherwise
    // it shows the colour.
    let id = ui.id().with(&salt).with("hex");
    let mut text = ui
        .data_mut(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| hex(value.unwrap_or(c)));
    let r = ui
        .add(
            egui::TextEdit::singleline(&mut text)
                .id_salt((&salt, "hex box"))
                .desired_width(66.0)
                .font(egui::TextStyle::Monospace)
                .hint_text("#RRGGBB"),
        )
        .on_hover_text("Type or paste a colour: #RRGGBB, #RGB, or R, G, B");
    if r.changed() {
        if let Some(p) = parse_colour(&text) {
            *value = Some(p);
        }
    }
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text));
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
    }
    if value.is_some()
        && ui
            .small_button("Reset")
            .on_hover_text("Back to the board's colour")
            .clicked()
    {
        *value = None;
    }
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

/// A colour as people write it: `#RRGGBB` or `#RGB`, with or without the
/// `#`, or three numbers from 0 to 255, as in `255, 136, 0` or
/// `rgb(255, 136, 0)`.
fn parse_colour(text: &str) -> Option<[u8; 3]> {
    let t = text.trim();
    if t.contains(',') {
        let nums: Vec<u8> = t
            .trim_start_matches("rgb")
            .trim_matches(|c| c == '(' || c == ')')
            .split(',')
            .map(|n| n.trim().parse().ok())
            .collect::<Option<_>>()?;
        return <[u8; 3]>::try_from(nums).ok();
    }
    let h = t.trim_start_matches('#');
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match h.len() {
        6 => Some([byte(&h[0..2])?, byte(&h[2..4])?, byte(&h[4..6])?]),
        3 => {
            let d = |i: usize| byte(&h[i..=i]).map(|v| v * 17);
            Some([d(0)?, d(1)?, d(2)?])
        }
        _ => None,
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "barlow".into(),
        egui::FontData::from_static(include_bytes!("../assets/BarlowCondensed-SemiBold.ttf"))
            .into(),
    );
    fonts
        .families
        .insert(FontFamily::Name("flap".into()), vec!["barlow".into()]);
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(cells: &[Cell]) -> String {
        cells
            .iter()
            .map(|c| match c {
                Cell::Card { text, .. } => text.clone(),
                Cell::Colon { .. } => ":".into(),
                Cell::Gap(_) => "_".into(),
            })
            .collect()
    }

    #[test]
    fn hours_read_right() {
        assert_eq!(texts(&time_cells(9, 5, None, true, true)), "09:05");
        assert_eq!(texts(&time_cells(21, 5, Some(7), true, true)), "21:05:07");
        assert_eq!(texts(&time_cells(0, 30, None, false, true)), "12:30_AM");
        assert_eq!(texts(&time_cells(13, 0, None, false, true)), " 1:00_PM");
        assert_eq!(texts(&time_cells(12, 0, None, false, true)), "12:00_PM");
    }

    #[test]
    fn labels_come_from_the_city() {
        assert_eq!(default_label("America/New_York"), "NEW YORK");
        assert_eq!(
            default_label("America/Argentina/Buenos_Aires"),
            "BUENOS AIRES"
        );
        assert_eq!(default_label("UTC"), "UTC");
    }

    #[test]
    fn zone_rows_line_up() {
        let a = zone_cells("LONDON", 9, 5, "MON", true);
        let b = zone_cells("A NAME THAT IS FAR TOO LONG", 21, 30, "TUE", true);
        assert_eq!(row_width(&a), row_width(&b));
    }

    #[test]
    fn old_saves_still_load() {
        let s: Saved = ron::from_str("(h24: false)").unwrap();
        assert!(!s.h24 && s.date && s.zones.is_empty());
        // A place saved before regions, colours and hand-set times.
        let s: Saved = ron::from_str("(zones: [(tz: \"Asia/Tokyo\", label: \"TOKYO\")])").unwrap();
        assert_eq!(s.zones[0], Zone::new("Asia/Tokyo"));
    }

    fn place(tz: &str, region: &str) -> Zone {
        Zone {
            region: region.into(),
            ..Zone::new(tz)
        }
    }

    #[test]
    fn places_group_by_region_in_order() {
        let mut s = Saved {
            zones: vec![
                place("Europe/London", "Europe"),
                place("Asia/Tokyo", "asia "),
                place("Europe/Paris", "EUROPE"),
                place("UTC", ""),
            ],
            ..Saved::default()
        };
        let names = |s: &Saved| -> Vec<(Option<String>, String)> {
            grouped(s)
                .into_iter()
                .map(|(h, z)| (h, z.tz.clone()))
                .collect()
        };
        let europe = Some("EUROPE".to_string());
        let asia = Some("ASIA".to_string());
        assert_eq!(
            names(&s),
            [
                (europe.clone(), "Europe/London".into()),
                (None, "Europe/Paris".into()),
                (asia.clone(), "Asia/Tokyo".into()),
                (Some("OTHER".into()), "UTC".into()),
            ]
        );
        // A saved order wins; regions it leaves out follow.
        s.region_order = vec!["".into(), "ASIA".into(), "GONE".into()];
        assert_eq!(region_order(&s), ["", "ASIA", "EUROPE"]);
        assert_eq!(names(&s)[0], (None, "UTC".into()));
        assert_eq!(names(&s)[1], (asia, "Asia/Tokyo".into()));
    }

    #[test]
    fn colours_paste_in_any_usual_form() {
        let orange = Some([255, 136, 0]);
        assert_eq!(parse_colour("#FF8800"), orange);
        assert_eq!(parse_colour(" ff8800 "), orange);
        assert_eq!(parse_colour("#f80"), orange);
        // What the picker's copy button gives.
        assert_eq!(parse_colour("255, 136, 0"), orange);
        assert_eq!(parse_colour("rgb(255,136,0)"), orange);
        assert_eq!(parse_colour("#FF88"), None);
        assert_eq!(parse_colour("256, 0, 0"), None);
        assert_eq!(parse_colour("1, 2"), None);
        assert_eq!(parse_colour("orange"), None);
        assert_eq!(hex([255, 136, 0]), "#FF8800");
    }

    #[test]
    fn hand_set_times_keep_their_day() {
        // 10:00 UTC; it's 15:30 there: UTC+05:30.
        assert_eq!(offset_for(15 * 60 + 30, 600, 0), 330);
        // 23:00 there could be UTC+13 or UTC-11; near -10 it's -11.
        assert_eq!(offset_for(23 * 60, 600, -600), -660);
        assert_eq!(offset_for(23 * 60, 600, 720), 780);
    }
}
