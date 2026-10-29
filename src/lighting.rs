//! The board's lighting: a row of incandescent lamps just above it, each
//! throwing a warm pool of light down the board, brightest near the top and
//! fading away below. Once in a while, a bulb flickers, then either comes
//! good or blows: a flash and a pop, a wisp of smoke, and its glass falling
//! to the foot of the board, its part of the board in shadow until it's
//! replaced: after a while by itself, or by hand, by clicking it, which
//! sweeps up the glass too.
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
}

/// A puff of a blown bulb's smoke.
#[derive(Clone, Copy)]
struct Puff {
    born: f64,
    life: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    grow: f32,
    thick: f32,
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
                        self.glass.retain(|g| g.lamp != i);
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
        self.glass.retain(|g| g.lamp < n);
        busy |= !self.smoke.is_empty()
            || !self.flashes.is_empty()
            || self.glass.iter().any(|g| time - g.at < (FALL + 0.3) as f64);
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
        });
        let mut r = |lo: f32, hi: f32| self.rng.between(lo as f64, hi as f64) as f32;
        for k in 0..24 {
            let puff = Puff {
                born: time + k as f64 * 0.1 + r(0.0, 0.1) as f64,
                life: r(4.0, 7.5),
                x: x + r(-0.012, 0.012),
                y: r(0.02, 0.06),
                vx: r(-0.028, 0.028),
                vy: r(-0.015, 0.022),
                size: r(0.02, 0.04),
                grow: r(0.025, 0.05),
                thick: r(0.45, 0.8),
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
                self.glass.retain(|g| g.lamp != i);
                lamp.state = State::Flicker {
                    until: time + self.rng.between(0.3, 1.2),
                    next: time,
                    dies: false,
                };
                true
            }
            _ => false,
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

    /// Lights the board at `rect`, drawn already, in `mood`, at `time`:
    /// and any bulb that has blown, its flash, smoke and glass.
    pub fn draw(&self, painter: &Painter, rect: Rect, mood: Mood, time: f64) {
        let Some(look) = mood.look() else {
            return;
        };
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
                additive(look.tint, 0.55 * lamp.power),
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
            let alpha = p.thick * (age / 0.4).min(1.0) * fade.powf(1.5) * 0.75;
            let radius = (p.size + p.grow * age) * rect.height();
            let y = (p.y + p.vy * age).max(0.01);
            painter.add(halo(
                at(p.x + p.vx * age, y),
                radius * 1.4,
                radius,
                Color32::from_rgba_unmultiplied(168, 162, 154, (alpha * 255.0) as u8),
            ));
        }
        // Its glass, falling to the foot of the board and lying there,
        // catching what light there is.
        for g in &self.glass {
            let centre = (g.lamp as f32 + 0.5) / n;
            for sh in &g.shards {
                let t = (time - g.at) as f32 - sh.delay;
                if t < 0.0 {
                    continue;
                }
                let s = (t / FALL).min(1.0);
                let x = centre + (sh.x * (0.3 + 0.7 * s) + sh.drift * s) / n;
                let top = 0.03;
                let y = top + (sh.y - top) * s * s;
                let angle = sh.angle + sh.spin * (1.0 - s);
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
        }
    }
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
        // A new bulb sweeps the glass up.
        assert!(l.replace(dead[0], t));
        assert!(l.glass.is_empty());
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
