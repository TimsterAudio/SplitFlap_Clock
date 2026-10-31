//! The board's lighting: a row of incandescent lamps just above it, each
//! throwing a warm pool of light down the board, brightest near the top and
//! fading away below. Once in a while, a bulb flickers, then either comes
//! good or blows: a flash and a pop, a wisp of smoke, and its glass falling
//! to the foot of the board, its part of the board in shadow until it's
//! replaced: after a while by itself, or by hand, by clicking it; and the
//! janitor comes along the foot of the board with a hand brush and sweeps
//! the glass off the nearer end.
//!
//! The light is drawn over the finished board: a fine grid, darkened where
//! the lamps don't reach and warmed where they do, so it falls on the cards,
//! their letters and the board alike.

use egui::{pos2, Color32, Mesh, Painter, Pos2, Rect};
use serde::{Deserialize, Serialize};

use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Mood {
    /// No lamps: the board evenly lit, as it always was.
    #[default]
    Off,
    /// A gentle warm wash.
    Soft,
    /// Warm pools under each lamp.
    Warm,
    /// A dim hall, the lamps doing all the work.
    Evening,
}

impl Mood {
    /// How the mood lights the board.
    fn look(self) -> Option<MoodLook> {
        let warm = [1.0, 0.72, 0.42];
        match self {
            Mood::Off => None,
            Mood::Soft => Some(MoodLook {
                ambient: 0.8,
                lamp: 0.3,
                glow: 0.06,
                tint: warm,
                shadow: 0.45,
            }),
            Mood::Warm => Some(MoodLook {
                ambient: 0.58,
                lamp: 0.5,
                glow: 0.1,
                tint: warm,
                shadow: 0.6,
            }),
            Mood::Evening => Some(MoodLook {
                ambient: 0.32,
                lamp: 0.75,
                glow: 0.13,
                tint: [1.0, 0.64, 0.34],
                shadow: 0.75,
            }),
        }
    }

    /// How dark the cards' shadows on the board are, 0 for none.
    pub fn shadow(self) -> f32 {
        self.look().map_or(0.0, |l| l.shadow)
    }
}

const DAY: f64 = 24.0 * 3600.0;

/// How far the dimmer turns the lamps down, and up.
pub const DIMMEST: f32 = 0.25;
pub const BRIGHTEST: f32 = 1.6;

/// Who replaces a dead bulb.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Replace {
    /// Someone comes round with a new one after a minute or so.
    #[default]
    Automatically,
    /// It stays out, even when the clock is restarted, until it's clicked.
    ByHand,
}

/// How often bulbs fail.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Failures {
    Never,
    #[default]
    Sometimes,
    Often,
    /// For the impatient.
    Hourly,
}

impl Failures {
    pub const ALL: [Failures; 4] = [
        Failures::Never,
        Failures::Sometimes,
        Failures::Often,
        Failures::Hourly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Failures::Never => "Never fail",
            Failures::Sometimes => "Fail every few days",
            Failures::Often => "Fail about once a day",
            Failures::Hourly => "Fail about every hour",
        }
    }

    /// The average seconds, while the clock is running, between a bulb
    /// starting to play up, anywhere on the board. When is left to chance:
    /// it could be minutes after the last, or a week.
    fn every(self) -> Option<f64> {
        match self {
            Failures::Never => None,
            Failures::Sometimes => Some(4.0 * DAY),
            Failures::Often => Some(DAY),
            Failures::Hourly => Some(3600.0),
        }
    }
}

#[derive(Clone, Copy)]
struct MoodLook {
    /// The light everywhere, lamps or not, from 0 to 1.
    ambient: f32,
    /// How much a lamp adds under itself.
    lamp: f32,
    /// How much warm colour a lamp adds.
    glow: f32,
    tint: [f32; 3],
    shadow: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Lit,
    /// Sputtering until `until`, its brightness jumping about; then it
    /// comes good, or `dies`.
    Flicker {
        until: f64,
        next: f64,
        dies: bool,
    },
    /// Out until it's replaced, at `until`, or, if that's infinite, by
    /// hand.
    Dead {
        until: f64,
    },
}

#[derive(Clone, Copy)]
struct Lamp {
    state: State,
    /// How bright the filament is, chasing `target` with a little lag, as a
    /// real filament warms and cools.
    power: f32,
    target: f32,
}

impl Lamp {
    fn new() -> Self {
        Self {
            state: State::Lit,
            power: 1.0,
            target: 1.0,
        }
    }
}

