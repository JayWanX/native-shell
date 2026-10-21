//! native-shell#195: `native-shell setup` output must degrade to plain text when stdout
//! is not a terminal. `--help` answers in plain text everywhere (piped or
//! not), and the non-tty wizard banner drops the ANSI pixel-art logo.
use std::path::PathBuf;
use std::process::{Command, Output};

fn native_shell_binary() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_BIN_EXE_native-shell"));
    if p.exists() {
        return p;
    }
    let mut fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fallback.push("target");
    fallback.push("debug");
    fallback.push(if cfg!(windows) {
        "native-shell.exe"
    } else {
        "native-shell"
    });
    fallback
}

fn run_niu(args: &[&str]) -> Output {
    Command::new(native_shell_binary())
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run native-shell {args:?}: {err}"))
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn noninteractive_setup_welcome_degrades_to_plain_text() {
    // A piped `native-shell setup` used to paint the full ANSI pixel-art logo
    // (native-shell#195). With stdout not a tty the welcome banner must be
    // plain text while the wizard still completes.
    let temp = std::env::temp_dir().join(format!(
        "native-shell-issue195-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&temp);
    let home = temp.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let output = Command::new(native_shell_binary())
        .args(["setup"])
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("NATIVE_SHELL_PLUGIN_SOURCES_ROOT", temp.join("sources"))
        .env("NATIVE_SHELL_LANG", "en")
        .output()
        .expect("run native-shell setup");
    assert!(
        output.status.success(),
        "non-interactive setup failed:\n{}",
        stdout_text(&output)
    );
    let stdout = stdout_text(&output);
    assert!(
        !stdout.contains('\u{1b}'),
        "non-tty setup output must contain no ANSI escape sequences, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Welcome to NativeShell"),
        "welcome text should survive the plain-text degradation, got:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn setup_help_is_plain_text_and_never_prints_ansi_art() {
    let output = run_niu(&["setup", "--help"]);
    assert!(
        output.status.success(),
        "setup --help failed: {}",
        output.status
    );
    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("native-shell setup"),
        "--help should print plain-text usage, got:\n{stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "--help output must contain no ANSI escape sequences, got:\n{stdout}"
    );
}

#[test]
fn setup_help_shorthand_h_is_plain_text_too() {
    let output = run_niu(&["setup", "-h"]);
    assert!(
        output.status.success(),
        "setup -h failed: {}",
        output.status
    );
    let stdout = stdout_text(&output);
    assert!(
        stdout.contains("Usage"),
        "-h should print usage, got:\n{stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "-h output must contain no ANSI escape sequences, got:\n{stdout}"
    );
}
