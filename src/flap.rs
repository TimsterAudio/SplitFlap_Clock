//! One split-flap card: what it shows, how it steps between symbols, and how
//! it is drawn.
//!
//! A real card sits on a drum of flaps and can only move forward, one flap at
//! a time. Each card here has a drum too, so a change from `B` to `F` flips
//! through `C`, `D` and `E` on the way, quickly, and the last flip lands at
//! normal speed.

use std::collections::VecDeque;
use std::f32::consts::PI;

use egui::epaint::{ClippedShape, Primitive};
use egui::{pos2, Color32, Context, CornerRadius, FontId, Painter, Rect, Shape};

/// The duration of a flip that lands on the wanted symbol, in seconds.
const FLIP: f64 = 0.30;
/// The duration of a flip on the way to it.
const FAST: f64 = 0.07;

const ALPHA: &str = " ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-+:/'&,()";

/// The symbols a card can show, in the order they come round.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Drum {
    /// 0 to 9: the units of hours, minutes and seconds.
    Digit,
    /// 0 to 5: the tens of minutes and seconds.
    Tens6,
    /// 0 to 2: the tens of hours on a 24-hour clock.
    Tens3,
    /// Blank or 1: the tens of hours on a 12-hour clock.
    Tens12,
    /// AM and PM.
    AmPm,
    /// Letters, digits and a little punctuation, for names and dates.
    Alpha,
}

impl Drum {
    fn len(self) -> usize {
        match self {
            Drum::Digit => 10,
            Drum::Tens6 => 6,
            Drum::Tens3 => 3,
            Drum::Tens12 => 2,
            Drum::AmPm => 2,
            Drum::Alpha => ALPHA.len(),
        }
    }

    fn symbol(self, i: usize) -> &'static str {
        match self {
            Drum::Digit => &"0123456789"[i..=i],
            Drum::Tens6 => &"012345"[i..=i],
            Drum::Tens3 => &"012"[i..=i],
            Drum::Tens12 => &" 1"[i..=i],
            Drum::AmPm => ["AM", "PM"][i],
            Drum::Alpha => &ALPHA[i..=i],
        }
    }

    fn index(self, s: &str) -> Option<usize> {
        (0..self.len()).find(|&i| self.symbol(i) == s)
    }

    /// The flaps that come round on the way from `from` to `to`, ending with
    /// `to`. A symbol the drum doesn't have is reached in one flip.
    pub fn steps(self, from: &str, to: &str) -> Vec<String> {
        let Some(b) = self.index(to) else {
            return vec![to.to_string()];
        };
        // A card that shows something off the drum (a blank digit card when
        // the clock starts) comes round from the drum's first flap.
        let mut i = match self.index(from) {
            Some(a) if a == b => return Vec::new(),
            Some(a) => (a + 1) % self.len(),
            None => 0,
        };
        let mut out = Vec::new();
        loop {
            out.push(self.symbol(i).to_string());
            if i == b {
                return out;
            }
            i = (i + 1) % self.len();
        }
    }

    /// One whole turn of the drum from `from`, then on to `to`: what a
    /// card does when the board refreshes.
    pub fn turn(self, from: &str, to: &str) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(a) = self.index(from) {
            out.extend((1..=self.len()).map(|k| self.symbol((a + k) % self.len()).to_string()));
        }
        out.extend(self.steps(from, to));
        out
    }
}

/// The colours a card is drawn in.
#[derive(Clone, Copy)]
pub struct Look {
    pub top: Color32,
    pub bottom: Color32,
    pub split: Color32,
    pub hinge: Color32,
    pub ink: Color32,
}

pub struct Flap {
    drum: Drum,
    /// What the card showed before the flip under way.
    prev: String,
    /// What the flip under way lands on.
    cur: String,
    /// What the card is on its way to.
    target: String,
    queue: VecDeque<String>,
    start: f64,
    dur: f64,
    /// No flipping before this time, so a row of new cards can ripple.
    hold: f64,
}

impl Flap {
    /// A blank card that starts flipping `delay` seconds after `now`.
    pub fn new(drum: Drum, now: f64, delay: f64) -> Self {
        Self {
            drum,
            prev: " ".into(),
            cur: " ".into(),
            target: " ".into(),
            queue: VecDeque::new(),
            start: 0.0,
            dur: 0.0,
            hold: now + delay,
        }
    }

