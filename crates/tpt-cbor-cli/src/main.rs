//! `tpt-cbor`: a command-line tool for CBOR (RFC 8949).
//!
//! Each command reads one input (a file, or standard input) and writes one
//! output (a file, or standard output).
//!
//! Exit status: 0 on success, 1 when the input is not valid for the command,
//! and 2 for a usage error or an I/O failure.

mod json;

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use tpt_cbor::{
    from_slice_located_with_config, to_vec, validate, Config, Located, OwnedValue, Value,
};

const USAGE: &str = "\
Usage: tpt-cbor <command> [options]

Commands:
  hex-encode     Binary CBOR to lowercase hex text
  hex-decode     Hex text (whitespace allowed) to binary CBOR
  diag           Print CBOR in diagnostic notation (RFC 8949 section 8)
  validate       Check that the input is exactly one CBOR data item
  canon          Rewrite the input in canonical form
  cbor-to-json   Convert CBOR to JSON
  json-to-cbor   Convert JSON to canonical CBOR

Options:
  -i, --input FILE    Read input from FILE (default: standard input)
  -o, --output FILE   Write output to FILE (default: standard output)
  --strict            For validate: reject non-canonical input
  -h, --help          Show this help
  -V, --version       Show the version

Exit status: 0 success, 1 invalid input, 2 usage or I/O error.";

/// Why the tool stopped without success.
enum Failure {
    /// Bad arguments: exit status 2.
    Usage(String),
    /// I/O failure: exit status 2.
    Io(String),
    /// The input is not valid for the command: exit status 1.
    Data(String),
}

#[derive(Default)]
struct Options {
    input: Option<String>,
    output: Option<String>,
    strict: bool,
}

type Transform = fn(&[u8], &Options) -> Result<Vec<u8>, Failure>;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Usage(message)) => {
            eprintln!("tpt-cbor: {message}");
            eprintln!("Run 'tpt-cbor --help' for usage.");
            ExitCode::from(2)
        }
        Err(Failure::Io(message)) => {
            eprintln!("tpt-cbor: {message}");
            ExitCode::from(2)
        }
        Err(Failure::Data(message)) => {
            eprintln!("tpt-cbor: {message}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), Failure> {
    let mut args = args.into_iter();
    let command = match args.next() {
        Some(flag) if flag == "-h" || flag == "--help" => {
            println!("{USAGE}");
            return Ok(());
        }
        Some(flag) if flag == "-V" || flag == "--version" => {
            println!("tpt-cbor {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Some(command) => command,
        None => return Err(Failure::Usage("missing command".to_string())),
    };

    let mut options = Options::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" | "--input" => options.input = Some(value_of(&mut args, &arg)?),
            "-o" | "--output" => options.output = Some(value_of(&mut args, &arg)?),
            "--strict" => options.strict = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(Failure::Usage(format!("unknown option `{other}`"))),
        }
    }

    // Look the command up before reading input, so an unknown command does
    // not wait on standard input.
    let transform: Transform = match command.as_str() {
        "hex-encode" => hex_encode,
        "hex-decode" => hex_decode,
        "diag" => diag,
        "validate" => validate_command,
        "canon" => canon,
        "cbor-to-json" => cbor_to_json,
        "json-to-cbor" => json_to_cbor,
        other => return Err(Failure::Usage(format!("unknown command `{other}`"))),
    };

    let input = read_input(options.input.as_deref())?;
    let output = transform(&input, &options)?;
    write_output(options.output.as_deref(), &output)
}

fn value_of(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, Failure> {
    args.next()
        .ok_or_else(|| Failure::Usage(format!("`{flag}` needs a value")))
}

fn read_input(path: Option<&str>) -> Result<Vec<u8>, Failure> {
    match path {
        Some(path) => fs::read(path).map_err(|e| Failure::Io(format!("cannot read `{path}`: {e}"))),
        None => {
            let mut buffer = Vec::new();
            io::stdin()
                .lock()
                .read_to_end(&mut buffer)
                .map_err(|e| Failure::Io(format!("cannot read standard input: {e}")))?;
            Ok(buffer)
        }
    }
}

fn write_output(path: Option<&str>, bytes: &[u8]) -> Result<(), Failure> {
    match path {
        Some(path) => {
            fs::write(path, bytes).map_err(|e| Failure::Io(format!("cannot write `{path}`: {e}")))
        }
        None => {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(bytes)
                .and_then(|()| stdout.flush())
                .map_err(|e| Failure::Io(format!("cannot write standard output: {e}")))
        }
    }
}

/// Turns a decode failure into a data error that includes the byte offset.
fn data_error(located: Located) -> Failure {
    Failure::Data(format!("invalid CBOR: {located}"))
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn hex_encode(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let mut text = String::with_capacity(input.len() * 2 + 1);
    for byte in input {
        let _ = write!(text, "{byte:02x}");
    }
    text.push('\n');
    Ok(text.into_bytes())
}

fn hex_decode(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let text = std::str::from_utf8(input)
        .map_err(|_| Failure::Data("hex input is not UTF-8 text".to_string()))?;
    let digits: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if digits.len() % 2 != 0 {
        return Err(Failure::Data(
            "hex input has an odd number of digits".to_string(),
        ));
    }
    let mut out = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        let mut byte = 0u8;
        for &c in pair {
            let digit = c
                .to_digit(16)
                .ok_or_else(|| Failure::Data(format!("`{c}` is not a hex digit")))?;
            // Two hex digits always fit in a byte.
            byte = byte * 16 + digit as u8;
        }
        out.push(byte);
    }
    Ok(out)
}

fn diag(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let value: Value<'_> =
        from_slice_located_with_config(input, Config::lenient()).map_err(data_error)?;
    Ok(format!("{value}\n").into_bytes())
}

fn validate_command(input: &[u8], options: &Options) -> Result<Vec<u8>, Failure> {
    let config = if options.strict {
        Config::strict()
    } else {
        Config::lenient()
    };
    validate(input, config).map_err(data_error)?;
    Ok(Vec::new())
}

fn canon(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let value: Value<'_> =
        from_slice_located_with_config(input, Config::lenient()).map_err(data_error)?;
    to_vec(&value).map_err(|e| Failure::Data(format!("cannot encode: {e}")))
}

fn cbor_to_json(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let value: OwnedValue =
        from_slice_located_with_config(input, Config::lenient()).map_err(data_error)?;
    let mut text = json::write(&value).map_err(Failure::Data)?;
    text.push('\n');
    Ok(text.into_bytes())
}

fn json_to_cbor(input: &[u8], _: &Options) -> Result<Vec<u8>, Failure> {
    let text = std::str::from_utf8(input)
        .map_err(|_| Failure::Data("JSON input is not UTF-8 text".to_string()))?;
    let value = json::parse(text).map_err(Failure::Data)?;
    to_vec(&value).map_err(|e| Failure::Data(format!("cannot encode: {e}")))
}