/// A piece of a blown bulb's glass, falling to the foot of the board and
/// lying there. Places and sizes are shares of the board's width and
/// height, so it stays put when the window changes size.
#[derive(Clone, Copy)]
struct Shard {
    /// Where it lands: across, from under its lamp, in lamp spacings; and
    /// down, as a share of the board's height.
    x: f32,
    y: f32,
    /// How far it drifts sideways on the way down, in lamp spacings.
    drift: f32,
    /// When it starts to fall, after the bulb blows.
    delay: f32,
    size: f32,
    /// Its turn when it lands, and how much it spins as it falls.
    angle: f32,
    spin: f32,
    /// Its corners, as angles and distances out from its middle.
    corners: [(f32, f32); 4],
    /// How bright it catches the light.
    shine: f32,
}

/// A blown bulb's glass, under lamp `lamp`, from when it blew.
struct Glass {
    lamp: usize,
    at: f64,
    shards: Vec<Shard>,
    /// Being swept away, since when, and which way: -1 to the left, 1 to
    /// the right.
    swept: Option<(f64, f32)>,
}

/// How fast the brush goes, a share of the board's width a second.
const BRUSH_SPEED: f32 = 0.2;
/// Its strokes: how far each pushes forward and draws back, a share of the
/// board's width, and how many a second.
const STROKE: f32 = 0.032;
const STROKES: f32 = 1.5;

/// How far along its way the brush is, `t` seconds into the sweep, as a
/// share of the board's width: a push forward, a short draw back, and
/// another push, gaining a little each time.
fn sweep_progress(t: f32) -> f32 {
    BRUSH_SPEED * t + STROKE * (t * std::f32::consts::TAU * STROKES).sin()
}

/// How fast the brush is going `t` seconds in: below nothing on a draw
/// back.
fn sweep_speed(t: f32) -> f32 {
    let w = std::f32::consts::TAU * STROKES;
    BRUSH_SPEED + STROKE * w * (t * w).cos()
}

/// The farthest the brush has pushed by `t`: where the glass it moved
/// lies, as it stays put while the brush draws back.
fn sweep_reach(t: f32) -> f32 {
    // The stroke turns back where its speed comes to nothing going
    // forward; each turn is farther on than the last, so the farthest yet
    // is now or the last turn.
    let tau = std::f32::consts::TAU;
    let w = tau * STROKES;
    let c = -BRUSH_SPEED / (STROKE * w);
    let now = sweep_progress(t).max(0.0);
    if c < -1.0 {
        return now;
    }
    let turn = c.acos();
    let k = ((w * t - turn) / tau).floor();
    if k < 0.0 {
        return now;
    }
    now.max(sweep_progress((turn + tau * k) / w))
}

impl Glass {
    /// Where the brush is across the board, a progress of `p` along its
    /// way: from just behind the pile, out past the nearer end.
    fn brush(&self, n: f32, p: f32, way: f32) -> f32 {
        let centre = (self.lamp as f32 + 0.5) / n;
        let start = centre - way * 0.5 / n;
        start + way * p
    }

    /// Whether the sweep is done at `time`: the brush and the glass gone
    /// off the end of the board.
    fn swept_away(&self, n: f32, time: f64) -> bool {
        match self.swept {
            Some((since, way)) => {
                let x = self.brush(n, sweep_reach((time - since) as f32), way);
                if way > 0.0 {
                    x > 1.12
                } else {
                    x < -0.12
                }
            }
            None => false,
        }
    }
}

/// A puff of a blown bulb's smoke, carried off sideways by the draught.
#[derive(Clone, Copy)]
struct Puff {
    born: f64,
    life: f32,
    x: f32,
    y: f32,
    /// How fast it sets off, and how hard the draught pushes it on, across
    /// the board, a share of its width a second (and a second squared).
    vx: f32,
    wind: f32,
    /// How fast it rises (or, caught in an eddy, sinks).
    vy: f32,
    size: f32,
    grow: f32,
    thick: f32,
    /// Where it is in its curl, so puffs don't wave together.
    phase: f32,
}

/// How long a shard takes to fall the height of the board, in seconds.
const FALL: f32 = 0.6;

pub struct Lighting {
    lamps: Vec<Lamp>,
    rng: Rng,
    /// When the lamps were last moved on.
    last: Option<f64>,
    glass: Vec<Glass>,
    smoke: Vec<Puff>,
    /// Bulbs that have blown since [`Lighting::take_pops`] was last asked,
    /// for the sound.
    pops: Vec<usize>,
    /// When each bulb blew, for its flash.
    flashes: Vec<(usize, f64)>,
}

impl Lighting {
    pub fn new(rng: Rng) -> Self {
        Self {
            lamps: Vec::new(),
            rng,
            last: None,
            glass: Vec::new(),
            smoke: Vec::new(),
            pops: Vec::new(),
            flashes: Vec::new(),
        }
    }

