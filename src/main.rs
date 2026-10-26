use std::io::{Read, Write};

mod arguments;
mod mid;
mod modes;
mod raw;
mod reader;
mod wav;
mod windows;

type MidiFile = Vec<Vec<Note>>;

// I had higher hopes for my abilities
// this is probably overkill
trait ToNotes {
    fn to_notes(&self, config: &ConvertCfg) -> Vec<Vec<Note>>;
}

#[derive(Clone)]
struct ConvertCfg {
    tracks: usize,
    samplerate: usize,
    channel: u8,
    ppqn: u16,
    bpm: f32,
    threads: usize,
}

#[derive(Debug, Clone)]
struct Note {
    channel: u8,
    time: usize,
    note: u8,
    vel: u8,
}

struct SoundData {
    samplerate: usize,
    channels: usize,
    data: Vec<f32>,
}

impl SoundData {
    fn from_wav(wav: wav::Wav) -> Self {
        Self {
            samplerate: wav.samplerate as usize,
            channels: wav.channels as usize,
            data: wav.data,
        }
    }
    fn from_raw(raw: raw::Raw, samplerate: usize, channels: usize) -> Self {
        Self {
            samplerate,
            channels,
            data: raw.data,
        }
    }
}

// const TRACKS: usize = 16;
// const PPQN: u16 = 30000; //8192;

fn progress_bar() {
    print!("#");
    let _ = std::io::stdout().flush();
}

fn progress_header(text: &str) {
    println!("\n=== {text}");
    (0..128).for_each(|_| print!("_"));
    println!(" v");
}

fn get_notes(sound_data: Vec<f32>, args: &arguments::Arguments, cfg: &ConvertCfg) -> MidiFile {
    match args.mode {
        modes::Modes::Multispectra => {
            modes::fft::Multispectra::new(sound_data, args.multispectra.clone()).to_notes(&cfg)
        }
        modes::Modes::ZeroCross => {
            modes::zero_cross::ZeroCross::new(sound_data, args.zero_cross.clone()).to_notes(cfg)
        }
    }
}

fn main() {
    let args = match arguments::parse_args() {
        Ok(args) => args,
        Err(msg) => {
            println!("\n({msg})");
            return;
        }
    };

    println!("\nSettings\n====\n{args:?}\n");

    let track_count = args.tracks;
    let ppqn = args.ppqn as u16;

    let sound_data = if args.path == "-" {
        println!("Using stdin, assuming 2 channel s16le 48khz.");
        let mut stdin = Vec::new();
        std::io::stdin().lock().read_to_end(&mut stdin).unwrap();
        SoundData::from_raw(raw::Raw::new_i16(stdin), 48000, 1)
    } else {
        SoundData::from_wav(wav::Wav::load(std::path::Path::new(&args.path)).unwrap())
    };

    let header = midly::Header::new(
        midly::Format::Parallel,
        midly::Timing::Metrical(ppqn.into()),
    );
    let mut midi = midly::Smf::new(header);
    midi.tracks = vec![vec![]; track_count];

    let mut note_data: Vec<Vec<Note>> = vec![vec![]; track_count];

    let mut cfg = ConvertCfg {
        tracks: track_count,
        samplerate: sound_data.samplerate,
        channel: 0,
        ppqn: ppqn,
        bpm: args.bpm,
        threads: args.threads,
    };

    if sound_data.channels == 2 {
        let mut l = vec![0.0; sound_data.data.len() / 2];
        let mut r = vec![0.0; sound_data.data.len() / 2];

        for i in (0..sound_data.data.len() - 2).step_by(2) {
            l[i / 2] = sound_data.data[i];
            r[i / 2] = sound_data.data[i + 1];
        }

        note_data.append(&mut note_data.clone());
        midi.tracks.append(&mut vec![vec![]; track_count]);
        midi.tracks[0].push(mid::pan(0, 0));
        midi.tracks[track_count].push(mid::pan(1, 127));

        progress_header("Left channel");

        let mut notes_l = get_notes(l, &args, &cfg);
        println!(" done\n");

        progress_header("Right channel");

        cfg.channel = 1;
        let mut notes_r = get_notes(r, &args, &cfg);
        println!(" done\n");

        for i in 0..track_count {
            note_data[i].append(&mut notes_l[i]);
        }
        for i in 0..track_count {
            note_data[i + track_count].append(&mut notes_r[i]);
        }

        println!(
            "{} notes",
            note_data
                .iter()
                .map(|x| x.len() / 2)
                .reduce(|acc, x| acc + x)
                .unwrap()
        );
    } else if sound_data.channels == 1 {
        progress_header("Progress");
        note_data = get_notes(sound_data.data, &args, &cfg);
        println!(" done\n");
    }

    for track in note_data.iter_mut() {
        track.sort_unstable_by_key(|a| a.time);

        for i in (1..track.len()).rev() {
            track[i].time -= track[i - 1].time;
        }
    }

    for (i, track) in note_data.iter().enumerate() {
        for note in track {
            if note.vel > 0 {
                midi.tracks[i].push(mid::note_on(
                    note.channel,
                    note.note,
                    note.vel,
                    note.time as u32,
                ));
            } else {
                midi.tracks[i].push(mid::note_off(
                    note.channel,
                    note.note,
                    note.vel,
                    note.time as u32,
                ));
            }
        }
    }

    println!(
        "\nEvent count: {:?}",
        midi.tracks.iter().map(|x| x.len()).collect::<Vec<usize>>()
    );

    for track in midi.tracks.iter_mut() {
        track.push(midly::TrackEvent {
            delta: 0.into(),
            kind: midly::TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
        });
    }

    midi.save(args.output).unwrap()
}
