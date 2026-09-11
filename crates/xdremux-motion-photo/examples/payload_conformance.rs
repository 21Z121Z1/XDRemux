use std::{env, path::Path, process::ExitCode};

use serde_json::{json, Value};
use xdremux_motion_photo::{copy_payload_range_with_options, ByteRange, MotionPhotoCopyError};

fn parse_u64(text: &str) -> Result<u64, String> {
    text.parse::<u64>()
        .map_err(|error| format!("invalid integer {text:?}: {error}"))
}

fn parse_usize(text: &str) -> Result<usize, String> {
    text.parse::<usize>()
        .map_err(|error| format!("invalid integer {text:?}: {error}"))
}

fn error_json(error: MotionPhotoCopyError) -> Value {
    match error {
        MotionPhotoCopyError::MotionPhoto(error) => {
            json!({"status": "error", "kind": "motionPhoto", "code": error.code()})
        }
        MotionPhotoCopyError::Io(error) => {
            json!({"status": "error", "kind": "io", "ioKind": format!("{:?}", error.kind())})
        }
    }
}

fn run() -> Result<Value, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 6 {
        return Err(
            "usage: payload_conformance <source> <lower> <upper> <destination> <max-bytes> <buffer-size>"
                .into(),
        );
    }

    let range = match ByteRange::new(parse_u64(&args[1])?, parse_u64(&args[2])?) {
        Ok(range) => range,
        Err(error) => {
            return Ok(json!({
                "status": "error",
                "kind": "motionPhoto",
                "code": error.code(),
            }));
        }
    };
    let max_bytes = parse_u64(&args[4])?;
    let buffer_size = parse_usize(&args[5])?;

    Ok(
        match copy_payload_range_with_options(
            Path::new(&args[0]),
            range,
            Path::new(&args[3]),
            max_bytes,
            buffer_size,
        ) {
            Ok(()) => json!({"status": "ok"}),
            Err(error) => error_json(error),
        },
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