    /// Moves the lamps on to `time`, for a board lit by `n` lamps. Whether
    /// anything is changing, so the board should be drawn again soon. The
    /// time since the last call is measured, not taken from the frame rate,
    /// as a still board is only drawn about once a second.
    pub fn update(&mut self, n: usize, time: f64, failures: Failures, replace: Replace) -> bool {
        let dt = self.last.map_or(0.0, |last| (time - last).clamp(0.0, 2.0));
        self.last = Some(time);
        self.lamps.resize(n, Lamp::new());
        // A new problem somewhere on the board, now and then.
        if let Some(every) = failures.every() {
            if self.rng.between(0.0, 1.0) < dt / every {
                let pick = self.rng.between(0.0, n as f64) as usize;
                if let Some(lamp) = self.lamps.get_mut(pick.min(n.saturating_sub(1))) {
                    if lamp.state == State::Lit {
                        let until = time + self.rng.between(1.5, 6.0);
                        let dies = self.rng.between(0.0, 1.0) < 0.35;
                        lamp.state = State::Flicker {
                            until,
                            next: time,
                            dies,
                        };
                    }
                }
            }
        }
        if failures == Failures::Never {
            for lamp in &mut self.lamps {
                lamp.state = State::Lit;
                lamp.target = 1.0;
            }
        }
        let mut busy = false;
        for i in 0..self.lamps.len() {
            let mut lamp = self.lamps[i];
            match lamp.state {
                State::Lit => lamp.target = 1.0,
                State::Flicker { until, next, dies } => {
                    if time >= until {
                        if dies {
                            lamp.state = State::Dead {
                                until: f64::INFINITY,
                            };
                            lamp.target = 0.0;
                            // A flash, then it blows.
                            lamp.power = 1.0;
                            self.blow_up(i, n, time);
                        } else {
                            lamp.state = State::Lit;
                            lamp.target = 1.0;
                        }
                    } else if time >= next {
                        // Mostly dimmed and wavering, with the odd drop out.
                        lamp.target = if self.rng.between(0.0, 1.0) < 0.3 {
                            self.rng.between(0.0, 0.15) as f32
                        } else {
                            self.rng.between(0.45, 1.0) as f32
                        };
                        let next = time + self.rng.between(0.04, 0.18);
                        lamp.state = State::Flicker { until, next, dies };
                    }
                    busy = true;
                }
                State::Dead { until } => {
                    lamp.target = 0.0;
                    if replace == Replace::Automatically && until.is_infinite() {
                        // Someone will be round with a new one.
                        lamp.state = State::Dead {
                            until: time + self.rng.between(20.0, 90.0),
                        };
                    } else if replace == Replace::ByHand && until.is_finite() {
                        lamp.state = State::Dead {
                            until: f64::INFINITY,
                        };
                    } else if time >= until {
                        // A new bulb, which sputters for a moment on, and
                        // the glass swept up.
                        self.sweep(i, time);
                        let until = time + self.rng.between(0.3, 1.2);
                        lamp.state = State::Flicker {
                            until,
                            next: time,
                            dies: false,
                        };
                    }
                }
            }
            // The filament's lag: about 60 ms to warm or cool.
            let k = 1.0 - (-dt / 0.06).exp() as f32;
            lamp.power += (lamp.target - lamp.power) * k;
            if (lamp.target - lamp.power).abs() > 0.01 {
                busy = true;
            }
            self.lamps[i] = lamp;
        }
        // Smoke drifts and clears; glass falls.
        self.smoke.retain(|p| time - p.born < p.life as f64);
        self.flashes.retain(|(_, at)| time - at < 0.5);
        let lamps = n.max(1) as f32;
        self.glass
            .retain(|g| g.lamp < n && !g.swept_away(lamps, time));
        busy |= !self.smoke.is_empty()
            || !self.flashes.is_empty()
            || self
                .glass
                .iter()
                .any(|g| time - g.at < (FALL + 0.3) as f64 || g.swept.is_some());
        busy
    }

    /// Makes a bulb that's lit blow, after a moment's flickering: the one at
    /// random, for anyone who can't wait.
    pub fn blow(&mut self, time: f64) -> bool {
        let lit: Vec<usize> = (0..self.lamps.len())
            .filter(|&i| self.lamps[i].state == State::Lit)
            .collect();
        if lit.is_empty() {
            return false;
        }
        let pick = lit[(self.rng.between(0.0, lit.len() as f64) as usize).min(lit.len() - 1)];
        self.lamps[pick].state = State::Flicker {
            until: time + self.rng.between(1.2, 2.2),
            next: time,
            dies: true,
        };
        true
    }

    /// The bulbs that have blown since this was last asked, by their place
    /// from the left, and how many lamps there are.
    pub fn take_pops(&mut self) -> (Vec<usize>, usize) {
        (std::mem::take(&mut self.pops), self.lamps.len())
    }

