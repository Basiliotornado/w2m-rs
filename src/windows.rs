use std::f32::consts::{E, PI};

pub fn blackman_harris_window(width: usize) -> Vec<f32> {
    let mut window_data = vec![0.0; width];

    for i in 0..width {
        let factor = (PI * i as f32) / width as f32;
        window_data[i] = 0.35875 - 0.4891775 * (2.0 * factor).cos()
            + 0.1365995 * (4.0 * factor).cos()
            - 0.0106411 * (6.0 * factor).cos()
    }

    window_data
}

pub fn hann(width: usize) -> Vec<f32> {
    let mut window_data = vec![0.0; width];

    for i in 0..width {
        let t = i as f32 / width as f32 - 0.5;
        window_data[i] = 0.5 + 0.5 * (2.0 * PI * t).cos()
    }

    window_data
}

pub fn odd_symmetric(width: usize) -> Vec<f32> {
    let b = 2.5f64;
    let mut out = vec![0.0; width];
    for i in 0..width {
        let t = i as f32 / width as f32 * 2.0 - 1.0; // -1 to 1
        out[i] = t * E.powf(-(b.powi(2)) as f32 * t.powi(2) as f32) / 0.282844; // Calculated max for b=2.5
    }
    out
}