    pub fn drum(&self) -> Drum {
        self.drum
    }

    pub fn set(&mut self, target: &str) {
        if self.target == target {
            return;
        }
        self.target = target.to_string();
        self.queue = self.drum.steps(&self.cur, target).into();
    }

    /// Spins the card once round its drum, starting `delay` seconds after
    /// `now`, and lands it on `target`.
    pub fn turn(&mut self, target: &str, now: f64, delay: f64) {
        self.target = target.to_string();
        self.queue = self.drum.turn(&self.cur, target).into();
        self.hold = self.hold.max(now + delay);
    }

    fn progress(&self, now: f64) -> f32 {
        if self.dur <= 0.0 {
            1.0
        } else {
            ((now - self.start) / self.dur).clamp(0.0, 1.0) as f32
        }
    }

    /// Moves the card on to its next flap when the current flip is done.
    pub fn tick(&mut self, now: f64) {
        if now < self.hold || self.progress(now) < 1.0 {
            return;
        }
        if let Some(next) = self.queue.pop_front() {
            self.prev = std::mem::replace(&mut self.cur, next);
            self.start = now;
            self.dur = if self.queue.is_empty() { FLIP } else { FAST };
        }
    }

    pub fn busy(&self, now: f64) -> bool {
        !self.queue.is_empty() || self.progress(now) < 1.0
    }

    pub fn draw(&self, ctx: &Context, painter: &Painter, rect: Rect, font: &FontId, look: &Look) {
        let now = ctx.input(|i| i.time);
        let p = self.progress(now);
        let flipping = p < 1.0;

        let h = rect.height();
        let r = (h * 0.07).round().max(1.0) as u8;
        let gap = (h * 0.025).max(1.0);
        let mid = rect.center().y;
        let top = Rect::from_min_max(rect.min, pos2(rect.max.x, mid - gap / 2.0));
        let bottom = Rect::from_min_max(pos2(rect.min.x, mid + gap / 2.0), rect.max);
        let top_r = CornerRadius {
            nw: r,
            ne: r,
            sw: 0,
            se: 0,
        };
        let bottom_r = CornerRadius {
            nw: 0,
            ne: 0,
            sw: r,
            se: r,
        };

        // The split, and the card's halves behind any moving flap: the new
        // symbol's top is already showing; the old one's bottom stays until
        // the flap lands on it.
        painter.rect_filled(rect.shrink2(egui::vec2(0.0, h * 0.02)), r, look.split);
        painter.rect_filled(top, top_r, look.top);
        painter.rect_filled(bottom, bottom_r, look.bottom);
        let under = if flipping { &self.prev } else { &self.cur };
        painter
            .with_clip_rect(top.intersect(painter.clip_rect()))
            .add(glyph(painter, rect, &self.cur, font, look.ink));
        painter
            .with_clip_rect(bottom.intersect(painter.clip_rect()))
            .add(glyph(painter, rect, under, font, look.ink));

        if flipping {
            if p < 0.5 {
                // The old top folds down towards the hinge, darkening as it
                // turns away from the light, and shades the half below it.
                let s = (PI * p).cos();
                let hinge = top.max.y;
                let flap = Rect::from_min_max(pos2(top.min.x, hinge - top.height() * s), top.max);
                let shade = 0.45 + 0.55 * s;
                painter.rect_filled(
                    bottom,
                    bottom_r,
                    Color32::from_black_alpha((70.0 * (1.0 - s)) as u8),
                );
                painter.rect_filled(flap, top_r, darken(look.top, shade));
                squashed(
                    ctx,
                    &painter.with_clip_rect(flap.intersect(painter.clip_rect())),
                    glyph(painter, rect, &self.prev, font, look.ink),
                    hinge,
                    s,
                    shade,
                );
            } else {
                // The new bottom swings down from the hinge.
                let s = -(PI * p).cos();
                let hinge = bottom.min.y;
                let flap =
                    Rect::from_min_max(bottom.min, pos2(bottom.max.x, hinge + bottom.height() * s));
                let shade = 0.6 + 0.4 * s;
                painter.rect_filled(flap, bottom_r, darken(look.bottom, shade));
                squashed(
                    ctx,
                    &painter.with_clip_rect(flap.intersect(painter.clip_rect())),
                    glyph(painter, rect, &self.cur, font, look.ink),
                    hinge,
                    s,
                    shade,
                );
            }
        }

        // The pins the flaps hang on.
        let pin = egui::vec2((rect.width() * 0.05).max(1.5), gap * 2.6);
        for x in [rect.min.x + pin.x / 2.0, rect.max.x - pin.x / 2.0] {
            painter.rect_filled(Rect::from_center_size(pos2(x, mid), pin), 1.0, look.hinge);
        }
    }
}

