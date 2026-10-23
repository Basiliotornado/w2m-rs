use crate::{mid, windows};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn f64_vec_to_complex(vec: Vec<f64>) -> Vec<Complex<f64>> {
    vec.iter().map(|&e| Complex { re: e, im: 0.0 }).collect()
}

#[derive(Default)]
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

#[derive(Clone)]
pub struct Multispectra {
    sound_data: Vec<Complex<f64>>,
    cfg: crate::arguments::MultispectraCfg,
}

impl Multispectra {
    pub fn new(sound_data: Vec<f64>, cfg: crate::arguments::MultispectraCfg) -> Self {
        Self {
            sound_data: f64_vec_to_complex(sound_data),
            cfg,
        }
    }
}

fn notes_range<T>(
    data: &[Complex<f64>],
    range: T,
    c_cfg: &crate::ConvertCfg,
    m_cfg: &crate::arguments::MultispectraCfg,
) -> crate::MidiFile
where
    T: Iterator<Item = usize>,
{
    let mut note_data: crate::MidiFile = vec![vec![]; c_cfg.tracks];
    for note in range {
        let mut mult = m_cfg.mult;

        let mut bins =
            c_cfg.samplerate as f64 / mid::piano_key_to_frequency(note as u8) * mult as f64;
        let minimum_bin_size = (m_cfg.m as f64 * (128.0 - note as f64)) + m_cfg.b as f64;
        while bins < minimum_bin_size {
            mult += m_cfg.mult_diff;
            bins = c_cfg.samplerate as f64 / mid::piano_key_to_frequency(note as u8) * mult as f64;
        }

        let bin_r = bins.round() as usize;
        let overlap = (bins * m_cfg.overlap).round() as usize; // + bin_r / 4;

        println!("Rendering note: {note}, {mult}, {bin_r}");

        let window = windows::blackman_harris_window(bin_r);
        let spec = Spectrogram::new(data, bin_r, overlap, c_cfg.samplerate, &window);

        let spec_odd;
        if !m_cfg.no_odd {
            let odd_window = windows::odd_symmetric(bin_r);
            spec_odd = Spectrogram::new(data, bin_r, overlap, c_cfg.samplerate, &odd_window);
        } else {
            spec_odd = Spectrogram::default();
        }
        for i in 0..spec.data.len() - 1 {
            let mut vel: f64;
            if !m_cfg.no_odd {
                vel =
                    (spec.data[i][mult] - spec_odd.data[i][mult] * m_cfg.odd_amp).powf(0.5) * 400.0;
            } else {
                vel = (spec.data[i][mult]).powf(0.5) * 400.0
            };
            vel = vel.clamp(0.0, 127.99);

            if vel < 1.0 {
                continue;
            }

            let track = (vel / 128.0 * c_cfg.tracks as f64) as usize;

            let start = mid::time_to_tick(spec.time[i], c_cfg.ppqn as f64, c_cfg.bpm)
                + spec.time[1] * c_cfg.ppqn as f64 * 2.0;

            let end = mid::time_to_tick(spec.time[i + 1], c_cfg.ppqn as f64, c_cfg.bpm)
                + spec.time[1] * c_cfg.ppqn as f64 * 2.0;

            note_data[track].push(crate::Note {
                time: start as usize,
                note: note as u8,
                vel: vel as u8,
                channel: c_cfg.channel,
            });
            note_data[track].push(crate::Note {
                time: end as usize - 1,
                note: note as u8,
                vel: 0,
                channel: c_cfg.channel,
            });
        }
    }
    note_data
}

impl crate::ToNotes for Multispectra {
    fn to_notes(&self, c_cfg: &crate::ConvertCfg) -> crate::MidiFile {
        let mut note_data: crate::MidiFile = vec![vec![]; c_cfg.tracks];

        let threads = c_cfg.threads;
        let batch_size = 128 / threads;

        let data = &self.sound_data;
        // Very naive multithreading, works fine but it could be more robust.
        std::thread::scope(|s| {
            let mut thread_pool: Vec<std::thread::ScopedJoinHandle<crate::MidiFile>> = vec![];

            for i in 0..threads {
                thread_pool.push(s.spawn(move || {
                    let cfg = c_cfg.clone();
                    let start = i * batch_size;
                    let end = if i < threads - 1 {
                        (i + 1) * batch_size
                    } else {
                        128
                    };
                    return notes_range(data, start..end, &cfg, &self.cfg);
                    // i tried (i..128).step_by(threads), but i * batch_size..(i + 1) * batch_size is faster?
                }));
            }

            for thread in thread_pool {
                let mut range_data = thread.join().unwrap();

                note_data
                    .iter_mut()
                    .zip(&mut range_data.iter_mut())
                    .map(|x| x.0.append(x.1))
                    .collect()
            }
        });

        note_data
    }
}