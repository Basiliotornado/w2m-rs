use crate::modes;

static SWITCHES: &[&str] = &["--no-odd"];

pub struct Arguments {
    mode: modes::Modes,
    path: String,
    ppqn: usize,
    bpm: usize,
    multispectra: MultispectraCfg,
}

#[derive(Default)]
pub struct MultispectraCfg {
    overlap: f64,
    m: usize,
    b: usize,
    do_odd: bool,
}

pub fn print_usage() {
    println!(
        "Usage: 
    convert PATH [ARGS...]
    convert [MODE] PATH [ARGS...]
    
Modes:
    fft:
        Creates 1 spectrogram for every note, with bin size based on hz."
    )
}

pub fn parse_args() -> Result<Arguments, String> {
    let mut args = std::env::args();

    let program_path = args.next();
    println!("{}", args.len());

    // let mode = modes::Modes::Multispectra;

    let ppqn = 30000;
    let bpm = 120;
    let multispectra = MultispectraCfg::default();

    let mut positional_args = vec![];
    let mut switches = vec![];
    let mut key_args = std::collections::HashMap::<String, String>::new();
    let mut key = String::new();
    let mut positional = true;
    for (i, arg) in args.enumerate() {
        println!("{i}, {arg}");
        if arg.len() > 1 && arg.bytes().nth(0).unwrap() == b'-' {
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

    let mode;
    let path;
    if positional_args.len() == 1 {
        mode = modes::Modes::Multispectra;
        path = positional_args[0].clone();
        println!("Using default mode: fft");
    } else if positional_args.len() == 2 {
        mode = match positional_args[0].as_str() {
            "fft" => modes::Modes::Multispectra,
            _ => {
                print_usage();
                return Err(format!("No mode \"{}\"", positional_args[0]));
            }
        };
        path = positional_args[1].clone();
    } else {
        print_usage();
        return Err("Incorrect arguments".to_string());
    }

    println!("{positional_args:?}, {key_args:?}");

    Ok(Arguments {
        mode,
        path,
        ppqn,
        bpm,
        multispectra,
    })
}