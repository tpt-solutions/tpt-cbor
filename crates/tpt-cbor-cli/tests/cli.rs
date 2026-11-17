//! End-to-end tests for the `tpt-cbor` binary: each command, the exit
//! statuses, file options and the JSON edge cases.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_tpt-cbor");

struct Outcome {
    code: i32,
    stdout: Vec<u8>,
    stderr: String,
}

fn run(args: &[&str], stdin: &[u8]) -> Outcome {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tpt-cbor");
    {
        let mut pipe = child.stdin.take().expect("stdin pipe");
        pipe.write_all(stdin).expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait for tpt-cbor");
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Parses whitespace-separated hex into bytes.
fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
        .collect()
}

/// A temporary path unique to this test process and name.
fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tpt-cbor-cli-{}-{name}", std::process::id()))
}

#[test]
fn hex_encode_and_decode_round_trip() {
    let encoded = run(&["hex-encode"], &hex("a2 61 61 01"));
    assert_eq!(encoded.code, 0);
    assert_eq!(encoded.stdout, b"a2616101\n");

    let decoded = run(&["hex-decode"], b"a2 61 61 01\n");
    assert_eq!(decoded.code, 0);
    assert_eq!(decoded.stdout, hex("a2 61 61 01"));
}

#[test]
fn hex_decode_rejects_bad_input_with_status_1() {
    assert_eq!(run(&["hex-decode"], b"abc").code, 1, "odd digit count");
    let bad = run(&["hex-decode"], b"zz");
    assert_eq!(bad.code, 1);
    assert!(bad.stderr.contains("not a hex digit"), "{}", bad.stderr);
}

#[test]
fn diag_prints_diagnostic_notation() {
    let out = run(&["diag"], &hex("a2 61 61 01 61 62 82 02 03"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout, b"{\"a\": 1, \"b\": [2, 3]}\n");
}

#[test]
fn validate_is_lenient_by_default_and_strict_on_request() {
    // 0x18 0x01 is a non-minimal integer.
    assert_eq!(run(&["validate"], &hex("18 01")).code, 0);

    let strict = run(&["validate", "--strict"], &hex("18 01"));
    assert_eq!(strict.code, 1);
    assert!(
        strict.stderr.contains("non-canonical encoding at byte 2"),
        "{}",
        strict.stderr
    );
    assert_eq!(run(&["validate", "--strict"], &hex("01")).code, 0);
}

#[test]
fn validate_rejects_trailing_bytes() {
    let out = run(&["validate"], &hex("01 02"));
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("trailing bytes"), "{}", out.stderr);
}

#[test]
fn canon_rewrites_to_canonical_bytes() {
    assert_eq!(run(&["canon"], &hex("18 01")).stdout, hex("01"));
    // Unsorted keys are reordered.
    assert_eq!(
        run(&["canon"], &hex("a2 61 62 01 61 61 02")).stdout,
        hex("a2 61 61 02 61 62 01")
    );
}

#[test]
fn cbor_to_json_converts_maps_and_arrays() {
    let out = run(&["cbor-to-json"], &hex("a2 61 61 01 61 62 82 02 03"));
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout, b"{\"a\":1,\"b\":[2,3]}\n");
}

#[test]
fn cbor_to_json_rejects_values_without_a_json_form() {
    let bytes = run(&["cbor-to-json"], &hex("41 01"));
    assert_eq!(bytes.code, 1);
    assert!(bytes.stderr.contains("byte strings"), "{}", bytes.stderr);

    // A map with an integer key.
    let key = run(&["cbor-to-json"], &hex("a1 01 02"));
    assert_eq!(key.code, 1);
    assert!(key.stderr.contains("must be text"), "{}", key.stderr);
}

#[test]
fn json_to_cbor_writes_canonical_bytes() {
    let out = run(&["json-to-cbor"], br#"{"b": [2, 3], "a": 1}"#);
    assert_eq!(out.code, 0);
    assert_eq!(out.stdout, hex("a2 61 61 01 61 62 82 02 03"));
}

#[test]
fn json_strings_round_trip_through_cbor() {
    let json = br#""x\"\\\n\u00e9\ud83d\ude00""#;
    let cbor = run(&["json-to-cbor"], json);
    assert_eq!(cbor.code, 0);
    let back = run(&["cbor-to-json"], &cbor.stdout);
    assert_eq!(back.code, 0);
    assert_eq!(back.stdout, "\"x\\\"\\\\\\n\u{e9}\u{1f600}\"\n".as_bytes());
}

#[test]
fn json_integers_out_of_range_are_rejected() {
    let out = run(&["json-to-cbor"], b"9223372036854775808");
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("64-bit"), "{}", out.stderr);
}

#[test]
fn json_floats_become_cbor_floats() {
    assert_eq!(run(&["json-to-cbor"], b"1.5").stdout, hex("f9 3e 00"));
}

#[test]
fn invalid_json_is_rejected_with_status_1() {
    for bad in [&b"{"[..], b"[1,]", b"tru", b"\"\\x\"", b"1 2", b"\"\x01\""] {
        let out = run(&["json-to-cbor"], bad);
        assert_eq!(out.code, 1, "input {:?}", String::from_utf8_lossy(bad));
    }
}

#[test]
fn json_nesting_is_bounded() {
    let deep = "[".repeat(200);
    let out = run(&["json-to-cbor"], deep.as_bytes());
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("too deep"), "{}", out.stderr);
}

#[test]
fn usage_errors_exit_with_status_2() {
    assert_eq!(run(&[], b"").code, 2, "no command");
    assert_eq!(run(&["nope"], b"").code, 2, "unknown command");
    assert_eq!(run(&["diag", "--bogus"], b"").code, 2, "unknown option");
    assert_eq!(run(&["diag", "-i"], b"").code, 2, "missing value");
}

#[test]
fn help_and_version_succeed() {
    let help = run(&["--help"], b"");
    assert_eq!(help.code, 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage"));
    let version = run(&["--version"], b"");
    assert_eq!(version.code, 0);
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("tpt-cbor "));
}

#[test]
fn input_and_output_files_are_used() {
    let input = temp_path("in.cbor");
    let output = temp_path("out.hex");
    fs::write(&input, hex("a1 61 61 01")).expect("write input");

    let out = run(
        &[
            "hex-encode",
            "-i",
            input.to_str().expect("utf-8 path"),
            "-o",
            output.to_str().expect("utf-8 path"),
        ],
        b"",
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(out.stdout.is_empty(), "output went to the file, not stdout");
    assert_eq!(fs::read(&output).expect("read output"), b"a1616101\n");

    let _ = fs::remove_file(&input);
    let _ = fs::remove_file(&output);
}

#[test]
fn missing_input_file_is_an_io_error() {
    let missing = temp_path("does-not-exist.cbor");
    let out = run(&["diag", "-i", missing.to_str().expect("utf-8 path")], b"");
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("cannot read"), "{}", out.stderr);
}
