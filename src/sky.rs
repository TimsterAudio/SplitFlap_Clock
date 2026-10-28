//! Day and night at a place: morning, day, evening or night, from when the
//! sun rises and sets there.
//!
//! Morning is the hour around sunrise, from half an hour before it to half
//! an hour after; evening the same around sunset; day is between them and
//! night is when the sun is down. Sunrise and sunset come from the usual
//! solar formulas (those of the NOAA's solar calculator), good to a minute
//! or so, which is plenty for a picture on a card. A place with no known
//! location goes by the clock instead: sunrise at six, sunset at six.

use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};

/// Half the morning or the evening, either side of sunrise or sunset.
const HALF: i64 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sky {
    Morning,
    Day,
    Evening,
    Night,
}

impl Sky {
    /// Its flap on the card's drum.
    pub fn symbol(self) -> &'static str {
        match self {
            Sky::Morning => crate::flap::MORNING,
            Sky::Day => crate::flap::DAY,
            Sky::Evening => crate::flap::EVENING,
            Sky::Night => crate::flap::NIGHT,
        }
    }
}

/// When the sun rises and sets on `date` (in UTC) at `lat`, `lon`:
/// `Ok((rise, set))` as minutes from that date's midnight UTC (either can
/// fall outside 0 to 1440), or `Err(true)` if it doesn't set that day, as in
/// a polar summer, and `Err(false)` if it doesn't rise.
pub fn sun_times(date: NaiveDate, lat: f64, lon: f64) -> Result<(f64, f64), bool> {
    // Julian centuries from J2000, at noon of the date.
    let jd = date.num_days_from_ce() as f64 + 1_721_424.5 + 0.5;
    let t = (jd - 2_451_545.0) / 36_525.0;
    let rad = f64::to_radians;
    let l0 = (280.46646 + t * (36000.76983 + t * 0.0003032)).rem_euclid(360.0);
    let m = 357.52911 + t * (35999.05029 - 0.0001537 * t);
    let e = 0.016708634 - t * (0.000042037 + 0.0000001267 * t);
    let c = rad(m).sin() * (1.914602 - t * (0.004817 + 0.000014 * t))
        + rad(2.0 * m).sin() * (0.019993 - 0.000101 * t)
        + rad(3.0 * m).sin() * 0.000289;
    let omega = 125.04 - 1934.136 * t;
    let lambda = l0 + c - 0.00569 - 0.00478 * rad(omega).sin();
    let mean_obliquity =
        23.0 + (26.0 + (21.448 - t * (46.815 + t * (0.00059 - t * 0.001813))) / 60.0) / 60.0;
    let obliquity = mean_obliquity + 0.00256 * rad(omega).cos();
    let decl = (rad(obliquity).sin() * rad(lambda).sin()).asin();
    let y = (rad(obliquity) / 2.0).tan().powi(2);
    let eq_time = 4.0
        * (y * (2.0 * rad(l0)).sin() - 2.0 * e * rad(m).sin()
            + 4.0 * e * y * rad(m).sin() * (2.0 * rad(l0)).cos()
            - 0.5 * y * y * (4.0 * rad(l0)).sin()
            - 1.25 * e * e * (2.0 * rad(m)).sin())
        .to_degrees();
    // The sun's centre 0.833 degrees below the horizon: its rim, allowing
    // for the air bending its light.
    let cos_ha = (rad(90.833).cos() / (rad(lat).cos() * decl.cos())) - rad(lat).tan() * decl.tan();
    if cos_ha < -1.0 {
        return Err(true);
    }
    if cos_ha > 1.0 {
        return Err(false);
    }
    let ha = cos_ha.acos().to_degrees();
    let noon = 720.0 - 4.0 * lon - eq_time;
    Ok((noon - 4.0 * ha, noon + 4.0 * ha))
}

/// The sky at `now` at a place at `at` (latitude, longitude), or, for a
/// place with no location, by its clock: `local` minutes past its midnight.
pub fn sky(now: DateTime<Utc>, at: Option<(f64, f64)>, local: i64) -> Sky {
    let Some((lat, lon)) = at else {
        return by_clock(local);
    };
    let today = now.date_naive();
    let midnight = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let minute = (now - midnight).num_seconds() as f64 / 60.0;
    let mut up = None;
    // Yesterday's and tomorrow's too: a place far from Greenwich can have
    // its sunset, by UTC, on the day before or after.
    for d in -1..=1 {
        let date = today + Duration::days(d);
        let shift = d as f64 * 1440.0;
        match sun_times(date, lat, lon) {
            Ok((rise, set)) => {
                let (rise, set) = (rise + shift, set + shift);
                if (minute - rise).abs() <= HALF as f64 {
                    return Sky::Morning;
                }
                if (minute - set).abs() <= HALF as f64 {
                    return Sky::Evening;
                }
                if minute > rise && minute < set {
                    up = Some(true);
                }
            }
            Err(always) => {
                if d == 0 {
                    up = Some(up.unwrap_or(false) || always);
                }
            }
        }
    }
    if up == Some(true) {
        Sky::Day
    } else {
        Sky::Night
    }
}

