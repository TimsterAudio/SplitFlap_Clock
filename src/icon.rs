//! The icon, drawn in code: two flap cards with yellow digits. The build
//! script uses it too, for the Windows exe's icon.

pub const SIZE: u32 = 64;

/// RGBA pixels, `SIZE` × `SIZE`.
pub fn pixels() -> Vec<u8> {
    let n = SIZE as f32;
    let mut out = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Two cards side by side, with rounded corners.
            let card = |x0: f32| rounded(fx - x0, fy - 6.0, 26.0, 52.0, 5.0);
            let a = card(4.0).max(card(34.0));
            let mut rgb = [34u8, 34, 33];
            if (fy - n / 2.0).abs() < 1.2 {
                rgb = [4, 4, 4];
            }
            // A "1" on the left card and a "2"-ish block on the right, in
            // the clock's yellow.
            let one = (fx - 17.0).abs() < 3.0 && (14.0..50.0).contains(&fy);
            let two = (fx - 47.0).abs() < 8.0
                && ((14.0..19.0).contains(&fy)
                    || (29.5..34.5).contains(&fy)
                    || (45.0..50.0).contains(&fy))
                || ((50.0..55.0).contains(&fx) && (14.0..34.0).contains(&fy))
                || ((39.0..44.0).contains(&fx) && (30.0..50.0).contains(&fy));
            if (one || two) && (fy - n / 2.0).abs() >= 1.2 {
                rgb = [244, 196, 48];
            }
            out.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (a * 255.0) as u8]);
        }
    }
    out
}

/// Coverage of a rounded rectangle at (`x`, `y`) relative to its corner.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let dx = (x - w / 2.0).abs() - (w / 2.0 - r);
    let dy = (y - h / 2.0).abs() - (h / 2.0 - r);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - r;
    (0.5 - outside).clamp(0.0, 1.0)
}
