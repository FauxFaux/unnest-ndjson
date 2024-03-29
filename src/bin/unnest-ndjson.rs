use std::env;
use std::io;
use std::process;
use std::str::FromStr;
use std::thread;

use unnest_ndjson::{unnest_to_ndjson, HeaderStyle};

fn main() -> io::Result<()> {
    process::exit(
        // evading stack problems, the lazy way
        thread::Builder::new()
            .stack_size(20 * 1024 * 1024)
            .spawn(run)?
            .join()
            .expect("worker panicked")?,
    )
}

fn run() -> io::Result<i32> {
    let mut args = env::args();
    let us = args.next().expect("bin name");
    let mut header_style = HeaderStyle::None;
    let mut target = None;
    let usage = || eprintln!("usage: {:?} [--path] TARGET_DEPTH", us);
    for arg in args {
        if arg.starts_with('-') {
            match arg.as_str() {
                "--path" => {
                    header_style = HeaderStyle::PathArray;
                    continue;
                }
                _ => {
                    eprintln!("unrecognised arg: {:?}", arg);
                    usage();
                    return Ok(3);
                }
            }
        }

        match usize::from_str(&arg) {
            Ok(v) => target = Some(v),
            Err(e) => {
                eprintln!("invalid target depth, try '1': {:?}: {}", arg, e);
                usage();
                return Ok(4);
            }
        }
    }

    let target = match target {
        Some(t) => t,
        None => {
            usage();
            return Ok(5);
        }
    };

    let stdin = io::stdin();
    let stdin = stdin.lock();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    unnest_to_ndjson(stdin, stdout, target, header_style)?;

    Ok(0)
}
