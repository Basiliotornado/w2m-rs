use std::f32::consts::PI;

use crate::modes;

fn sinc(size: usize, f: f32, sr: f32) -> Vec<f32> {
    (0..size)
        .map(|x| (x as f32 - (size as f32 / 2.0).floor()) * 2.0 / sr * f * PI)
        .map(|x| match x {
            0.0 => 1.0,
            _ => x.sin() / x,
        })
        .collect::<Vec<f32>>()
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

    // println!("{:?}", convolve(&a, &b));
    // println!("{:?}", filt_fft(&a, &b));

    let mut c = filt_fft(&a, &b);

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

fn filt_ola(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let filt_len = filter.len();
    let buffer_len = (filt_len + 1).next_power_of_two();
    let ola_len = buffer_len - filt_len;

    let mut output = Vec::with_capacity(signal.len());
    let fac = buffer_len as f32;

    let mut planner = rustfft::FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(buffer_len);
    let fft_plan_i = planner.plan_fft_inverse(buffer_len);

    let mut filt = crate::modes::fft::f32_vec_to_complex(filter);
    filt.append(&mut vec![
        rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
        buffer_len - filt_len
    ]);
    fft_plan.process(&mut filt);

    let mut sig_buf;
    for i in 0..(signal.len() - 1) / filt_len + 1 {
        if (i + 1) * filt_len < signal.len() {
            sig_buf =
                crate::modes::fft::f32_vec_to_complex(&signal[i * filt_len..(i + 1) * filt_len]);
        } else {
            sig_buf = crate::modes::fft::f32_vec_to_complex(&signal[(i) * filt_len..]);
        }
        sig_buf.append(&mut vec![
            rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
            buffer_len - sig_buf.len()
        ]);
        fft_plan.process(&mut sig_buf);

        sig_buf
            .iter_mut()
            .zip(&mut filt.iter_mut())
            .for_each(|x| *x.0 *= *x.1);

        fft_plan_i.process(&mut sig_buf);
        sig_buf.iter_mut().for_each(|x| x.im = 0.0);

        let output_len = output.len();
        if output_len >= ola_len {
            for i in 0..ola_len {
                output[output_len - ola_len + i] += sig_buf[i].re / fac;
            }
            output.append(
                &mut sig_buf[ola_len..]
                    .iter()
                    .map(|x| x.re / fac)
                    .collect::<Vec<f32>>(),
            );
        } else {
            output.append(&mut sig_buf.iter().map(|x| x.re / fac).collect::<Vec<f32>>())
        }
    }

    output[ola_len / 2..signal.len() + ola_len / 2].to_vec()
}

fn filt_fft(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let s_len = signal.len() * 2;
    let f_len = filter.len() * 2;

    let mut planner = rustfft::FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(s_len);
    let fft_plan_i = planner.plan_fft_inverse(s_len);

    let mut sig = crate::modes::fft::f32_vec_to_complex(signal);
    let mut fil = crate::modes::fft::f32_vec_to_complex(filter);

    fil.append(&mut vec![
        rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
        s_len - sig.len()
    ]);
    sig.append(&mut vec![
        rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
        f_len - filter.len()
    ]);

    fft_plan.process(&mut sig);
    fft_plan.process(&mut fil);

    sig.iter_mut()
        .zip(&mut fil.iter_mut())
        .for_each(|x| *x.0 *= *x.1);

    fft_plan_i.process(&mut sig);

    let even = (signal.len() % 2) ^ 1;
    let fac = s_len as f32;
    sig[s_len / 4 - even..s_len / 2 + s_len / 4 - even]
        .iter()
        .map(|x| x.re / fac)
        .collect()
}

fn filtfilt(data: &[f32], filter: &[f32]) -> Vec<f32> {
    let mut output = filt(data, filter);
    output.reverse();
    output = filt(&output, filter);
    output.reverse();

    output
}

fn filtfilt_ola(data: &[f32], filter: &[f32]) -> Vec<f32> {
    let mut output = filt_ola(data, filter);
    output.reverse();
    output = filt_ola(&output, filter);
    output.reverse();

    output
}

fn filtfilt_fft(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let s_len = signal.len();
    let f_len = filter.len();

    let mut planner = rustfft::FftPlanner::new();
    let fft_plan = planner.plan_fft_forward(s_len);
    let fft_plan_i = planner.plan_fft_inverse(s_len);

    let mut sig = crate::modes::fft::f32_vec_to_complex(signal);
    let mut fil = crate::modes::fft::f32_vec_to_complex(filter);

    fil.append(&mut vec![
        rustfft::num_complex::Complex { re: 0.0, im: 0.0 };
        s_len - f_len
    ]);

    fft_plan.process(&mut sig);
    fft_plan.process(&mut fil);

    sig.iter_mut()
        .zip(&mut fil.iter_mut())
        .for_each(|x| *x.0 *= *x.1);

    fft_plan_i.process(&mut sig);

    sig.iter_mut().for_each(|x| x.im = 0.0);
    sig.reverse();

    fft_plan.process(&mut sig);
    sig.iter_mut()
        .zip(&mut fil.iter_mut())
        .for_each(|x| *x.0 *= *x.1);

    fft_plan_i.process(&mut sig);
    sig.reverse();

    let fac = s_len.pow(2) as f32;
    sig.iter().map(|x| x.re / fac).collect()
}

#[derive(Clone)]
struct Cross {
    index: usize,
    amp: f32,
    note: u8,
}

fn notes_range<T>(
    data: &[f32],
    range: T,
    c_cfg: &crate::ConvertCfg,
    z_cfg: &crate::arguments::ZeroCrossCfg,
) -> Vec<Cross>
where
    T: Iterator<Item = usize>,
{
    let mut crosses = vec![];

    for note in range {
        let frequency = crate::mid::piano_key_to_frequency(note as u8);

        let skip = (((note.pow(2) / 1000) as f32 * z_cfg.skip_m) as usize).clamp(1, usize::MAX);

        let band_filt = bandpass(
            z_cfg.taps,
            frequency,
            z_cfg.filter_width,
            c_cfg.samplerate as f32,
        );
        let filtered = filtfilt_ola(&data, &band_filt);

        let mut sign = true;
        let mut amp = 0.0;
        let mut count = 0;
        let mut avg = 0.0;
        let mut abs;
        for (i, sample) in filtered.iter().enumerate() {
            if sample.is_sign_negative() != sign {
                sign = sample.is_sign_negative();
                count += 1;
                avg += amp;
                amp = 0.0;
                if sign && ((count / 2) >= skip) {
                    crosses.push(Cross {
                        index: i,
                        amp: avg / count as f32,
                        note: note as u8,
                    });
                    avg = 0.0;
                    count = 0;
                }
            }
            abs = sample.abs();
            if abs > amp {
                amp = abs;
            }
        }
        crate::progress_bar();
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

        let threads = c_cfg.threads;
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
                    return notes_range(data, start..end, cfg, &self.cfg);
                    // i tried (i..128).step_by(threads), but i * batch_size..(i + 1) * batch_size is faster?
                }));
            }
            for thread in thread_pool {
                let mut range_data = thread.join().unwrap();
                crosses.append(&mut range_data);
            }
        });

        let max_amp = crosses
            .iter()
            .map(|x| x.amp)
            .reduce(|acc, x| if x > acc { x } else { acc })
            .unwrap();

        for i in 0..crosses.len() - 1 {
            let cross = &crosses[i];
            let amp = (cross.amp / max_amp).sqrt();
            let vel = ((amp * 127.99) as u8).clamp(0, 127);
            let track = ((amp * c_cfg.tracks as f32) as usize).clamp(0, c_cfg.tracks - 1);

            // println!("{vel}, {track}");
            if vel < 1 {
                continue;
            }
            note_data[track].push(crate::Note {
                channel: c_cfg.channel,
                time: crate::mid::time_to_tick(
                    cross.index as f32 / sr,
                    c_cfg.ppqn as f32,
                    c_cfg.bpm as f32,
                ) as usize,
                note: cross.note,
                vel: vel,
            });
            let cross2 = &crosses[i + 1];
            note_data[track].push(crate::Note {
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