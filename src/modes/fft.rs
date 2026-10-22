use crate::{mid, windows};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn f64_vec_to_complex(vec: Vec<f64>) -> Vec<Complex<f64>> {
    vec.iter().map(|&e| Complex { re: e, im: 0.0 }).collect()
}

pub struct Spectrogram {
    pub data: Vec<Vec<f64>>,
    pub time: Vec<f64>,
    pub frequencies: Vec<f64>,
}

impl Spectrogram {
    pub fn new(
        sound: &[Complex<f64>],
        bin_size: usize,
        bin_overlap: usize,
        sample_rate: usize,
        window: &[f64],
    ) -> Self {
        let step_size = bin_size - bin_overlap;
        let total_steps = (sound.len() - bin_size) / step_size + 1;

        let mut fft_planner = FftPlanner::new();
        let fft_plan = fft_planner.plan_fft_forward(bin_size);

        let mut scratch: Vec<Complex<f64>> =
            vec![Complex { re: 0.0, im: 0.0 }; fft_plan.get_inplace_scratch_len()];

        let mut data: Vec<Vec<f64>> = vec![vec![]; total_steps];
        let mut time: Vec<f64> = vec![0.0; total_steps];
        let mut frequencies: Vec<f64> = vec![0.0; bin_size / 2];

        for i in 0..total_steps {
            let sample = step_size * i;
            let mut buffer = sound[sample..(bin_size + sample)].to_vec();

            for j in 0..bin_size {
                buffer[j] *= window[j];
            }

            fft_plan.process_with_scratch(&mut buffer, &mut scratch);

            data[i] = buffer
                .clone()
                .iter()
                .map(|num| (num.re.powf(2.0) + num.im.powf(2.0)).sqrt() / bin_size as f64)
                .collect();

            time[i] = sample as f64 / sample_rate as f64;
        }

        for i in 0..bin_size / 2 {
            frequencies[i] = sample_rate as f64 * i as f64 / (bin_size as f64 - 1.0);
        }

        Self {
            data,
            time,
            frequencies,
        }
    }
}

pub struct Multispectra {
    sound_data: Vec<Complex<f64>>,
}

impl Multispectra {
    pub fn new(sound_data: Vec<f64>) -> Self {
        Self {
            sound_data: f64_vec_to_complex(sound_data),
        }
    }
}

impl crate::ToNotes for Multispectra {
    fn to_notes(&self, c_cfg: &crate::ConvertCfg) -> Vec<Vec<crate::Note>> {
        let mut note_data: Vec<Vec<crate::Note>> = vec![vec![]; c_cfg.tracks];

        for note in 0..128 {
            let mult = note as usize / 2 + 1; //32;
            let bins = c_cfg.samplerate as f64 / mid::piano_key_to_frequency(note) * mult as f64;

            let mut mult = 36;

            let mut bin_size =
                c_cfg.samplerate as f64 / mid::piano_key_to_frequency(note) * mult as f64;
            let minimum_bin_size = (32.0 * (128.0 - note as f64)) + 128.0;
            while bin_size < minimum_bin_size {
                mult += 12;
                bin_size =
                    c_cfg.samplerate as f64 / mid::piano_key_to_frequency(note) * mult as f64;
            }

            let bins = bin_size;

            let bin_r = bins.round() as usize;
            let overlap = (bins * 0.5).round() as usize; // + bin_r / 4;

            println!("Rendering note: {note}, {mult}, {bin_r}");

            let window = windows::blackman_harris_window(bin_r);
            let odd_window = windows::odd_symmetric(bin_r);

            let spec =
                Spectrogram::new(&self.sound_data, bin_r, overlap, c_cfg.samplerate, &window);

            let spec_odd = Spectrogram::new(
                &self.sound_data,
                bin_r,
                overlap,
                c_cfg.samplerate,
                &odd_window,
            );

            for i in 0..spec.data.len() - 1 {
                let vel = (spec.data[i][mult] - spec_odd.data[i][mult] * 0.166).powf(0.5) * 300.0;

                if vel < 1.0 || vel > 128.0 {
                    continue;
                }

                let track = (vel.clamp(0.0, 128.0) / 128.0 * c_cfg.tracks as f64) as usize;

                let start = mid::time_to_tick(spec.time[i], c_cfg.ppqn as f64, 120.0)
                    + spec.time[1] * c_cfg.ppqn as f64 * 2.0;

                let end = mid::time_to_tick(spec.time[i + 1], c_cfg.ppqn as f64, 120.0)
                    + spec.time[1] * c_cfg.ppqn as f64 * 2.0;

                note_data[track].push(crate::Note {
                    time: start as usize,
                    note,
                    vel: vel as u8,
                    channel: c_cfg.channel,
                });
                note_data[track].push(crate::Note {
                    time: end as usize,
                    note,
                    vel: 0,
                    channel: c_cfg.channel,
                });
            }
        }

        note_data
    }
}