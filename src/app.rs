//! The clock window: the board, its layout, and the settings window.

use std::collections::HashMap;
use std::str::FromStr;

use chrono::{Datelike, FixedOffset, Timelike, Utc};
use chrono_tz::Tz;
use egui::{
    vec2, Align2, Color32, FontFamily, FontId, Rect, Sense, Ui, Vec2, ViewportCommand, ViewportId,
};
use serde::{Deserialize, Serialize};

use crate::alarm::{self, Alarm};
use crate::flap::{Drum, Flap, Look};
use crate::lighting::{Failures, Lighting, Mood, Replace};
use crate::rng::Rng;
use crate::sky::{sky, Sky};
use crate::sound::{Click, Sound};
use crate::{autostart, desktops, places, sky};

mod settings;

/// The longest zone name a row has cards for.
pub const NAME_CARDS: usize = 12;
/// The longest region name.
const REGION_CHARS: usize = 24;

const BOARD: Color32 = Color32::from_rgb(10, 10, 10);
const RULE: Color32 = Color32::from_rgb(38, 38, 36);
const CAPTION: Color32 = Color32::from_rgb(130, 128, 118);
const REGION: Color32 = Color32::from_rgb(200, 198, 188);
const YELLOW: Color32 = Color32::from_rgb(244, 196, 48);
/// The letters on light cards.
const DARK_INK: Color32 = Color32::from_rgb(30, 30, 28);

/// The board's colours around the cards.
struct Palette {
    board: Color32,
    rule: Color32,
    caption: Color32,
    region: Color32,
}

impl Saved {
    /// The colour everything without its own colour is in.
    fn default_ink(&self) -> Color32 {
        match (self.light_board, self.ink) {
            (true, _) => DARK_INK,
            (false, Ink::Yellow) => YELLOW,
            (false, Ink::White) => WHITE,
        }
    }

    fn palette(&self) -> Palette {
        if self.light_board {
            Palette {
                board: Color32::from_rgb(206, 203, 195),
                rule: Color32::from_rgb(178, 174, 165),
                caption: Color32::from_rgb(98, 95, 88),
                region: Color32::from_rgb(52, 50, 46),
            }
        } else {
            Palette {
                board: BOARD,
                rule: RULE,
                caption: CAPTION,
                region: REGION,
            }
        }
    }
}
const WHITE: Color32 = Color32::from_rgb(236, 234, 226);

/// Light or dark, for the menus and the settings window.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

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
    /// Its own number, so alarms can follow it wherever it moves.
    #[serde(default)]
    pub id: u64,
}