    /// Bulb `i` of `n` blows at `time`: its glass falls and its smoke rises.
    fn blow_up(&mut self, i: usize, n: usize, time: f64) {
        self.pops.push(i);
        self.flashes.push((i, time));
        let x = (i as f32 + 0.5) / n.max(1) as f32;
        let mut r = |lo: f32, hi: f32| self.rng.between(lo as f64, hi as f64) as f32;
        let count = r(9.0, 15.0) as usize;
        let mut shards = Vec::with_capacity(count);
        for _ in 0..count {
            let size = r(0.009, 0.022);
            // Four corners, roughly a quarter turn apart, so it's a convex
            // sliver of some shape or other.
            let mut corners = [(0.0, 0.0); 4];
            let start = r(0.0, 6.3);
            for (k, c) in corners.iter_mut().enumerate() {
                let a = start + k as f32 * std::f32::consts::FRAC_PI_2 + r(-0.45, 0.45);
                *c = (a, r(0.35, 1.0));
            }
            shards.push(Shard {
                x: r(-0.32, 0.32),
                y: 1.0 - r(0.006, 0.03),
                drift: r(-0.08, 0.08),
                delay: r(0.0, 0.12),
                size,
                angle: r(0.0, 6.3),
                spin: r(-9.0, 9.0),
                corners,
                shine: r(0.5, 1.0),
            });
        }
        // Glass from an earlier bulb there has been swept up already.
        self.glass.retain(|g| g.lamp != i);
        self.glass.push(Glass {
            lamp: i,
            at: time,
            shards,
            swept: None,
        });
        let mut r = |lo: f32, hi: f32| self.rng.between(lo as f64, hi as f64) as f32;
        // The draught takes the smoke one way or the other, a thin trail
        // that thins as it goes: mostly across the board rather than off
        // its edge.
        let inward = if x < 0.5 { 1.0 } else { -1.0 };
        let way = if r(0.0, 1.0) < 0.8 { inward } else { -inward };
        let draught = r(0.6, 1.4);
        for k in 0..16 {
            let puff = Puff {
                born: time + k as f64 * 0.14 + r(0.0, 0.1) as f64,
                life: r(3.8, 6.0),
                x: x + r(-0.006, 0.006),
                y: r(0.03, 0.07),
                vx: way * draught * r(0.02, 0.045),
                wind: way * draught * r(0.012, 0.028),
                vy: r(-0.012, 0.01),
                size: r(0.01, 0.018),
                grow: r(0.015, 0.03),
                thick: r(0.28, 0.5),
                phase: r(0.0, 6.3),
            };
            self.smoke.push(puff);
        }
    }

    /// The bulbs that are out, by their place from the left.
    pub fn dead(&self) -> Vec<usize> {
        (0..self.lamps.len())
            .filter(|&i| matches!(self.lamps[i].state, State::Dead { .. }))
            .collect()
    }

    /// Puts back the bulbs that were out when the clock last ran.
    pub fn restore(&mut self, dead: &[usize]) {
        for &i in dead {
            if i >= self.lamps.len() {
                self.lamps.resize(i + 1, Lamp::new());
            }
            // Its glass, still lying where it fell, long since quiet.
            self.blow_up(i, 1, f64::NEG_INFINITY);
            self.pops.clear();
            self.flashes.clear();
            self.smoke.clear();
            self.lamps[i] = Lamp {
                state: State::Dead {
                    until: f64::INFINITY,
                },
                power: 0.0,
                target: 0.0,
            };
        }
    }

    /// Screws in a new bulb at `i`, if that one is out: it sputters for a
    /// moment, then comes on.
    pub fn replace(&mut self, i: usize, time: f64) -> bool {
        match self.lamps.get_mut(i) {
            Some(lamp) if matches!(lamp.state, State::Dead { .. }) => {
                lamp.state = State::Flicker {
                    until: time + self.rng.between(0.3, 1.2),
                    next: time,
                    dies: false,
                };
                self.sweep(i, time);
                true
            }
            _ => false,
        }
    }

    /// Sends the janitor for lamp `i`'s glass: swept off the nearer end of
    /// the board.
    fn sweep(&mut self, i: usize, time: f64) {
        let n = self.lamps.len().max(1) as f32;
        for g in self.glass.iter_mut().filter(|g| g.lamp == i) {
            if g.swept.is_none() {
                let centre = (i as f32 + 0.5) / n;
                let way = if centre < 0.5 { -1.0 } else { 1.0 };
                g.swept = Some((time, way));
            }
        }
    }

    /// The lamp at `pos`, over the board at `rect`: within its glow at the
    /// top edge.
    pub fn lamp_at(&self, rect: Rect, pos: Pos2) -> Option<usize> {
        let n = self.lamps.len();
        if n == 0 || !rect.contains(pos) {
            return None;
        }
        let w = rect.width() / n as f32;
        let i = (((pos.x - rect.min.x) / w) as usize).min(n - 1);
        let centre = rect.min.x + w * (i as f32 + 0.5);
        let deep = (rect.height() * 0.12).max(24.0);
        ((pos.x - centre).abs() < w * 0.42 && pos.y - rect.min.y < deep).then_some(i)
    }

