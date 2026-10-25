use crate::reader::Reader;
use std::io::BufReader;

pub struct Wav {
    pub samplerate: u32,
    pub channels: u16,
    pub data: Vec<f32>,
}

impl Wav {
    pub fn load(path: &std::path::Path) -> Result<Self, &str> {
        let file = std::fs::File::open(path).unwrap();
        let mut reader = Reader::new(BufReader::new(file));

        if !reader.check_header("RIFF") {
            return Err("Not a WAV file!");
        }

        let file_size = reader.read_type::<u32, 4>();

        if !reader.check_header("WAVE") {
            return Err("No WAVE header!");
        }
        if !reader.check_header("fmt ") {
            return Err("No fmt Header!");
        }

        let bloc_size = reader.read_type::<u32, 4>();
        let audio_format = reader.read_type::<u16, 2>(); // 1: int, 3: float
        let channels = reader.read_type::<u16, 2>();
        let samplerate = reader.read_type::<u32, 4>();
        let byte_per_sec = reader.read_type::<u32, 4>();
        let byte_per_block = reader.read_type::<u16, 2>();
        let bits_per_sample = reader.read_type::<u16, 2>();

        println!("Size: {file_size}, BlocSize: {bloc_size}, AudioFormat: {audio_format}, Channels: {channels}, Samplerate: {samplerate}\nBytePerSec: {byte_per_sec} BytePerBlock: {byte_per_block}, BitsPerSample: {bits_per_sample}");

        let mut buf = [0u8; 4];
        let mut index = 0;
        // Scan until data header hopefully
        loop {
            buf[index] = reader.read(1)[0];
            if buf[(index - 3) % 4] == b'd'
                && buf[(index - 2) % 4] == b'a'
                && buf[(index - 1) % 4] == b't'
                && buf[index] == b'a'
            {
                break;
            }
            index += 1;
            index %= 4;
        }

        let block_size = reader.read_type::<u32, 4>();
        let samples = block_size / (bits_per_sample as u32 / 8);

        if block_size == u32::MAX {
            return Err("Invalid block size");
        }
        let mut data = vec![0f32; samples as usize];

        println!("BlockSize: {block_size}, Samples: {samples}");

        if audio_format == 1 {
            for i in 0..samples as usize {
                data[i] = reader.read_type::<i16, 2>() as f32 / i16::MAX as f32;
            }
        } else if audio_format == 3 {
            for i in 0..samples as usize {
                data[i] = reader.read_type::<f32, 4>() as f32;
            }
        }
        // return Err("Hey");

        Ok(Self {
            samplerate,
            channels,
            data,
        })
    }
}