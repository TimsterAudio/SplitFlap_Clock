//! The board's lighting: a row of incandescent lamps just above it, each
//! throwing a warm pool of light down the board, brightest near the top and
//! fading away below. Once in a while, days apart, a bulb flickers, then
//! either comes good or dies, leaving its part of the board in shadow until
//! it's replaced.
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
    pub const ALL: [Mood; 4] = [Mood::Off, Mood::Soft, Mood::Warm, Mood::Evening];

    pub fn label(self) -> &'static str {
        match self {
            Mood::Off => "Off",
            Mood::Soft => "Soft lamps",
            Mood::Warm => "Warm lamps",
            Mood::Evening => "Late evening",
        }
    }

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

/// How often bulbs fail.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Failures {
    Never,
    #[default]
    Sometimes,
    Often,
}

impl Failures {
    pub const ALL: [Failures; 3] = [Failures::Never, Failures::Sometimes, Failures::Often];

    pub fn label(self) -> &'static str {
        match self {
            Failures::Never => "Never fail",
            Failures::Sometimes => "Fail every few days",
            Failures::Often => "Fail about once a day",
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
    /// Out until it's replaced, at `until`.
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

pub struct Lighting {
    lamps: Vec<Lamp>,
    rng: Rng,
    /// When the lamps were last moved on.
    last: Option<f64>,
}

impl Lighting {
    pub fn new(rng: Rng) -> Self {
        Self {
            lamps: Vec::new(),
            rng,
            last: None,
        }
    }

    /// Moves the lamps on to `time`, for a board lit by `n` lamps. Whether
    /// anything is changing, so the board should be drawn again soon. The
    /// time since the last call is measured, not taken from the frame rate,
    /// as a still board is only drawn about once a second.
    pub fn update(&mut self, n: usize, time: f64, failures: Failures) -> bool {
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
                                until: time + self.rng.between(20.0, 90.0),
                            };
                            lamp.target = 0.0;
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
                    if time >= until {
                        // A new bulb, which sputters for a moment on.
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
        busy
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

    /// Lights the board at `rect`, drawn already, in `mood`.
    pub fn draw(&self, painter: &Painter, rect: Rect, mood: Mood) {
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
    }
}

/// A colour to add on top, rather than paint over: premultiplied, with no
/// alpha.
fn additive(tint: [f32; 3], amount: f32) -> Color32 {
    let c = |t: f32| (t * amount.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_premultiplied(c(tint[0]), c(tint[1]), c(tint[2]), 0)
}

/// A soft oval glow, `colour` at its centre fading to nothing at its edge.
fn halo(centre: Pos2, rx: f32, ry: f32, colour: Color32) -> Mesh {
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
        l.update(3, 0.0, Failures::Never);
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
        l.update(3, 0.0, Failures::Sometimes);
        l.lamps[1].state = State::Flicker {
            until: 1.0,
            next: 0.0,
            dies: true,
        };
        let mut t = 0.0;
        while t < 3.0 {
            l.update(3, t, Failures::Sometimes);
            t += 0.016;
        }
        assert!(matches!(l.lamps[1].state, State::Dead { .. }));
        assert!(l.lamps[1].power < 0.05);
        let look = Mood::Warm.look().unwrap();
        assert!(l.light(&look, 0.5, 0.1).0 < l.light(&look, 1.0 / 6.0, 0.1).0);
        // And comes back on, in time.
        let mut t = 3.0;
        while t < 100.0 {
            l.update(3, t, Failures::Never);
            t += 0.05;
        }
        assert!(l.lamps[1].power > 0.95);
    }

    #[test]
    fn failures_happen_about_as_often_as_asked() {
        let mut l = Lighting::new(Rng::with_seed(3));
        let mut starts = 0;
        let mut t = 0.0;
        // Thirty days, with a bulb failing about once a day.
        while t < 30.0 * DAY {
            let before: Vec<bool> = l.lamps.iter().map(|x| x.state == State::Lit).collect();
            l.update(4, t, Failures::Often);
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
