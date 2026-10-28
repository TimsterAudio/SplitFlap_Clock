//! Alarms: when they ring, and by whose clock.
//!
//! An alarm rings at its time on its days, by the time of its place: your
//! own, or one of the places on the board. It rings once a day at most, and
//! if the clock wasn't running or the computer was asleep when it was due,
//! it still rings if that was less than [`GRACE`] minutes ago.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// How late, in minutes, an alarm may still ring.
pub const GRACE: i64 = 10;
/// How long a snooze lasts, in minutes. Nine, like every snooze button.
pub const SNOOZE: i64 = 9;
/// How long an alarm rings, in minutes, before it gives up by itself.
pub const RING_FOR: i64 = 30;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alarm {
    pub id: u64,
    pub on: bool,
    pub hour: u32,
    pub minute: u32,
    /// Its name on the board when it rings.
    pub label: String,
    /// What it's for, which the board shows when it rings, where it would
    /// otherwise say NOW.
    pub reason: String,
    /// The days it rings, Monday first. With none ticked it rings once, at
    /// the next time it comes round, and then turns itself off.
    pub days: [bool; 7],
    /// The place whose time it follows, by its id, or `None` for yours.
    pub place: Option<u64>,
    /// The day, by its place's calendar, it last rang: (year, day of year).
    pub rang: Option<(i32, u32)>,
}

impl Default for Alarm {
    fn default() -> Self {
        Self {
            id: 0,
            on: true,
            hour: 7,
            minute: 0,
            label: "ALARM".into(),
            reason: String::new(),
            days: [false; 7],
            place: None,
            rang: None,
        }
    }
}

impl Alarm {
    pub fn once(&self) -> bool {
        !self.days.iter().any(|&d| d)
    }

    /// The day it is due for, if it is due to ring at `now`, its place's
    /// time.
    pub fn due(&self, now: NaiveDateTime) -> Option<NaiveDate> {
        if !self.on {
            return None;
        }
        let today = now.date();
        // Yesterday too, for an alarm just before midnight that is still
        // within its grace just after.
        for day in [today, today.pred_opt()?] {
            let at = day.and_hms_opt(self.hour, self.minute, 0)?;
            let rang = self.rang == Some((day.year(), day.ordinal()));
            let on_day = self.once() || self.days[day.weekday().num_days_from_monday() as usize];
            if !rang && on_day && now >= at && now < at + Duration::minutes(GRACE) {
                return Some(day);
            }
        }
        None
    }

    /// Notes that it has rung for `day`; an alarm that rings once turns off.
    pub fn rung(&mut self, day: NaiveDate) {
        self.rang = Some((day.year(), day.ordinal()));
        if self.once() {
            self.on = false;
        }
    }

    /// Its days in words, for Settings.
    pub fn days_text(&self) -> &'static str {
        match self.days {
            [false, false, false, false, false, false, false] => "Once",
            [true, true, true, true, true, true, true] => "Every day",
            [true, true, true, true, true, false, false] => "Weekdays",
            [false, false, false, false, false, true, true] => "Weekends",
            _ => "On the days ticked",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(d: u32, h: u32, m: u32) -> NaiveDateTime {
        // October 2026: the 26th is a Monday.
        NaiveDate::from_ymd_opt(2026, 10, d)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    fn alarm(h: u32, m: u32) -> Alarm {
        Alarm {
            hour: h,
            minute: m,
            ..Alarm::default()
        }
    }

    #[test]
    fn it_rings_at_its_time_and_a_little_after() {
        let a = alarm(7, 30);
        assert_eq!(a.due(at(27, 7, 29)), None);
        assert!(a.due(at(27, 7, 30)).is_some());
        assert!(a.due(at(27, 7, 39)).is_some());
        assert_eq!(a.due(at(27, 7, 40)), None);
    }

    #[test]
    fn it_rings_once_a_day() {
        let mut a = alarm(7, 30);
        a.days = [true; 7];
        let day = a.due(at(27, 7, 30)).unwrap();
        a.rung(day);
        assert!(a.on);
        assert_eq!(a.due(at(27, 7, 31)), None);
        assert!(a.due(at(28, 7, 30)).is_some());
    }

    #[test]
    fn once_turns_itself_off() {
        let mut a = alarm(7, 30);
        let day = a.due(at(27, 7, 30)).unwrap();
        a.rung(day);
        assert!(!a.on);
        assert_eq!(a.due(at(28, 7, 30)), None);
    }

    #[test]
    fn only_on_its_days() {
        let mut a = alarm(7, 30);
        a.days = [true, true, true, true, true, false, false];
        assert_eq!(a.days_text(), "Weekdays");
        assert!(a.due(at(30, 7, 30)).is_some()); // Friday
        assert_eq!(a.due(at(31, 7, 30)), None); // Saturday
    }

    #[test]
    fn just_before_midnight_still_rings_just_after() {
        let a = alarm(23, 58);
        let day = a.due(at(28, 0, 3)).unwrap();
        assert_eq!(day, at(27, 0, 0).date());
    }

    #[test]
    fn off_never_rings() {
        let a = Alarm {
            on: false,
            ..alarm(7, 30)
        };
        assert_eq!(a.due(at(27, 7, 30)), None);
    }
}
