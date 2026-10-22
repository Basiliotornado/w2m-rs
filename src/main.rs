use std::io::Read;

mod arguments;
mod mid;
mod modes;
mod raw;
mod reader;
mod wav;
mod windows;

trait ToNotes {
    fn to_notes(&self, config: &ConvertCfg) -> Vec<Vec<Note>>;
}

struct ConvertCfg {
    tracks: usize,
    samplerate: usize,
    channel: u8,
    ppqn: u16,
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
    data: Vec<f64>,
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

const TRACKS: usize = 16;
const PPQN: u16 = 30000; //8192;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: convert [input.mp3]");

    let arguments = arguments::parse_args().unwrap();

    let sound_data = if path == "-" {
        println!("Using stdin, assuming 2 channel s16le 48khz.");
        let mut stdin = Vec::new();
        std::io::stdin().lock().read_to_end(&mut stdin).unwrap();
        println!("{:?}", &stdin[400000 * 2..400000 * 2 + 100]);
        SoundData::from_raw(raw::Raw::new_i16(stdin), 48000, 2)
    } else {
        SoundData::from_wav(wav::Wav::load(std::path::Path::new(&path)).unwrap())
    };

    let header = midly::Header::new(
        midly::Format::Parallel,
        midly::Timing::Metrical(PPQN.into()),
    );
    let mut midi = midly::Smf::new(header);
    midi.tracks = vec![vec![]; TRACKS];

    let mut note_data: Vec<Vec<Note>> = vec![vec![]; TRACKS];

    println!("{}, {}", sound_data.data.len(), sound_data.samplerate);

    if sound_data.channels == 2 {
        let mut l = vec![0.0; sound_data.data.len() / 2];
        let mut r = vec![0.0; sound_data.data.len() / 2];

        for i in (0..sound_data.data.len() - 2).step_by(2) {
            l[i / 2] = sound_data.data[i];
            r[i / 2] = sound_data.data[i + 1];
        }

        note_data.append(&mut note_data.clone());
        midi.tracks.append(&mut vec![
            vec![
                mid::note_on(1, 0, 1, 0),
                mid::note_off(1, 0, 0, 1)
            ];
            TRACKS
        ]);
        midi.tracks[0].push(mid::pan(0, 0));
        midi.tracks[TRACKS].push(mid::pan(1, 127));

        let spec_l = modes::fft::Multispectra::new(l);
        let spec_r = modes::fft::Multispectra::new(r);

        let mut cfg = ConvertCfg {
            tracks: TRACKS,
            samplerate: sound_data.samplerate,
            channel: 0,
            ppqn: PPQN,
        };

        let mut notes_l = spec_l.to_notes(&cfg);
        cfg.channel = 1;
        let mut notes_r = spec_r.to_notes(&cfg);

        for i in 0..TRACKS {
            note_data[i].append(&mut notes_l[i]);
        }
        for i in 0..TRACKS {
            note_data[i + TRACKS].append(&mut notes_r[i]);
        }

        println!("{} notes", note_data.len());

        for track in note_data.iter_mut() {
            track.sort_unstable_by_key(|a| a.time);

            for i in (1..track.len()).rev() {
                track[i].time -= track[i - 1].time;
            }
        }
        for track in 0..TRACKS * 2 {
            for note in &note_data[track] {
                if note.vel > 0 {
                    midi.tracks[track].push(mid::note_on(
                        note.channel,
                        note.note,
                        note.vel,
                        note.time as u32,
                    ));
                } else {
                    midi.tracks[track].push(mid::note_off(
                        note.channel,
                        note.note,
                        note.vel,
                        note.time as u32,
                    ));
                }
            }
        }
        midi.tracks.insert(0, vec![]);
    }

    println!(
        "{:?}",
        midi.tracks.iter().map(|x| x.len()).collect::<Vec<usize>>()
    );

    for track in midi.tracks.iter_mut() {
        track.push(midly::TrackEvent {
            delta: 0.into(),
            kind: midly::TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
        });
    }

    midi.save("midi2.mid").unwrap()
}
