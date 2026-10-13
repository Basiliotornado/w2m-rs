use std::f64::consts::PI;
use std::fs::File;
use std::vec;
use symphonia::core::audio::{SampleBuffer, Channels};
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::Error;
use symphonia::core::probe::Hint;
use symphonia::default;
use symphonia::core::io::MediaSourceStream;

use rustfft::{FftPlanner, num_complex::Complex};

use apres::{MIDI, MIDIEvent};

#[derive()]
struct AudioData {
    pub data: Vec<f64>,
    pub stereo: bool,
    pub sample_rate: usize,
}

struct Spectrogram {
    pub data: Vec<Vec<f64>>,
    pub interval: Vec<f64>,
    pub frequencies: Vec<f64>,
}

fn load_mp3(file: File) -> AudioData {
    let codecs = default::get_codecs();
    let probe = default::get_probe();


    let source_stream = MediaSourceStream::new(Box::new(file),Default::default());

    let probe_result = probe.format(&Hint::new(), source_stream, &Default::default(), &Default::default()).expect("probe error");
    let mut format = probe_result.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("error finding track");

    let mut decoder = codecs
        .make(&track.codec_params, &Default::default())
        .expect("something something decoder error");

    let track_id = track.id;

    let mut sample_buffer = None;
    let mut sample_count = 0;

    let n_frames = track.codec_params.n_frames.unwrap();
    let channels = track.codec_params.channels.unwrap();
    let sample_rate: usize = track.codec_params.sample_rate.unwrap().try_into().unwrap();

    let mut audio_data: Vec<f64>;
    let stereo:bool;
    if channels.contains(Channels::FRONT_RIGHT) {
        audio_data = vec![0.0 as f64; (n_frames*2) as usize];
        stereo = true
    } else {
        audio_data = vec![0.0 as f64; n_frames as usize];
        stereo = false
    }

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break
        };
        if packet.track_id() != track_id {
            continue
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                if sample_buffer.is_none() {
                    // Get the audio buffer specification.
                    let spec = *decoded.spec();

                    // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                    let duration = decoded.capacity() as u64;

                    // Create the f64 sample buffer.
                    sample_buffer = Some(SampleBuffer::<f64>::new(duration, spec));
                }
                if let Some(buf) = &mut sample_buffer {
                    buf.copy_planar_ref(decoded);

                    // The samples may now be access via the `samples()` function.
                    let samples = buf.samples();

                    audio_data.splice(sample_count..sample_count+samples.len(), samples.iter().cloned());

                    sample_count += samples.len();
                }
            }
            Err(Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                continue;
            }
            Err(err) => {
                // An unrecoverable error occured, halt decoding.
                panic!("{}", err);
            }
        }
    }
    AudioData{data:audio_data,stereo,sample_rate}
}

fn f64_vec_to_complex(vec:Vec<f64>) -> Vec<Complex<f64>> {
    vec.iter().map(|&e| Complex{ re: e , im: 0.0 }).collect()
}

fn blackman_harris_window(width: usize) -> Vec<f64> {
    let mut window_data = vec![0.0; width];

    for i in 0..width {
        let factor = (PI * i as f64) / width as f64;
        window_data[i] = 0.35875 - 0.4891775 * (2.0 * factor).cos() + 0.1365995 * (4.0 * factor).cos() - 0.0106411 * (6.0 * factor).cos();
    }

    window_data
}

fn spectrogram_stereo(sound:Vec<Complex<f64>>, bin_size:usize, bin_overlap:usize, sample_rate:usize, window: &Vec<f64>) -> (Spectrogram, Spectrogram) {
    let mut l = vec![Complex{ re: 0.0, im: 0.0 }; sound.len()/2];
    let mut r = vec![Complex{ re: 0.0, im: 0.0 }; sound.len()/2];

    for i in (0..sound.len()-2).step_by(2) {
        l[i/2] = sound[i];
        r[i/2] = sound[i+1];
    }

    let l_spec = spectrogram(l, bin_size, bin_overlap, sample_rate, &window);
    let r_spec = spectrogram(r, bin_size, bin_overlap, sample_rate, &window);

    (l_spec, r_spec)
}