impl Zone {
    pub fn new(tz: &str) -> Self {
        Self {
            tz: tz.to_string(),
            label: default_label(tz),
            region: String::new(),
            colour: None,
            offset: None,
            id: 0,
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
    /// The sky card, morning, day, evening or night: at the end of each
    /// place's row, and after the main time.
    pub sky_places: bool,
    pub sky_main: bool,
    /// The menus' and settings window's theme.
    pub menus: Theme,
    /// White cards with dark letters, instead of black with yellow or white.
    pub light_board: bool,
    /// How fast the flaps fall, 1 being the usual: from a quarter as fast
    /// to four times as fast.
    pub flip_speed: f32,
    /// How much the cards stand out from the board, from 0 (the classic
    /// look) to 1: lighter black cards with brighter pins, or on white
    /// cards a darker split and pins.
    pub card_brightness: f32,
    /// Lamps above the board, and how often their bulbs fail.
    pub lighting: Mood,
    pub bulbs: Failures,
    /// Whether dead bulbs are replaced for you, or by clicking them; and,
    /// by hand, the ones that are out, by their place from the left.
    pub replace: Replace,
    pub dead_bulbs: Vec<usize>,
    /// Their own colours for the main time and the date, or the board's.
    pub time_colour: Option<[u8; 3]>,
    pub date_colour: Option<[u8; 3]>,
    /// The order of the regions down the board.
    pub region_order: Vec<String>,
    pub alarms: Vec<Alarm>,
    /// The chime's own volume, which rings whether the clatter is on or not.
    pub alarm_volume: f32,
}

impl Saved {
    /// Gives every place and alarm a number of its own: ones saved before
    /// they had numbers, and any that somehow share one.
    fn number(&mut self) {
        let mut seen = std::collections::HashSet::new();
        let mut next = self.zones.iter().map(|z| z.id).max().unwrap_or(0) + 1;
        for z in &mut self.zones {
            if z.id == 0 || !seen.insert(z.id) {
                z.id = next;
                seen.insert(next);
                next += 1;
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut next = self.alarms.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        for a in &mut self.alarms {
            if a.id == 0 || !seen.insert(a.id) {
                a.id = next;
                seen.insert(next);
                next += 1;
            }
        }
    }

    fn zone(&self, id: Option<u64>) -> Option<&Zone> {
        id.and_then(|id| self.zones.iter().find(|z| z.id == id))
    }

    /// The time now at a place, or your own time for `None` (or a place
    /// that has gone).
    fn time_at(&self, place: Option<u64>) -> chrono::NaiveDateTime {
        let utc = Utc::now();
        match self.zone(place) {
            Some(z) => match z.offset {
                Some(off) => FixedOffset::east_opt(off * 60)
                    .map(|f| utc.with_timezone(&f).naive_local())
                    .unwrap_or_else(|| utc.naive_utc()),
                None => match Tz::from_str(&z.tz) {
                    Ok(tz) => utc.with_timezone(&tz).naive_local(),
                    Err(_) => chrono::Local::now().naive_local(),
                },
            },
            None => chrono::Local::now().naive_local(),
        }
    }
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
            sky_places: false,
            sky_main: false,
            menus: Theme::System,
            light_board: false,
            flip_speed: 1.0,
            card_brightness: 0.0,
            lighting: Mood::Off,
            bulbs: Failures::Sometimes,
            replace: Replace::Automatically,
            dead_bulbs: Vec::new(),
            time_colour: None,
            date_colour: None,
            region_order: Vec::new(),
            alarms: Vec::new(),
            alarm_volume: 0.8,
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
    /// A small caption under the line.
    note: Option<&'static str>,
    /// Cards that are buttons: which cells, and what they do.
    actions: Vec<(std::ops::Range<usize>, Act)>,
}

/// What a button on the board does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Snooze,
    Stop,
}

/// The room a caption takes under its line.
const NOTE: f32 = 16.0;

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

fn zone_cells(label: &str, h: u32, m: u32, day: &str, h24: bool, sky: Option<Sky>) -> Vec<Cell> {
    let mut cells = text_cards(label, NAME_CARDS);
    cells.push(Cell::Gap(18.0));
    cells.extend(time_cells(h, m, None, h24, false));
    cells.push(Cell::Gap(18.0));
    cells.extend(text_cards(day, 3));
    if let Some(sky) = sky {
        cells.push(Cell::Gap(10.0));
        cells.push(sky_card(sky, SKY_SMALL));
    }
    cells
}

// The sky card is wider than a letter's, for its picture.
const SKY_SMALL: Vec2 = vec2(30.0, 32.0);
const SKY_BIG: Vec2 = vec2(90.0, 100.0);

fn sky_card(sky: Sky, size: Vec2) -> Cell {
    Cell::Card {
        drum: Drum::Sky,
        text: sky.symbol().into(),
        size,
        font: 1.0,
    }
}

/// Whether a row ends with a sky card.
fn has_sky(cells: &[Cell]) -> bool {
    matches!(
        cells.last(),
        Some(Cell::Card {
            drum: Drum::Sky,
            ..
        })
    )
}

/// An alarm that's ringing or snoozing.
struct Ringing {
    id: u64,
    label: String,
    /// What it's for, shown where NOW would be.
    reason: String,
    /// The time on the board: the alarm's, or when a snooze ends.
    shown: (u32, u32),
    place: Option<u64>,
    /// When it started ringing (again), in seconds since 1970.
    since: i64,
    snoozed_until: Option<i64>,
    /// Its own colour, or the board's.
    colour: Option<[u8; 3]>,
}

/// The alarm's row, as a board announces a flight: its label, its time,
/// and its reason, or NOW if it has none, or DELAYED while it snoozes. The
/// last cards flip to their word once and stay there.
fn alarm_cells(r: &Ringing, h24: bool) -> Vec<Cell> {
    let reason = r.reason.trim();
    let remark = match r.snoozed_until {
        Some(_) => "DELAYED",
        None if reason.is_empty() => "NOW",
        None => reason,
    };
    // The same number of cards snoozed or not, so the row keeps its shape.
    let room = reason.chars().count().clamp(7, NAME_CARDS);
    let label = r.label.trim();
    let mut cells = text_cards(label, label.chars().count().clamp(1, NAME_CARDS));
    cells.push(Cell::Gap(18.0));
    cells.extend(time_cells(r.shown.0, r.shown.1, None, h24, false));
    cells.push(Cell::Gap(18.0));
    cells.extend(text_cards(remark, room));
    cells
}

/// The alarm's buttons: SNOOZE and STOP, or only STOP while it snoozes.
fn alarm_buttons(snoozed: bool) -> (Vec<Cell>, Vec<(std::ops::Range<usize>, Act)>) {
    let mut cells = Vec::new();
    let mut actions = Vec::new();
    if !snoozed {
        cells.extend(text_cards("SNOOZE", 6));
        actions.push((0..6, Act::Snooze));
        cells.push(Cell::Gap(30.0));
    }
    let start = cells.len();
    cells.extend(text_cards("STOP", 4));
    actions.push((start..start + 4, Act::Stop));
    (cells, actions)
}

/// `cells` made smaller, if need be, to fit `width`, so that a row that
/// comes and goes, like an alarm's, never changes the board's width (and
/// so the window's).
fn fit(cells: Vec<Cell>, width: f32) -> Vec<Cell> {
    let k = (width / row_width(&cells)).min(1.0);
    if k >= 1.0 {
        return cells;
    }
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
    /// Where the settings window opens: next to the clock, worked out when
    /// it opens.
    settings_at: Option<egui::Pos2>,
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
    /// Where you are, from your system's time zone, for the main time's sky.
    home: Option<(f64, f64)>,
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
    /// Alarms ringing or snoozing, the first one on the board.
    ringing: Vec<Ringing>,
    /// From when, by its place's time, each alarm may ring: when the clock
    /// started or the alarm was last changed, so a time just gone doesn't
    /// ring it.
    armed: HashMap<u64, chrono::NaiveDateTime>,
    beer_open: bool,
    /// The settings page showing, and the place and alarm opened up there.
    page: settings::Page,
    open_place: Option<u64>,
    open_alarm: Option<u64>,
    /// The settings window's preview of the board, by row and card.
    preview_flaps: HashMap<(u8, usize), Flap>,
    lighting: Lighting,
    /// When to ring the chime again.
    next_chime: f64,
    /// The board size the window was last fitted to.
    fitted: Option<Vec2>,
    last_window: Vec2,
    resized_at: f64,
}

impl ClockApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let mut saved: Saved = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        saved.number();
        // Bulbs that were out, to be replaced by hand, still are.
        let mut lighting = Lighting::new(Rng::seeded());
        if saved.replace == Replace::ByHand {
            lighting.restore(&saved.dead_bulbs);
        }
        Self {
            saved,
            flaps: HashMap::new(),
            settings_open: false,
            settings_at: None,
            search: String::new(),
            picked: None,
            applied_on_top: None,
            applied_frameless: None,
            applied_all_desktops: false,
            desktops_error: None,
            autostart: autostart::enabled(),
            home: iana_time_zone::get_timezone()
                .ok()
                .and_then(|tz| places::coords(&tz)),
            autostart_error: None,
            refresh_slot: None,
            next_random: None,
            refresh_now: false,
            rng: Rng::seeded(),
            sound: None,
            sound_error: None,
            sound_applied: false,
            sound_retry: 0.0,
            ringing: Vec::new(),
            armed: HashMap::new(),
            beer_open: false,
            page: settings::Page::default(),
            open_place: None,
            open_alarm: None,
            preview_flaps: HashMap::new(),
            lighting,
            next_chime: 0.0,
            fitted: None,
            last_window: Vec2::ZERO,
            resized_at: 0.0,
        }
    }

    fn lines(&self) -> Vec<Line> {
        let now = chrono::Local::now();
        let s = &self.saved;
        let ink = self.ink();
        let mut main = time_cells(
            now.hour(),
            now.minute(),
            s.seconds.then(|| now.second()),
            s.h24,
            true,
        );
        if s.sky_main {
            let local = sky::minutes(now.naive_local());
            main.push(Cell::Gap(BIG.y * 0.16));
            main.push(sky_card(sky(Utc::now(), self.home, local), SKY_BIG));
        }
        let mut lines = vec![Line {
            key: "main".into(),
            cells: main,
            ink: s.time_colour.map_or(ink, rgb),
            heading: None,
            note: None,
            actions: Vec::new(),
        }];
        if s.date {
            let date = now.format("%a %d %b %Y").to_string().to_uppercase();
            lines.push(Line {
                key: "date".into(),
                cells: date_cells(&date),
                ink: s.date_colour.map_or(ink, rgb),
                heading: None,
                note: None,
                actions: Vec::new(),
            });
        }
        let utc = Utc::now();
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for (heading, z) in grouped(s) {
            // The time there: by hand, or from its time zone. A place set by
            // hand has no location, so its sky goes by its clock.
            let (h, m, day, at) = if let Some(off) = z.offset {
                let Some(fixed) = FixedOffset::east_opt(off * 60) else {
                    continue;
                };
                let t = utc.with_timezone(&fixed);
                (t.hour(), t.minute(), t.weekday(), None)
            } else {
                let Ok(tz) = Tz::from_str(&z.tz) else {
                    continue;
                };
                let t = utc.with_timezone(&tz);
                (t.hour(), t.minute(), t.weekday(), places::coords(&z.tz))
            };
            let sky = s.sky_places.then(|| sky(utc, at, (h * 60 + m) as i64));
            let n = seen.entry(&z.tz).or_default();
            *n += 1;
            let day = day.to_string().to_uppercase();
            lines.push(Line {
                key: format!("zone:{}:{n}", z.tz),
                cells: zone_cells(&z.label.to_uppercase(), h, m, &day, s.h24, sky),
                ink: z.colour.map_or(ink, rgb),
                heading,
                note: None,
                actions: Vec::new(),
            });
        }
        // The alarm's row goes under the main time, no wider than the rest.
        if let Some(r) = self.ringing.first() {
            let ink = r.colour.map_or(ink, rgb);
            let width = lines
                .iter()
                .map(|l| row_width(&l.cells))
                .fold(0.0, f32::max);
            lines.insert(
                1,
                Line {
                    key: "alarm".into(),
                    cells: fit(alarm_cells(r, s.h24), width),
                    ink,
                    heading: None,
                    note: None,
                    actions: Vec::new(),
                },
            );
            // And under it, flap cards to press: SNOOZE and STOP.
            let (cells, actions) = alarm_buttons(r.snoozed_until.is_some());
            lines.insert(
                2,
                Line {
                    key: "buttons".into(),
                    cells: fit(cells, width),
                    ink,
                    heading: None,
                    note: None,
                    actions,
                },
            );
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
        let note = if lines[i - 1].note.is_some() {
            NOTE
        } else {
            0.0
        };
        heading + note + Self::rows_gap(lines, i)
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
        self.saved.default_ink()
    }

    fn look(&self) -> Look {
        // Card brightness moves each colour from the classic look towards a
        // brighter one; the split keeps its contrast, so the detail shows.
        let k = self.saved.card_brightness.clamp(0.0, 1.0);
        let mix = |a: [u8; 3], b: [u8; 3]| {
            let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * k).round() as u8;
            Color32::from_rgb(m(0), m(1), m(2))
        };
        if self.saved.light_board {
            Look {
                top: Color32::from_rgb(244, 242, 236),
                bottom: Color32::from_rgb(232, 229, 221),
                split: mix([150, 147, 140], [88, 86, 80]),
                hinge: mix([178, 174, 166], [112, 109, 102]),
                ink: self.ink(),
            }
        } else {
            Look {
                top: mix([36, 36, 35], [92, 92, 89]),
                bottom: mix([29, 29, 28], [76, 76, 73]),
                split: Color32::from_rgb(4, 4, 4),
                hinge: mix([70, 70, 68], [176, 176, 170]),
                ink: self.ink(),
            }
        }
    }

    /// Opens the sound output when the sound is turned on, and closes it
    /// when it is turned off. If it won't open, as when the clock starts at
    /// login before the sound system is up, it tries again now and then.
    fn apply_sound(&mut self, time: f64) {
        // The output is open for the clatter, and for alarms, which chime
        // whether the clatter is on or not.
        let want =
            self.saved.sound || self.saved.alarms.iter().any(|a| a.on) || !self.ringing.is_empty();
        if want != self.sound_applied {
            self.sound_applied = want;
            self.sound = None;
            self.sound_error = None;
            self.sound_retry = 0.0;
        }
        if want && self.sound.is_none() && time >= self.sound_retry {
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

    /// Starts alarms that are due, wakes snoozed ones, rings the chime and
    /// gives up on an alarm nobody has answered.
    fn alarms(&mut self, time: f64) {
        let now = Utc::now().timestamp();
        let mut started = Vec::new();
        for i in 0..self.saved.alarms.len() {
            let place = self.saved.alarms[i].place;
            let at = self.saved.time_at(place);
            let since = *self.armed.entry(self.saved.alarms[i].id).or_insert(at);
            if let Some(day) = self.saved.alarms[i].due(at, since) {
                self.saved.alarms[i].rung(day);
                started.push(self.saved.alarms[i].id);
            }
        }
        for id in started {
            self.ring(id);
        }
        for r in &mut self.ringing {
            if r.snoozed_until.is_some_and(|until| now >= until) {
                r.snoozed_until = None;
                r.since = now;
                self.refresh_now = true;
                self.next_chime = 0.0;
            }
        }
        self.ringing
            .retain(|r| r.snoozed_until.is_some() || now - r.since < alarm::RING_FOR * 60);
        if self.ringing.iter().any(|r| r.snoozed_until.is_none()) && time >= self.next_chime {
            // Until the sound output is open, the chime waits for it.
            if let Some(sound) = &mut self.sound {
                sound.chime(self.saved.alarm_volume);
                self.next_chime = time + 4.0;
            }
        }
    }

    /// Rings alarm `id` now: it goes on the board with a clatter.
    fn ring(&mut self, id: u64) {
        let Some(a) = self.saved.alarms.iter().find(|a| a.id == id) else {
            return;
        };
        let r = Ringing {
            id,
            label: a.label.to_uppercase(),
            reason: a.reason.to_uppercase(),
            shown: (a.hour, a.minute),
            place: a.place,
            since: Utc::now().timestamp(),
            snoozed_until: None,
            colour: a.colour,
        };
        self.ringing.retain(|x| x.id != id);
        self.ringing.push(r);
        self.refresh_now = true;
        self.next_chime = 0.0;
    }

    /// Snoozes every alarm that's ringing for nine minutes; the board shows
    /// when it will ring again, by its place's time.
    fn snooze(&mut self) {
        let until = Utc::now().timestamp() + alarm::SNOOZE * 60;
        for i in 0..self.ringing.len() {
            if self.ringing[i].snoozed_until.is_none() {
                let then = self.saved.time_at(self.ringing[i].place)
                    + chrono::Duration::minutes(alarm::SNOOZE);
                self.ringing[i].snoozed_until = Some(until);
                self.ringing[i].shown = (then.hour(), then.minute());
            }
        }
    }

    fn board(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);
        let lines = self.lines();
        let board = Self::board_size(&lines);
        let full = ui.max_rect();
        self.fit_window(&ctx, board, full.size(), time);

        let painter = ui.painter_at(full);
        let pal = self.saved.palette();
        painter.rect_filled(full, 0.0, pal.board);

        let scale = (full.width() / board.x).min(full.height() / board.y);
        let origin = full.center() - board * scale / 2.0;
        let u = |x: f32, y: f32| origin + vec2(x, y) * scale;

        let base = self.look();
        let speed = self.saved.flip_speed.clamp(0.25, 4.0) as f64;
        let refresh = self.refresh_due(time) | std::mem::take(&mut self.refresh_now);
        let mut busy = false;
        // The lamps above the board: one for every 230 units or so.
        let mood = self.saved.lighting;
        let shadow = mood.shadow();
        if mood != Mood::Off {
            let lamps = ((board.x / 230.0).round() as usize).max(2);
            busy |= self
                .lighting
                .update(lamps, time, self.saved.bulbs, self.saved.replace);
            if self.saved.replace == Replace::ByHand {
                let dead = self.lighting.dead();
                if dead != self.saved.dead_bulbs {
                    self.saved.dead_bulbs = dead;
                }
            } else if !self.saved.dead_bulbs.is_empty() {
                self.saved.dead_bulbs.clear();
            }
        }
        let mut y = PAD;
        let mut first_zone = true;
        // Where the buttons on the board are, to click.
        let mut buttons: Vec<(Rect, Act, Color32)> = Vec::new();
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
                    pal.region,
                );
            }

            if line.key.starts_with("zone") && first_zone {
                first_zone = false;
                let rule_y = y - 40.0 - above;
                painter.hline(
                    u(PAD, rule_y).x..=u(board.x - PAD, rule_y).x,
                    u(0.0, rule_y).y,
                    (1.0_f32.max(scale), pal.rule),
                );
                let time_x = x + row_width(&line.cells[..NAME_CARDS + 1]) + SMALL.y * SPACING;
                let tail = if has_sky(&line.cells) { 5 } else { 3 };
                let day_x = x + w - row_width(&line.cells[line.cells.len() - tail..]);
                for (cx, text) in [(x, "CITY"), (time_x, "TIME"), (day_x, "DAY")] {
                    painter.text(
                        u(cx, y - 8.0 - above),
                        Align2::LEFT_BOTTOM,
                        text,
                        caption.clone(),
                        pal.caption,
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
                            Flap::new(d, time, delay / speed, pace)
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
                            flap.turn(text, time, delay / speed, pace);
                        } else {
                            flap.set(text);
                        }
                        if let Some(fall) = flap.tick(time, speed) {
                            if let (true, Some(sound)) = (self.saved.sound, &mut self.sound) {
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
                        // Lit from above, each card throws a soft shadow down
                        // onto the board behind it.
                        if shadow > 0.0 {
                            let h = r.height();
                            painter.add(
                                egui::epaint::RectShape::filled(
                                    r.translate(vec2(0.0, h * 0.07)),
                                    (h * 0.07) as u8,
                                    Color32::from_black_alpha((shadow * 170.0) as u8),
                                )
                                .with_blur_width(h * 0.22),
                            );
                        }
                        flap.draw(&ctx, &painter, r, &font, &look);
                        for (range, act) in &line.actions {
                            if range.contains(&k) {
                                match buttons.iter_mut().find(|(_, a, _)| a == act) {
                                    Some((rect, _, _)) => *rect = rect.union(r),
                                    None => buttons.push((r, *act, line.ink)),
                                }
                            }
                        }
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
            if let Some(note) = line.note {
                painter.text(
                    u(board.x / 2.0, y + 5.0),
                    Align2::CENTER_TOP,
                    note,
                    caption.clone(),
                    pal.caption,
                );
            }
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
        // The lamps' light over everything.
        self.lighting.draw(&painter, full, mood);
        // A dead bulb, to be replaced by hand: click it.
        let dead_lamp = if mood != Mood::Off && self.saved.replace == Replace::ByHand {
            ctx.input(|i| i.pointer.hover_pos())
                .and_then(|p| self.lighting.lamp_at(full, p))
                .filter(|i| self.saved.dead_bulbs.contains(i))
        } else {
            None
        };
        if dead_lamp.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            response
                .clone()
                .on_hover_text_at_pointer("Click to put in a new bulb");
        }
        // The buttons light up under the pointer, like a lit push-button.
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        for (rect, _, ink) in &buttons {
            let lit = pointer.is_some_and(|p| rect.expand(4.0).contains(p));
            let colour = if lit { *ink } else { ink.gamma_multiply(0.35) };
            painter.rect_stroke(
                rect.expand(4.0 * scale.max(0.5)),
                3.0,
                egui::Stroke::new(1.5 * scale.max(0.6), colour),
                egui::StrokeKind::Outside,
            );
        }
        // A ringing alarm: STOP stops it, SNOOZE or anywhere else on the
        // board snoozes it, and so does the menu; Esc stops it.
        let replaced =
            response.clicked() && dead_lamp.is_some_and(|i| self.lighting.replace(i, time));
        if replaced {
            ctx.request_repaint();
        }
        if !self.ringing.is_empty() {
            if response.clicked() && !replaced {
                let at = response.interact_pointer_pos();
                let hit = buttons
                    .iter()
                    .find(|(rect, _, _)| at.is_some_and(|p| rect.expand(4.0).contains(p)))
                    .map(|(_, act, _)| *act);
                match hit {
                    Some(Act::Stop) => self.ringing.clear(),
                    _ => self.snooze(),
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.ringing.clear();
            }
        }
        response.context_menu(|ui| {
            if !self.ringing.is_empty() {
                if self.ringing.iter().any(|r| r.snoozed_until.is_none())
                    && ui.button("Snooze 9 minutes").clicked()
                {
                    self.snooze();
                    ui.close();
                }
                if ui.button("Stop alarm").clicked() {
                    self.ringing.clear();
                    ui.close();
                }
                ui.separator();
            }
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
            let out = self.saved.dead_bulbs.len();
            if mood != Mood::Off && self.saved.replace == Replace::ByHand && out > 0 {
                let text = match out {
                    1 => "Replace the dead bulb".to_string(),
                    n => format!("Replace the {n} dead bulbs"),
                };
                if ui.button(text).clicked() {
                    for i in self.saved.dead_bulbs.clone() {
                        self.lighting.replace(i, time);
                    }
                    ui.close();
                }
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
}

impl eframe::App for ClockApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_window(&ctx);
        ctx.set_theme(match self.saved.menus {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        });
        let time = ctx.input(|i| i.time);
        self.alarms(time);
        self.apply_sound(time);
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
        if !self.settings_open {
            self.settings_at = None;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.saved);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.saved.palette().board.to_normalized_gamma_f32()
    }
}

const BEER_PAYPAL: &str = "spoolbeermoney@outlook.com";
const CLINK: &str = r"            \   |   /
         *   CLINK!   *
            /   |   \
   .-~~~~~~-.  .-~~~~~~-.
  (  o  o  . )( .  o  o  )
   |~~~~~~~~|  |~~~~~~~~|
 __|  o  .  |  |  .  o  |__
(__|    o   |  |   o    |__)
 __| .    o |  | o    . |__
(__|  o     |  |     o  |__)
   |________|  |________|";

/// "Split-Flap Clock 1.1.0, released 29 Oct 2026", from the version and
/// the date the changelog gives it (see build.rs).
fn version_text() -> String {
    let version = env!("CARGO_PKG_VERSION");
    match chrono::NaiveDate::parse_from_str(env!("RELEASE_DATE"), "%Y-%m-%d") {
        Ok(d) => format!(
            "Split-Flap Clock {version}, released {}",
            d.format("%-d %b %Y")
        ),
        Err(_) => format!("Split-Flap Clock {version}, a test build"),
    }
}

/// Where the settings window goes, of `size`, for a clock window at `clock`
/// on a screen of `screen`: to the clock's right if it fits, or else its
/// left, level with its top, and kept on the screen. Screens side by side
/// are taken to be the same size, as the system doesn't say where each one
/// starts.
fn settings_spot(clock: Rect, screen: Vec2, size: Vec2) -> egui::Pos2 {
    const GAP: f32 = 12.0;
    let left = (clock.center().x / screen.x).floor() * screen.x;
    let top = (clock.center().y / screen.y).floor() * screen.y;
    let right_edge = left + screen.x;
    let x = if clock.max.x + GAP + size.x <= right_edge {
        clock.max.x + GAP
    } else if clock.min.x - GAP - size.x >= left {
        clock.min.x - GAP - size.x
    } else {
        // No room either side: as close as it will go on the right.
        (right_edge - size.x).max(left)
    };
    // Room for the title bar and a taskbar at the bottom.
    let y = clock.min.y.min(top + screen.y - size.y - 80.0).max(top);
    egui::pos2(x, y)
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
        let a = zone_cells("LONDON", 9, 5, "MON", true, None);
        let b = zone_cells("A NAME THAT IS FAR TOO LONG", 21, 30, "TUE", true, None);
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
    fn places_and_alarms_get_numbers_of_their_own() {
        let mut s: Saved = ron::from_str(
            "(zones: [(tz: \"Asia/Tokyo\", label: \"TOKYO\"), (tz: \"UTC\", label: \"UTC\", id: 5), (tz: \"UTC\", label: \"B\", id: 5)], alarms: [(), ()])",
        )
        .unwrap();
        s.number();
        let ids: Vec<u64> = s.zones.iter().map(|z| z.id).collect();
        assert!(ids.iter().all(|&id| id != 0));
        assert_eq!(ids[1], 5);
        assert!(ids[0] != ids[1] && ids[0] != ids[2] && ids[1] != ids[2]);
        assert!(s.alarms[0].id != 0 && s.alarms[0].id != s.alarms[1].id);
        // An alarm that follows a place that has gone follows your time.
        assert!(s.zone(Some(999)).is_none());
    }

    #[test]
    fn the_alarm_row_never_widens_the_board() {
        let r = Ringing {
            id: 1,
            label: "TIME FOR TEA".into(),
            reason: String::new(),
            shown: (16, 0),
            place: None,
            since: 0,
            snoozed_until: None,
            colour: None,
        };
        let main = time_cells(9, 5, None, true, true);
        let row = fit(alarm_cells(&r, false), row_width(&main));
        assert!(row_width(&row) <= row_width(&main) + 0.01);
        // On a wide board it keeps its full size.
        let row = fit(alarm_cells(&r, false), 5000.0);
        assert_eq!(row_width(&row), row_width(&alarm_cells(&r, false)));
    }

    #[test]
    fn the_alarm_row_announces_and_delays() {
        let mut r = Ringing {
            id: 1,
            label: "WAKE UP".into(),
            reason: String::new(),
            shown: (7, 30),
            place: None,
            since: 0,
            snoozed_until: None,
            colour: None,
        };
        // NOW, and only NOW: it doesn't change while it rings.
        let row = texts(&alarm_cells(&r, true));
        assert!(row.starts_with("WAKE UP") && row.contains("07:30") && row.contains("NOW"));
        // With a reason, the reason.
        r.reason = "DENTIST AT 9".into();
        let row = texts(&alarm_cells(&r, true));
        assert!(row.ends_with("DENTIST AT 9") && !row.contains("NOW"));
        // Snoozed, DELAYED, on as many cards as the reason had.
        r.snoozed_until = Some(540);
        r.shown = (7, 39);
        let snoozed = alarm_cells(&r, true);
        assert!(texts(&snoozed).contains("07:39") && texts(&snoozed).contains("DELAYED"));
        assert_eq!(
            snoozed.len(),
            alarm_cells(
                &Ringing {
                    snoozed_until: None,
                    colour: None,
                    ..r
                },
                true
            )
            .len()
        );
    }

    #[test]
    fn settings_open_next_to_the_clock() {
        let screen = vec2(3440.0, 1440.0);
        let size = vec2(560.0, 680.0);
        let clock = |x: f32, y: f32| Rect::from_min_size(egui::pos2(x, y), vec2(550.0, 330.0));
        // Room on the right: just to the right, level with the top.
        assert_eq!(
            settings_spot(clock(100.0, 200.0), screen, size),
            egui::pos2(662.0, 200.0)
        );
        // At the right edge: on the left instead.
        assert_eq!(
            settings_spot(clock(2850.0, 200.0), screen, size),
            egui::pos2(2278.0, 200.0)
        );
        // Low on the screen: moved up to fit.
        assert_eq!(
            settings_spot(clock(100.0, 1000.0), screen, size).y,
            1440.0 - 680.0 - 80.0
        );
        // On a second screen to the right, it stays on that screen.
        let p = settings_spot(clock(3440.0 + 2850.0, 200.0), screen, size);
        assert!(p.x >= 3440.0 && p.x + size.x <= 2.0 * 3440.0);
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
