"""Writes src/cities.rs: the world's cities and towns, each with its time
zone, country and place, for finding a place by name in Settings. From the
city-timezones package's cityMap.json (MIT licence), itself from Natural
Earth's populated places:

    npm pack city-timezones && tar xzf city-timezones-*.tgz
    python3 tools/cities.py package/data/cityMap.json /usr/share/zoneinfo

Cities in a time zone this system doesn't know are left out, and so are
repeats of the same city in the same place.
"""
import json
import pathlib
import sys
import unicodedata

data = json.loads(pathlib.Path(sys.argv[1]).read_text())
zi = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "/usr/share/zoneinfo")


def plain(s):
    """Lower case, without accents, for matching what's typed."""
    s = unicodedata.normalize("NFKD", s)
    return "".join(c for c in s if not unicodedata.combining(c)).lower()


def rust(s):
    return json.dumps(s, ensure_ascii=False)


seen = set()
rows = []
for c in data:
    tz = c.get("timezone") or ""
    if not tz or not (zi / tz).is_file():
        continue
    name = (c.get("city") or "").strip()
    if not name:
        continue
    region = (c.get("province") or "").strip()
    country = (c.get("country") or "").strip()
    key = (plain(name), plain(region), plain(country))
    if key in seen:
        continue
    seen.add(key)
    ascii_name = (c.get("city_ascii") or name).strip()
    words = " ".join(dict.fromkeys([plain(name), plain(ascii_name)]))
    rows.append(
        (
            name,
            ascii_name,
            region,
            country,
            tz,
            float(c["lat"]),
            float(c["lng"]),
            max(int(c.get("pop") or 0), 0),
            words,
            plain(f"{region} {country}").strip(),
        )
    )

# Biggest first, so the likeliest match comes first.
rows.sort(key=lambda r: (-r[7], r[0]))

out = [
    "//! The world's cities and towns, with their time zones, for finding a",
    "//! place by name. Made by `tools/cities.py` from the city-timezones",
    "//! package (MIT licence), from Natural Earth's populated places; don't",
    "//! edit it by hand. Biggest first.",
    "",
    "// Some towns sit at a latitude that looks like pi.",
    "#![allow(clippy::approx_constant)]",
    "",
    "pub struct City {",
    "    pub name: &'static str,",
    "    /// The name in plain letters, for the board's cards.",
    "    pub ascii: &'static str,",
    "    /// The state, province or region it's in, if any.",
    "    pub region: &'static str,",
    "    pub country: &'static str,",
    "    pub tz: &'static str,",
    "    pub lat: f64,",
    "    pub lon: f64,",
    "    /// Its names, lower case and without accents, for matching.",
    "    pub words: &'static str,",
    "    /// Its region and country, the same way.",
    "    pub around: &'static str,",
    "}",
    "",
    "#[rustfmt::skip]",
    "pub const CITIES: &[City] = &[",
]
for name, ascii_name, region, country, tz, lat, lon, pop, words, around in rows:
    out.append(
        f"    City {{ name: {rust(name)}, ascii: {rust(ascii_name)}, region: {rust(region)}, "
        f"country: {rust(country)}, tz: {rust(tz)}, lat: {lat:.4f}, lon: {lon:.4f}, "
        f"words: {rust(words)}, around: {rust(around)} }},"
    )
out.append("];")
out.append("")
root = pathlib.Path(__file__).resolve().parent.parent
(root / "src" / "cities.rs").write_text("\n".join(out))
print(f"{len(rows)} cities")