    /// How lit a point is, at `x` across and `y` down the board, both from 0
    /// to 1: the ambient light and the lamps' pools.
    fn light(&self, look: &MoodLook, x: f32, y: f32) -> (f32, f32) {
        let n = self.lamps.len().max(1) as f32;
        let spread = 0.4 / n;
        let mut pools = 0.0;
        for (i, lamp) in self.lamps.iter().enumerate() {
            let lx = (i as f32 + 0.5) / n;
            let dx = (x - lx) / spread;
            pools += lamp.power * (-dx * dx).exp();
        }
        // Brightest at the top, near the lamps, fading down the board.
        let down = (-(y / 0.95) * (y / 0.95)).exp();
        let pool = pools * down;
        ((look.ambient + look.lamp * pool).min(1.0), look.glow * pool)
    }

    /// Lights the board at `rect`, drawn already, in `mood`, at `time`,
    /// the lamps turned up or down by `dimmer` (1 as the mood has them):
    /// and any bulb that has blown, its flash, smoke and glass.
    pub fn draw(&self, painter: &Painter, rect: Rect, mood: Mood, dimmer: f32, time: f64) {
        let Some(mut look) = mood.look() else {
            return;
        };
        // The dimmer turns the lamps up or down; the hall's own light stays.
        let dimmer = dimmer.clamp(DIMMEST, BRIGHTEST);
        look.lamp *= dimmer;
        look.glow *= dimmer;
        const COLS: usize = 40;
        const ROWS: usize = 20;
        let mut shade = Mesh::default();
        let mut glow = Mesh::default();
        for row in 0..=ROWS {
            for col in 0..=COLS {
                let (x, y) = (col as f32 / COLS as f32, row as f32 / ROWS as f32);
                let p = pos2(
                    rect.min.x + rect.width() * x,
                    rect.min.y + rect.height() * y,
                );
                let (light, warmth) = self.light(&look, x, y);
                shade.colored_vertex(p, Color32::from_black_alpha(((1.0 - light) * 255.0) as u8));
                glow.colored_vertex(p, additive(look.tint, warmth));
            }
        }
        for row in 0..ROWS as u32 {
            for col in 0..COLS as u32 {
                let i = row * (COLS as u32 + 1) + col;
                let j = i + COLS as u32 + 1;
                for m in [&mut shade, &mut glow] {
                    m.add_triangle(i, i + 1, j);
                    m.add_triangle(i + 1, j + 1, j);
                }
            }
        }
        painter.add(shade);
        painter.add(glow);
        // The lamps themselves, just above the top edge: a warm glow each,
        // gone when the bulb is.
        let n = self.lamps.len().max(1) as f32;
        for (i, lamp) in self.lamps.iter().enumerate() {
            let x = rect.min.x + rect.width() * (i as f32 + 0.5) / n;
            let rx = rect.width() / n * 0.42;
            let ry = (rect.height() * 0.08).max(8.0);
            painter.add(halo(
                pos2(x, rect.min.y),
                rx,
                ry,
                additive(look.tint, 0.55 * lamp.power * dimmer),
            ));
        }
        let at = |x: f32, y: f32| {
            pos2(
                rect.min.x + rect.width() * x,
                rect.min.y + rect.height() * y,
            )
        };
        // A blowing bulb's last flash, white hot.
        for &(i, when) in &self.flashes {
            let k = (1.0 - (time - when) as f32 / 0.35).max(0.0);
            if k > 0.0 {
                painter.add(halo(
                    at((i as f32 + 0.5) / n, 0.0),
                    rect.width() / n * 0.9,
                    rect.height() * 0.4,
                    additive([1.0, 0.95, 0.85], k * k),
                ));
            }
        }
        // Its smoke, spreading under the top of the board and clearing.
        for p in &self.smoke {
            let age = (time - p.born) as f32;
            if age < 0.0 {
                continue;
            }
            let fade = (1.0 - age / p.life).max(0.0);
            let alpha = p.thick * (age / 0.3).min(1.0) * fade * fade * 0.85;
            let radius = (p.size + p.grow * age) * rect.height();
            // Off with the draught, curling as it goes.
            let x = p.x + p.vx * age + 0.5 * p.wind * age * age;
            let curl = (age * 2.4 + p.phase).sin() * 0.012 * age.min(1.5);
            let y = (p.y + p.vy * age + curl).max(0.008);
            painter.add(halo(
                at(x, y),
                radius * 1.8,
                radius,
                Color32::from_rgba_unmultiplied(132, 127, 120, (alpha * 255.0) as u8),
            ));
        }
        // Its glass, falling to the foot of the board and lying there,
        // catching what light there is.
        for g in &self.glass {
            let centre = (g.lamp as f32 + 0.5) / n;
            // Where the brush is, if the janitor's at work.
            // Where the brush is now, and the farthest it has pushed.
            let brush = g.swept.map(|(since, way)| {
                let t = (time - since) as f32;
                let now = g.brush(n, sweep_progress(t), way);
                let reach = g.brush(n, sweep_reach(t), way);
                (now, reach, way, t)
            });
            for sh in &g.shards {
                let t = (time - g.at) as f32 - sh.delay;
                if t < 0.0 {
                    continue;
                }
                let s = (t / FALL).min(1.0);
                let mut x = centre + (sh.x * (0.3 + 0.7 * s) + sh.drift * s) / n;
                let top = 0.03;
                let mut y = top + (sh.y - top) * s * s;
                let mut angle = sh.angle + sh.spin * (1.0 - s);
                // Pushed along in front of the brush, skittering as it goes.
                // Pushed along by the brush's farthest stroke so far, and left
                // there as it draws back; skittering while it's pushing.
                if let Some((bx, reach, way, t)) = brush {
                    let front = reach + way * (0.018 + sh.size * 0.8);
                    let pushed = if way > 0.0 { front - x } else { x - front };
                    if pushed > 0.0 {
                        x = front;
                        if (bx - reach).abs() < 0.002 && sweep_speed(t) > 0.0 {
                            y -= 0.006 * (pushed * 90.0).sin().abs();
                        }
                        angle += pushed * sh.spin * 3.0;
                    }
                }
                let size = sh.size * rect.height();
                let middle = at(x, y);
                let points: Vec<Pos2> = sh
                    .corners
                    .iter()
                    .map(|&(a, d)| {
                        let a = a + angle;
                        middle + egui::vec2(a.cos(), a.sin() * 0.6) * d * size
                    })
                    .collect();
                let lit = self.light(&look, x, y).0 * sh.shine;
                let fill =
                    Color32::from_rgba_unmultiplied(196, 206, 212, (50.0 + 110.0 * lit) as u8);
                let edge =
                    Color32::from_rgba_unmultiplied(255, 248, 230, (70.0 + 170.0 * lit) as u8);
                let glint = points[0];
                painter.add(egui::Shape::convex_polygon(
                    points,
                    fill,
                    egui::Stroke::new(0.8, edge),
                ));
                // The odd piece catches the light at a corner.
                if sh.shine > 0.8 {
                    painter.add(halo(
                        glint,
                        size * 0.9,
                        size * 0.9,
                        additive([1.0, 0.96, 0.86], 0.25 + 0.6 * lit),
                    ));
                }
            }
            if let Some((bx, _, way, t)) = brush {
                let lit = self.light(&look, bx.clamp(0.0, 1.0), 0.95).0;
                // Going forward or drawing back, and how hard.
                let speed = sweep_speed(t) / (STROKE * std::f32::consts::TAU * STROKES);
                draw_brush(painter, rect, bx, way, speed, (t / 0.25).min(1.0), lit);
            }
        }
    }
}