/// `text` centred on the card, with every symbol on the same baseline.
fn glyph(painter: &Painter, card: Rect, text: &str, font: &FontId, ink: Color32) -> Shape {
    let galley = painter.layout_no_wrap(text.to_string(), font.clone(), ink);
    // Centre on the ink of a digit rather than on the line, which has room
    // for descenders below.
    let reference = painter.layout_no_wrap("0".into(), font.clone(), ink);
    let ink_mid = reference
        .rows
        .first()
        .map(|row| row.pos.y + row.visuals.mesh_bounds.center().y)
        .unwrap_or(reference.size().y / 2.0);
    let pos = pos2(
        card.center().x - galley.size().x / 2.0,
        card.center().y - ink_mid,
    );
    Shape::galley(pos, galley, ink)
}

/// Draws `shape` squashed towards the line at `hinge` by `s`, as a flap
/// seen turning edge-on.
fn squashed(ctx: &Context, painter: &Painter, shape: Shape, hinge: f32, s: f32, shade: f32) {
    let clipped = ClippedShape {
        clip_rect: Rect::EVERYTHING,
        shape,
    };
    for prim in ctx.tessellate(vec![clipped], ctx.pixels_per_point()) {
        if let Primitive::Mesh(mut mesh) = prim.primitive {
            for v in &mut mesh.vertices {
                v.pos.y = hinge + (v.pos.y - hinge) * s;
                v.color = darken(v.color, shade);
            }
            painter.add(Shape::mesh(mesh));
        }
    }
}

fn darken(c: Color32, k: f32) -> Color32 {
    let f = |x: u8| (x as f32 * k).round() as u8;
    Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_go_forward_and_wrap() {
        assert_eq!(Drum::Digit.steps("3", "4"), ["4"]);
        assert_eq!(Drum::Digit.steps("9", "0"), ["0"]);
        assert_eq!(Drum::Tens6.steps("5", "0"), ["0"]);
        assert_eq!(Drum::Alpha.steps("B", "E"), ["C", "D", "E"]);
        assert!(Drum::Digit.steps("7", "7").is_empty());
    }

    #[test]
    fn a_refresh_turns_the_whole_drum() {
        assert_eq!(Drum::Tens3.turn("1", "1"), ["2", "0", "1"]);
        assert_eq!(Drum::Tens3.turn("1", "2"), ["2", "0", "1", "2"]);
        assert_eq!(Drum::Alpha.turn("A", "A").len(), Drum::Alpha.len());
        // A card that is still blank just comes round to its symbol.
        assert_eq!(Drum::Digit.turn(" ", "1"), ["0", "1"]);
    }

    #[test]
    fn a_blank_card_comes_round_from_the_start() {
        assert_eq!(Drum::Digit.steps(" ", "2"), ["0", "1", "2"]);
        assert_eq!(Drum::AmPm.steps(" ", "PM"), ["AM", "PM"]);
    }

    #[test]
    fn symbols_off_the_drum_take_one_flip() {
        assert_eq!(Drum::Alpha.steps("A", "é"), ["é"]);
    }

    #[test]
    fn a_card_lands_on_its_target() {
        let mut f = Flap::new(Drum::Alpha, 0.0, 0.0);
        f.set("C");
        let mut t = 0.0;
        while f.busy(t) || t == 0.0 {
            f.tick(t);
            t += 0.01;
            assert!(t < 5.0);
        }
        assert_eq!(f.cur, "C");
    }
}
