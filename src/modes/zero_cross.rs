use std::f32::consts::PI;

fn sinc(size: usize, f: f32, sr: f32) -> Vec<f32> {
    // let output = vec![0.0; size];

    (0..size)
        .map(|x| (x as f32 - (size as f32 / 2.0).floor()) * 2.0 / sr * f * PI)
        .map(|x| match x {
            0.0 => 1.0,
            _ => x.sin() / x,
        })
        .collect::<Vec<f32>>()
    // output
}

fn convolve(a: &[f32], b: &[f32]) -> Vec<f32> {
    let al = a.len();
    let bl = b.len();
    let c_len = al.min(bl);
    let half = c_len - c_len / 2;

    let odd = c_len % 2;
    let even = odd ^ 1;

    let mut output = vec![0.0; c_len];
    let mut v;
    for i in 0..c_len {
        v = 0.0;

        for j in if i < half {
            0..half + i
        } else {
            i - half + odd..c_len
        } {
            v += a[j] * b[bl - (half + j - i) - even];
        }
        output[i] = v
    }

    output
}

fn bandpass(taps: usize, f: f32, width: f32, sr: f32) -> Vec<f32> {
    let max_res = sr / taps as f32 * 2.0;
    let width_d = f * width;

    let f1;
    let f2;
    if width_d > max_res {
        f1 = f - width_d;
        f2 = f + width_d;
    } else {
        f1 = f - max_res;
        f2 = f + max_res;
    };

    let mut a = sinc(taps, sr / 2.0 - f1, sr);
    let b = sinc(taps, f2, sr);

    let a_sum = a.iter().copied().reduce(|acc, x| acc + x).unwrap();
    let b_sum = b.iter().copied().reduce(|acc, x| acc + x).unwrap();

    let fact = a_sum * b_sum;

    for i in (1..taps).step_by(2) {
        a[i] *= -1.0;
    }

    let mut c = convolve(&a, &b);

    let window = crate::windows::hann(taps);

    for x in c.iter_mut().zip(window) {
        *x.0 *= x.1 / fact;
    }

    c
}

fn filt(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let s_len = signal.len();
    let f_len = filter.len();
    let mut output = vec![0.0; signal.len()];

    let mut val;
    for i in 0..s_len {
        val = 0.0;
        for j in 0..f_len {
            if i + j - f_len / 2 >= s_len - 1 {
                continue;
            }
            val += signal[i + j - f_len / 2] * filter[f_len - j - 1];
        }
        output[i] = val;
    }

    output
}

fn filtfilt(data: &[f32], filter: &[f32]) -> Vec<f32> {
    let mut output = filt(data, filter);
    output.reverse();
    output = filt(&output, filter);
    output.reverse();

    output
}

fn filtfilt_fft(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let s_len = signal.len();
    let f_len = filter.len();
    // let mut output = vec![0.0; signal.len()];

    let mut planner = rustfft::FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(s_len);
    let fft_plan_i = planner.plan_fft_inverse(s_len);

    let mut sig = crate::modes::fft::f64_vec_to_complex(signal);
    let mut fil = crate::modes::fft::f64_vec_to_complex(filter);

    fil.append(&mut vec![
        rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
        s_len - f_len
    ]);

    fft_plan.process(&mut sig);
    fft_plan.process(&mut fil);

    let _ = sig
        .iter_mut()
        .zip(&mut fil.iter_mut())
        .map(|x| *x.0 *= *x.1)
        .collect::<Vec<_>>();

    fft_plan_i.process(&mut sig);

    let _ = sig.iter_mut().map(|x| x.im = 0.0).collect::<Vec<_>>();
    sig.reverse();

    fft_plan.process(&mut sig);
    let _ = sig
        .iter_mut()
        .zip(&mut fil.iter_mut())
        .map(|x| *x.0 *= *x.1)
        .collect::<Vec<_>>();
    fft_plan_i.process(&mut sig);
    sig.reverse();

    let fac = s_len.pow(2) as f32;
    sig.iter().map(|x| x.re / fac).collect()
}

