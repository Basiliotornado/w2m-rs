use crate::modes;

static SWITCHES: &[&str] = &["--no-odd"];

pub struct Arguments {
    pub mode: modes::Modes,
    pub path: String,
    pub output: String,
    pub ppqn: usize,
    pub bpm: f64,
    pub tracks: usize,
    pub threads: usize,
    pub multispectra: MultispectraCfg,
}

#[derive(Default, Clone)]
pub struct MultispectraCfg {
    pub overlap: f64,
    pub m: usize,
    pub b: usize,
    pub mult: usize,
    pub mult_diff: usize,
    pub no_odd: bool,
    pub odd_amp: f64,
}

pub fn print_usage() {
    println!(
        "Usage: 
    convert PATH [ARGS...]
    convert MODE PATH [ARGS...]
    convert MODE PATH OUTPUT [ARGS...]

Arguments:
    --ppqn PPQN - Sets the ppqn of the midi file (Default: 30000)
    --bpm BPM - Sets the bpm of the midi file (Default: 120.0)
    --tracks TRACKS - Sets how many tracks the notes are split into (Default: 16)
    --threads THREADS - Maximum threads used for conversion (Default: cpu threads / 2)
    
Modes:
    fft:
        Creates 1 spectrogram for every note, with bin size based on hz.
        options:
            -o OVERLAP - Percentage overlap between each bin (Default 0.5)
            -m M - m for the linear equation used to calculate the minimum bin size (min = m * (128-x) + b) (Default: 32)
            -b B - b for the equation above (Default: 128)
            --mult - Default bin size multiplier (Default: 36)
            --mult_diff - Number the mult increases by until it's above the minimum (Default: 12)
            --no-odd - Disable the odd symmetric windowed spectrogram
            --odd-amp AMP - Amplitude of the odd spectrogram (Default 0.166)"
    )
}

pub fn get_mode(arg: &str) -> Result<modes::Modes, String> {
    match arg {
        "fft" => Ok(modes::Modes::Multispectra),
        _ => {
            print_usage();
            Err(format!("Invalid mode \"{}\"", arg))
        }
    }
}

pub fn get_num_or<T>(map: &std::collections::HashMap<String, String>, arg: &str, or: T) -> T
where
    T: std::str::FromStr,
{
    match map.get(arg) {
        Some(value) => match value.parse() {
            Ok(value) => value,
            Err(_) => or,
        },
        None => or,
    }
}

pub fn parse_args() -> Result<Arguments, String> {
    let mut args = std::env::args();

    let program_path = args.next();
    println!("{}", args.len());

    // let mode = modes::Modes::Multispectra;

    let mut positional_args = vec![];
    let mut switches = vec![];
    let mut key_args = std::collections::HashMap::<String, String>::new();

    let mut key = String::new();
    let mut positional = true;
    for (i, arg) in args.enumerate() {
        println!("{i}, {arg}");
        if arg.len() > 1 && arg.as_bytes()[0] == b'-' {
            if SWITCHES.contains(&arg.as_str()) {
                switches.push(arg);
                continue;
            }
            positional = false;
            key = arg;
            continue;
        }

        if positional {
            positional_args.push(arg);
        } else {
            key_args.insert(key, arg);
            key = String::new(); // otherwise "possibly uninitialized"
            positional = true;
        }
    }

    println!("{positional_args:?}, {key_args:?}");
    let pos_len = positional_args.len();

    let mode;
    let path;
    let mut output = "output.mid".to_string();
    if pos_len == 1 {
        mode = modes::Modes::Multispectra;
        path = positional_args[0].clone();
        println!("Using default mode: fft");
    } else if pos_len == 2 || pos_len == 3 {
        mode = get_mode(positional_args[0].as_str()).unwrap();
        path = positional_args[1].clone();
        if pos_len == 3 {
            output = positional_args[2].clone();
        }
    } else {
        print_usage();
        return Err("Incorrect arguments".to_string());
    }

    let overlap = get_num_or(&key_args, "-o", 0.5);
    let m = get_num_or(&key_args, "-m", 32);
    let b = get_num_or(&key_args, "-b", 128);
    let mult = get_num_or(&key_args, "--mult", 36);
    let mult_diff = get_num_or(&key_args, "--mult_diff", 12);
    let no_odd = switches.contains(&"--no-odd".to_string());
    let odd_amp = get_num_or(&key_args, "--odd-amp", 0.166);

    let multispectra = MultispectraCfg {
        overlap,
        m,
        b,
        mult,
        mult_diff,
        no_odd,
        odd_amp,
    };

    let ppqn = get_num_or(&key_args, "--ppqn", 30000);
    let bpm = get_num_or(&key_args, "--bpm", 120.0);
    let tracks = get_num_or(&key_args, "--tracks", 16);
    let threads = get_num_or(
        &key_args,
        "--threads",
        match std::thread::available_parallelism() {
            Ok(v) => v.get() / 2,
            Err(_) => {
                println!("Couldn't count threads, using 4 unless specified");
                4 // 4 why not
            }
        },
    );

    Ok(Arguments {
        mode,
        path,
        output,
        ppqn,
        bpm,
        tracks,
        threads,
        multispectra,
    })
}