/// The janitor's hand brush at `x` across the foot of the board at
/// `rect`, sweeping `way` (-1 left, 1 right), its handle trailing behind,
/// in the middle of a `stroke`: from 1, pushing hard, to -1, drawing back
/// as fast as it goes: a pale
/// wooden back with its handle running out behind, and dark bristles under
/// the head, splayed by the stroke. Drawn back, it lifts a little off the
/// floor. `fade` from 0 to 1 as it comes into view, in `lit` light.
fn draw_brush(painter: &Painter, rect: Rect, x: f32, way: f32, stroke: f32, fade: f32, lit: f32) {
    let shade = |c: [u8; 3]| {
        let k = 0.35 + 0.65 * lit;
        Color32::from_rgba_unmultiplied(
            (c[0] as f32 * k) as u8,
            (c[1] as f32 * k) as u8,
            (c[2] as f32 * k) as u8,
            (fade * 255.0) as u8,
        )
    };
    let p = egui::pos2;
    let going = way * stroke.signum();
    let pace = stroke.abs().min(1.0);
    let cx = rect.min.x + rect.width() * x;
    // The head's length, the bristles' depth, and the wood's thickness.
    let w = (rect.width() * 0.055).max(16.0);
    let hb = (rect.height() * 0.045).max(8.0);
    // Lifted a little off the floor as it's drawn back.
    let lift = if going * way < 0.0 {
        hb * 0.3 * pace
    } else {
        0.0
    };
    let foot = rect.max.y - 1.0 - lift;
    let t = (hb * 0.42).max(3.0);
    let top = foot - hb;
    let front = cx + way * w * 0.5;
    let back = cx - way * w * 0.5;
    let end = back - way * w * 0.85;
    // The bristles, fanning out a little to the floor and swept back by
    // the stroke, either way, with a few lighter hairs among them.
    let splay = -going * w * 0.1 * (0.3 + 0.7 * pace);
    painter.add(egui::Shape::convex_polygon(
        vec![
            p(cx - w * 0.47, top),
            p(cx + w * 0.47, top),
            p(cx + w * 0.55 + splay, foot),
            p(cx - w * 0.55 + splay, foot),
        ],
        shade([54, 54, 56]),
        egui::Stroke::NONE,
    ));
    for k in 0..9 {
        let f = (k as f32 + 0.5) / 9.0 - 0.5;
        painter.line_segment(
            [
                p(cx + f * w * 0.9, top + 1.0),
                p(cx + f * w * 1.06 + splay, foot),
            ],
            egui::Stroke::new(0.7, shade([128, 128, 130])),
        );
    }
    // The wooden back and its handle, rising gently as it runs out behind,
    // with a hanging hole near its rounded end.
    let rise = t * 0.9;
    painter.add(egui::Shape::convex_polygon(
        vec![
            p(front, top),
            p(front, top - t),
            p(end, top - t - rise),
            p(end, top - rise * 0.55),
            p(back, top),
        ],
        shade([214, 180, 130]),
        egui::Stroke::NONE,
    ));
    painter.circle_filled(
        p(end, top - t * 0.5 - rise * 0.8),
        t * 0.5,
        shade([214, 180, 130]),
    );
    // Light along its top edge, as in the picture.
    painter.line_segment(
        [p(front, top - t + 0.8), p(end, top - t - rise + 0.8)],
        egui::Stroke::new(1.0, shade([240, 214, 170])),
    );
    painter.circle_filled(
        p(end + way * t * 0.35, top - t * 0.5 - rise * 0.8),
        (t * 0.16).max(1.0),
        shade([70, 50, 30]),
    );
}