#[derive(Clone)]
struct Cross {
    index: usize,
    vel: u8,
    track: usize,
    note: u8,
}

fn notes_range<T>(data: &[f32], range: T, c_cfg: &crate::ConvertCfg) -> Vec<Cross>
where
    T: Iterator<Item = usize>,
{
    let mut crosses = vec![];
    for note in range {
        println!("Doing note {note}");
        let frequency = crate::mid::piano_key_to_frequency(note as u8);

        let skip = note.pow(2) / 1000;
        // let skip = 1;

        let band_filt = bandpass(4096 * 4, frequency, 0.05, c_cfg.samplerate as f32);
        let filtered = filtfilt_fft(&data, &band_filt);
        let mut sign = true;
        let mut count = 0;
        let mut amp = 0.0;
        let mut abs;
        for (i, sample) in filtered.iter().enumerate() {
            if sample.is_sign_negative() != sign {
                sign = sample.is_sign_negative();
                count += 1;
                if sign && ((count / 2) >= skip) {
                    let vel = ((amp * 127.99 * 1.5) as u8).clamp(0, 127);
                    let track =
                        (amp * c_cfg.tracks as f32).clamp(0.0, c_cfg.tracks as f32 - 1.0) as usize;
                    amp = 0.0;
                    crosses.push(Cross {
                        index: i,
                        vel,
                        track,
                        note: note as u8,
                    });
                    count = 0;
                }
            }
            abs = sample.abs();
            if abs > amp {
                amp = abs;
            }
        }
    }
    crosses
}

pub struct ZeroCross {
    pub sound_data: Vec<f32>,
    cfg: crate::arguments::ZeroCrossCfg,
}

impl ZeroCross {
    pub fn new(sound_data: Vec<f32>, cfg: crate::arguments::ZeroCrossCfg) -> Self {
        Self { sound_data, cfg }
    }
}

impl crate::ToNotes for ZeroCross {
    fn to_notes(&self, c_cfg: &crate::ConvertCfg) -> crate::MidiFile {
        let mut note_data = vec![vec![]; c_cfg.tracks];

        let sr = c_cfg.samplerate as f32;

        let mut crosses = vec![];

        let threads = 8;
        let batch_size = 128 / threads;

        let data = &self.sound_data;
        let cfg = &c_cfg;
        std::thread::scope(|s| {
            let mut thread_pool: Vec<std::thread::ScopedJoinHandle<Vec<Cross>>> = vec![];

            for i in 0..threads {
                thread_pool.push(s.spawn(move || {
                    let start = i * batch_size;
                    let end = if i < threads - 1 {
                        (i + 1) * batch_size
                    } else {
                        128
                    };
                    return notes_range(data, start..end, cfg);
                    // i tried (i..128).step_by(threads), but i * batch_size..(i + 1) * batch_size is faster?
                }));
            }
            for thread in thread_pool {
                let mut range_data = thread.join().unwrap();
                crosses.append(&mut range_data);
            }
        });

        for i in 0..crosses.len() - 1 {
            let cross = &crosses[i];
            if cross.vel < 1 {
                continue;
            }
            note_data[cross.track].push(crate::Note {
                channel: c_cfg.channel,
                time: crate::mid::time_to_tick(
                    cross.index as f32 / sr,
                    c_cfg.ppqn as f32,
                    c_cfg.bpm as f32,
                ) as usize,
                note: cross.note,
                vel: cross.vel,
            });
            let cross2 = &crosses[i + 1];
            note_data[cross.track].push(crate::Note {
                channel: c_cfg.channel,
                time: crate::mid::time_to_tick(
                    cross2.index as f32 / sr,
                    c_cfg.ppqn as f32,
                    c_cfg.bpm as f32,
                ) as usize
                    - 1,
                note: cross.note,
                vel: 0,
            });
        }
        note_data
    }
}