//! Colour-space helpers (no external deps).
use lamparray::Rgb;

/// Convert HSV (`h` in degrees 0..360, `s`/`v` in 0..1) to [`Rgb`].
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
    let c = v * s;
    let h6 = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to_u8 = |f: f64| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(to_u8(r1), to_u8(g1), to_u8(b1))
}