fn spectrogram(sound:Vec<Complex<f64>>, bin_size:usize, bin_overlap:usize, sample_rate:usize, window: &Vec<f64>) -> Spectrogram {
    let step_size = bin_size - bin_overlap;
    let total_steps = (sound.len()-bin_size)/step_size + 1;

    let mut fft_planner = FftPlanner::new();
    let fft_plan =  fft_planner.plan_fft_forward(bin_size);

    let mut spectro: Vec<Vec<f64>> = vec![vec![]; total_steps];
    let mut time: Vec<f64> = vec![0.0; total_steps];
    let mut freq: Vec<f64> = vec![0.0; bin_size];

    for i in 0..total_steps {
        let sample = step_size*i;
        let mut buffer = sound[sample..(bin_size+sample)].to_vec();

        for j in 0..bin_size {
            buffer[j] *= window[j]
        }
        
        fft_plan.process(&mut buffer);
        
        spectro[i] = buffer.clone().iter().map(|num| (num.re.powi(2)+num.im.powi(2)).sqrt() / bin_size as f64).collect();

        time[i] = sample as f64 / sample_rate as f64;
    }

    for i in 0..bin_size {
        freq[i] = sample_rate as f64 * i as f64 / (bin_size as f64 - 1.0) / 2.0;
    }
    Spectrogram{ data:spectro , interval:time, frequencies:freq }
    
}
fn frequency_to_piano_key(freq:f64) -> usize {
    (12.0 * (freq / 440.0).log2() + 49.0).round() as usize
}
fn spectrogram_to_piano_notes(spectrogram: Spectrogram) -> Spectrogram {
    let mut spectro: Vec<Vec<f64>> = vec![vec![]; spectrogram.data.len()];
    for row in 0..spectrogram.data.len() {
        spectro[row] = vec![0.0; 128];
        for i in 0..spectrogram.data[row].len() {
            let key = frequency_to_piano_key(spectrogram.frequencies[i]);
            if key > 127 {continue}
            //println!("{}",(spectrogram.data[row][i]/spectrogram.frequencies[i]).powf(0.25));
            spectro[row][key] += (spectrogram.data[row][i]/spectrogram.frequencies[i]).powf(0.25)*1000.0;
        }
    }
    let mut freq: Vec<f64> = vec![0.0; 128];
    for i in 0..128 {
        freq[i] = 2.0f64.powf((i as f64-49.0)/12.0) * 440.0
    }
    Spectrogram{ data:spectro , interval:spectrogram.interval , frequencies:freq }
}

fn main() {
    let path = std::env::args().nth(1).expect("Usage: convert [input.mp3]");
    let sound_data = load_mp3(File::open(path).expect("error in reading mp3"));

    let window = blackman_harris_window(44100);
    let complex_data = f64_vec_to_complex(sound_data.data.clone());

    let mut midi = MIDI::new();

    if sound_data.stereo {
        let (spec_l,_spec_r) = spectrogram_stereo(complex_data, 2048, 1024, sound_data.sample_rate, &window);
        for i in spec_l.frequencies.iter() {
            println!("{}",frequency_to_piano_key(*i))
        }

        let piano_spec = spectrogram_to_piano_notes(spec_l);
        let mut progress = 0;
        for column in piano_spec.data.iter() {
            let mut placed: [[u8;3];128]= [[0;3];128];
            for note in 0..column.len() {
                midi.push_event(1, 0, MIDIEvent::NoteOn(1, note as u8, column[note] as u8));
                placed[note] = [1, note as u8, column[note] as u8];
            }
            midi.push_event(1, 10, MIDIEvent::NoteOff(1, 1, 0));
            for note in 0..128 {
                midi.push_event(1, 0, MIDIEvent::NoteOff(placed[note][0],placed[note][1],placed[note][2]));
            }
            println!("{progress},{}",piano_spec.data.len());
            progress += 1;
            if progress > 400 {break};
        }
        
    } else {
        let spec = spectrogram(complex_data, 2048, 512+1024, sound_data.sample_rate, &window);
        let piano_spec = spectrogram_to_piano_notes(spec);
        for i in piano_spec.data.iter() {
            let mut placed: Vec<[u8;3]> = vec![];
            for j in 0..i.len() {
                midi.push_event(1, 0, MIDIEvent::NoteOn(1, j as u8, i[j] as u8 + 1));
                placed.push([1, j as u8, i[j] as u8 + 1]);
            }
            midi.push_event(1, 10, MIDIEvent::NoteOff(1, 1, 0));
            for j in placed.iter() {
                midi.push_event(1, 0, MIDIEvent::NoteOff(j[0],j[1],j[2]));
            }
        }
    }
    midi.push_event(1, 0, MIDIEvent::EndOfTrack);
    midi.save("midi2.mid")
}