/// By the clock alone: sunrise at six in the morning and sunset at six in
/// the evening.
fn by_clock(local: i64) -> Sky {
    let m = local.rem_euclid(1440);
    match m {
        _ if (m - 6 * 60).abs() <= HALF => Sky::Morning,
        _ if (m - 18 * 60).abs() <= HALF => Sky::Evening,
        _ if m > 6 * 60 && m < 18 * 60 => Sky::Day,
        _ => Sky::Night,
    }
}

/// Minutes past midnight of a local time.
pub fn minutes(t: chrono::NaiveDateTime) -> i64 {
    (t.hour() * 60 + t.minute()) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, mo, d)
            .unwrap()
            .and_hms_opt(h, mi, 0)
            .unwrap()
            .and_utc()
    }

    fn hm(minutes: f64) -> (i64, i64) {
        let m = minutes.round() as i64;
        (m.div_euclid(60), m.rem_euclid(60))
    }

    const LONDON: (f64, f64) = (51.5083, -0.1253);

    #[test]
    fn sunrise_and_sunset_in_london() {
        // Midsummer: about 04:43 and 21:21 summer time, so 03:43 and 20:21 UTC.
        let (rise, set) = sun_times(
            NaiveDate::from_ymd_opt(2026, 6, 21).unwrap(),
            LONDON.0,
            LONDON.1,
        )
        .unwrap();
        let (rh, rm) = hm(rise);
        let (sh, sm) = hm(set);
        assert!(rh == 3 && (40..=46).contains(&rm), "rise {rh}:{rm}");
        assert!(sh == 20 && (18..=24).contains(&sm), "set {sh}:{sm}");
        // Midwinter: about 08:04 and 15:53.
        let (rise, set) = sun_times(
            NaiveDate::from_ymd_opt(2026, 12, 21).unwrap(),
            LONDON.0,
            LONDON.1,
        )
        .unwrap();
        assert_eq!(hm(rise).0, 8);
        assert!(hm(rise).1 <= 8);
        assert_eq!(hm(set).0, 15);
        assert!(hm(set).1 >= 49);
    }

    #[test]
    fn the_sky_over_london_through_a_summer_day() {
        let at = Some(LONDON);
        assert_eq!(sky(utc(2026, 6, 21, 2, 0), at, 0), Sky::Night);
        assert_eq!(sky(utc(2026, 6, 21, 3, 20), at, 0), Sky::Morning); // 23 min before sunrise
        assert_eq!(sky(utc(2026, 6, 21, 4, 10), at, 0), Sky::Morning); // 27 min after
        assert_eq!(sky(utc(2026, 6, 21, 4, 20), at, 0), Sky::Day);
        assert_eq!(sky(utc(2026, 6, 21, 12, 0), at, 0), Sky::Day);
        assert_eq!(sky(utc(2026, 6, 21, 20, 0), at, 0), Sky::Evening);
        assert_eq!(sky(utc(2026, 6, 21, 21, 0), at, 0), Sky::Night);
    }

    #[test]
    fn sunset_across_midnight_utc() {
        // Los Angeles in summer sets after 03:00 UTC, on the next UTC day.
        let la = Some((34.0522, -118.2428));
        assert_eq!(sky(utc(2026, 6, 22, 3, 0), la, 0), Sky::Evening);
        assert_eq!(sky(utc(2026, 6, 22, 20, 0), la, 0), Sky::Day);
        // Tokyo's sunrise is before midnight UTC.
        let tokyo = Some((35.6544, 139.7447));
        assert_eq!(sky(utc(2026, 6, 21, 19, 25), tokyo, 0), Sky::Morning);
    }

    #[test]
    fn midnight_sun_and_polar_night() {
        let svalbard = Some((78.0, 16.0));
        assert_eq!(sky(utc(2026, 6, 21, 0, 0), svalbard, 0), Sky::Day);
        assert_eq!(sky(utc(2026, 12, 21, 12, 0), svalbard, 0), Sky::Night);
    }

    #[test]
    fn without_a_location_it_goes_by_the_clock() {
        assert_eq!(sky(utc(2026, 1, 1, 0, 0), None, 5 * 60 + 40), Sky::Morning);
        assert_eq!(sky(utc(2026, 1, 1, 0, 0), None, 12 * 60), Sky::Day);
        assert_eq!(sky(utc(2026, 1, 1, 0, 0), None, 18 * 60 + 30), Sky::Evening);
        assert_eq!(sky(utc(2026, 1, 1, 0, 0), None, 23 * 60), Sky::Night);
        assert_eq!(sky(utc(2026, 1, 1, 0, 0), None, 3 * 60), Sky::Night);
    }
}