/// A colour to add on top, rather than paint over: premultiplied, with no
/// alpha.
fn additive(tint: [f32; 3], amount: f32) -> Color32 {
    let c = |t: f32| (t * amount.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_premultiplied(c(tint[0]), c(tint[1]), c(tint[2]), 0)
}

/// A soft oval glow, `colour` at its centre fading to nothing at its edge.
pub fn halo(centre: Pos2, rx: f32, ry: f32, colour: Color32) -> Mesh {
    let mut m = Mesh::default();
    m.colored_vertex(centre, colour);
    const N: u32 = 32;
    for k in 0..N {
        let a = k as f32 / N as f32 * std::f32::consts::TAU;
        m.colored_vertex(
            centre + egui::vec2(a.cos() * rx, a.sin() * ry),
            Color32::TRANSPARENT,
        );
    }
    for k in 0..N {
        m.add_triangle(0, 1 + k, 1 + (k + 1) % N);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightest_under_a_lamp_and_near_the_top() {
        let mut l = Lighting::new(Rng::with_seed(1));
        l.update(3, 0.0, Failures::Never, Replace::Automatically);
        let look = Mood::Warm.look().unwrap();
        let under = l.light(&look, 0.5, 0.1).0;
        let between = l.light(&look, 1.0 / 3.0, 0.1).0;
        let low = l.light(&look, 0.5, 0.95).0;
        assert!(under > between && under > low, "{under} {between} {low}");
        assert!(low >= look.ambient);
    }

    #[test]
    fn a_dead_bulb_leaves_its_pool_dark_until_replaced() {
        let mut l = Lighting::new(Rng::with_seed(2));
        l.update(3, 0.0, Failures::Sometimes, Replace::Automatically);
        l.lamps[1].state = State::Flicker {
            until: 1.0,
            next: 0.0,
            dies: true,
        };
        let mut t = 0.0;
        while t < 3.0 {
            l.update(3, t, Failures::Sometimes, Replace::Automatically);
            t += 0.016;
        }
        assert!(matches!(l.lamps[1].state, State::Dead { .. }));
        assert!(l.lamps[1].power < 0.05);
        let look = Mood::Warm.look().unwrap();
        assert!(l.light(&look, 0.5, 0.1).0 < l.light(&look, 1.0 / 6.0, 0.1).0);
        // And comes back on, in time.
        let mut t = 3.0;
        while t < 100.0 {
            l.update(3, t, Failures::Never, Replace::Automatically);
            t += 0.05;
        }
        assert!(l.lamps[1].power > 0.95);
    }

    #[test]
    fn by_hand_a_dead_bulb_stays_out_until_it_is_clicked() {
        let mut l = Lighting::new(Rng::with_seed(4));
        let rect = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(600.0, 300.0));
        l.update(3, 0.0, Failures::Never, Replace::ByHand);
        l.lamps[2].state = State::Flicker {
            until: 1.0,
            next: 0.0,
            dies: true,
        };
        let mut t = 0.0;
        while t < 600.0 {
            l.update(3, t, Failures::Sometimes, Replace::ByHand);
            t += 0.5;
        }
        assert_eq!(l.dead(), vec![2]);
        // Restarted, it's still out.
        let mut again = Lighting::new(Rng::with_seed(5));
        again.restore(&l.dead());
        again.update(3, 0.0, Failures::Sometimes, Replace::ByHand);
        assert_eq!(again.dead(), vec![2]);
        // A click between the lamps, or low on the board, misses.
        assert_eq!(again.lamp_at(rect, pos2(200.0, 5.0)), None);
        assert_eq!(again.lamp_at(rect, pos2(500.0, 200.0)), None);
        assert!(!again.replace(1, 0.0));
        let at = again.lamp_at(rect, pos2(510.0, 8.0)).unwrap();
        assert!(again.replace(at, 0.0));
        let mut t = 0.0;
        while t < 3.0 {
            again.update(3, t, Failures::Sometimes, Replace::ByHand);
            t += 0.016;
        }
        assert!(again.dead().is_empty());
        assert!(again.lamps[2].power > 0.95);
    }

    #[test]
    fn a_blown_bulb_pops_smokes_and_leaves_its_glass_until_replaced() {
        let mut l = Lighting::new(Rng::with_seed(6));
        l.update(3, 0.0, Failures::Sometimes, Replace::ByHand);
        assert!(l.blow(0.0));
        let mut t = 0.0;
        while t < 3.0 {
            l.update(3, t, Failures::Sometimes, Replace::ByHand);
            t += 0.05;
        }
        let dead = l.dead();
        assert_eq!(dead.len(), 1);
        let (pops, n) = l.take_pops();
        assert_eq!((pops, n), (dead.clone(), 3));
        assert!(l.take_pops().0.is_empty());
        assert_eq!(l.glass.len(), 1);
        assert!(l.glass[0].shards.len() >= 7);
        assert!(!l.smoke.is_empty());
        // The smoke clears; the glass stays, and so does the dark.
        while t < 20.0 {
            l.update(3, t, Failures::Sometimes, Replace::ByHand);
            t += 0.5;
        }
        assert!(l.smoke.is_empty());
        assert_eq!(l.glass.len(), 1);
        assert_eq!(l.dead(), dead);
        // A new bulb, and the janitor sweeps the glass away: it's still
        // there while the brush is at work, then gone.
        assert!(l.replace(dead[0], t));
        assert_eq!(l.glass.len(), 1);
        assert!(l.glass[0].swept.is_some());
        let mut t2 = t;
        while t2 < t + 2.0 {
            assert!(l.update(3, t2, Failures::Sometimes, Replace::ByHand));
            t2 += 0.05;
        }
        assert_eq!(l.glass.len(), 1, "still sweeping");
        while t2 < t + 8.0 {
            l.update(3, t2, Failures::Sometimes, Replace::ByHand);
            t2 += 0.05;
        }
        assert!(l.glass.is_empty());
    }

    #[test]
    fn the_brush_sweeps_back_and_forth_but_the_glass_only_goes_one_way() {
        let mut back = 0;
        let mut last = (0.0, 0.0);
        let mut t = 0.0;
        while t < 4.0 {
            let (now, reach) = (sweep_progress(t), sweep_reach(t));
            if now < last.0 {
                back += 1;
            }
            // The glass lies where the farthest stroke left it: it never
            // comes back with the brush.
            assert!(reach >= last.1 && reach >= now - 1e-6);
            last = (now, reach);
            t += 0.01;
        }
        assert!(back > 40, "it draws back now and then: {back}");
        // And it gets there: about a fifth of the board a second.
        assert!((0.7..0.9).contains(&sweep_reach(4.0)));
    }

    #[test]
    fn bulbs_out_after_a_restart_have_their_glass_but_make_no_fuss() {
        let mut l = Lighting::new(Rng::with_seed(8));
        l.restore(&[1]);
        assert!(!l.update(3, 0.0, Failures::Sometimes, Replace::ByHand));
        assert!(l.take_pops().0.is_empty());
        assert!(l.smoke.is_empty());
        assert_eq!(l.glass.len(), 1);
    }

    #[test]
    fn failures_happen_about_as_often_as_asked() {
        let mut l = Lighting::new(Rng::with_seed(3));
        let mut starts = 0;
        let mut t = 0.0;
        // Thirty days, with a bulb failing about once a day.
        while t < 30.0 * DAY {
            let before: Vec<bool> = l.lamps.iter().map(|x| x.state == State::Lit).collect();
            l.update(4, t, Failures::Often, Replace::Automatically);
            for (i, lamp) in l.lamps.iter().enumerate() {
                if before.get(i) == Some(&true) && lamp.state != State::Lit {
                    starts += 1;
                }
            }
            t += 2.0;
        }
        assert!((15..=50).contains(&starts), "{starts}");
    }
}
