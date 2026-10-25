// fn frequency_to_piano_key(freq: f64) -> usize {
//     (12.0 * (freq / 440.0).log2() + 69.0).round() as usize
// }
pub fn piano_key_to_frequency(note: u8) -> f32 {
    2.0f32.powf((note as f32 - 69.0) / 12.0) * 440.0
}

pub fn time_to_tick(time: f32, ppqn: f32, bpm: f32) -> f32 {
    time * ppqn * bpm / 60.0
}
pub fn note_on<'a>(channel: u8, key: u8, vel: u8, delta: u32) -> midly::TrackEvent<'a> {
    midly::TrackEvent {
        delta: delta.into(),
        kind: midly::TrackEventKind::Midi {
            channel: channel.into(),
            message: midly::MidiMessage::NoteOn {
                key: key.into(),
                vel: vel.into(),
            },
        },
    }
}
pub fn note_off<'a>(channel: u8, key: u8, vel: u8, delta: u32) -> midly::TrackEvent<'a> {
    midly::TrackEvent {
        delta: delta.into(),
        kind: midly::TrackEventKind::Midi {
            channel: channel.into(),
            message: midly::MidiMessage::NoteOff {
                key: key.into(),
                vel: vel.into(),
            },
        },
    }
}
pub fn pan<'a>(channel: u8, value: u8) -> midly::TrackEvent<'a> {
    midly::TrackEvent {
        delta: 0.into(),
        kind: midly::TrackEventKind::Midi {
            channel: channel.into(),
            message: midly::MidiMessage::Controller {
                controller: 10.into(), //pan
                value: value.into(),   //left
            },
        },
    }
}
