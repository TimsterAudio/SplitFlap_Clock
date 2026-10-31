//! Gives the Windows exe its icon, drawn by the same code as the window's.

#[path = "src/icon.rs"]
#[allow(dead_code)]
mod icon;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    release_date();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico = out.join("clock.ico");
    std::fs::write(&ico, ico_bytes(&icon::pixels(), icon::SIZE)).unwrap();
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().unwrap());
    res.set("FileDescription", "Split-Flap Clock");
    res.set("ProductName", "Split-Flap Clock");
    res.compile().expect("compile the Windows resources");
}

/// Tells the app the date its version was released, from the changelog's
/// "## X.Y.Z – YYYY-MM-DD" heading, or nothing for a version not released,
/// or a test build of changes since, with an Unreleased section on top.
fn release_date() {
    println!("cargo:rerun-if-changed=CHANGELOG.md");
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let changelog = std::fs::read_to_string("CHANGELOG.md").unwrap_or_default();
    let unreleased = changelog
        .lines()
        .find(|l| l.starts_with("## "))
        .is_some_and(|l| l.trim() == "## Unreleased");
    let date = changelog
        .lines()
        .find_map(|l| l.strip_prefix(&format!("## {version} ")))
        .and_then(|rest| rest.split_whitespace().last())
        .unwrap_or("");
    let date = if unreleased { "" } else { date };
    println!("cargo:rustc-env=RELEASE_DATE={date}");
}

/// A one-image .ico holding a 32-bit bitmap.
fn ico_bytes(rgba: &[u8], size: u32) -> Vec<u8> {
    let mask_row = size.div_ceil(32) * 4;
    let image_len = 40 + size * size * 4 + mask_row * size;
    let mut b = Vec::new();
    // ICONDIR, then one ICONDIRENTRY.
    b.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    b.extend_from_slice(&[size as u8, size as u8, 0, 0, 1, 0, 32, 0]);
    b.extend_from_slice(&image_len.to_le_bytes());
    b.extend_from_slice(&22u32.to_le_bytes());
    // BITMAPINFOHEADER; the height counts the mask too.
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(&(size * 2).to_le_bytes());
    b.extend_from_slice(&[1, 0, 32, 0]);
    b.extend_from_slice(&[0; 24]);
    // Pixels bottom row first, as BGRA.
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            b.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    // An empty mask: the alpha channel does the work.
    b.resize(b.len() + (mask_row * size) as usize, 0);
    b
}
