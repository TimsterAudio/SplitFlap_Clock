"""Writes src/places.rs: where each time zone's city is, from the IANA time
zone database's zone1970.tab and zone.tab (coordinates) and its links (old
and other names for the same zones).

    python3 tools/zone_places.py /usr/share/zoneinfo
"""
import pathlib
import re
import sys

zi = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "/usr/share/zoneinfo")


def degrees(s):
    # +DDMM, +DDMMSS, +DDDMM or +DDDMMSS
    sign = -1 if s[0] == "-" else 1
    s = s[1:]
    d = 2 if len(s) in (4, 6) else 3
    deg, rest = int(s[:d]), s[d:]
    mins = int(rest[:2])
    secs = int(rest[2:4]) if len(rest) >= 4 else 0
    return sign * (deg + mins / 60 + secs / 3600)


places = {}
for tab in ("zone.tab", "zone1970.tab"):
    for line in (zi / tab).read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        cols = line.split("\t")
        m = re.fullmatch(r"([+-]\d+)([+-]\d+)", cols[1])
        places[cols[2]] = (degrees(m.group(1)), degrees(m.group(2)))

version = "unknown"
for line in (zi / "tzdata.zi").read_text().splitlines():
    if line.startswith("# version"):
        version = line.split()[-1]
    if line.startswith("L "):
        _, target, name = line.split()
        if target in places and name not in places:
            places[name] = places[target]

rows = "\n".join(f'    ("{n}", {lat:.4f}, {lon:.4f}),' for n, (lat, lon) in sorted(places.items()))
out = f'''//! Where each time zone's city is, as latitude and longitude in degrees
//! (north and east positive), for working out its sunrise and sunset.
//!
//! Made by `tools/zone_places.py` from the IANA time zone database,
//! version {version}; don't edit it by hand.

#[rustfmt::skip]
const PLACES: &[(&str, f64, f64)] = &[
{rows}
];

/// The latitude and longitude of time zone `tz`'s city, if it has one.
pub fn coords(tz: &str) -> Option<(f64, f64)> {{
    let i = PLACES.binary_search_by(|(n, _, _)| n.cmp(&tz)).ok()?;
    let (_, lat, lon) = PLACES[i];
    Some((lat, lon))
}}
'''
pathlib.Path("src/places.rs").write_text(out)
print(len(places), "places, tz", version)
