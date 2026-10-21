//! rubash#382 / native-shell#382: GNU clustered invocation options.
//!
//! `bash -eo pipefail -c '...'` is a common CI / Makefile invocation: GNU
//! shell.c parse_shell_options (shell.c:931-943) reads `-eo` as `-e` plus
//! `-o` whose value is the NEXT argv word. native-shell used to reject the cluster
//! with `bash: -eo: invalid option`, so any tool that invoked the shell that
//! way failed. These tests pin the clustered form to the separated one.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

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

fn run(args: &[&str]) -> Output {
    Command::new(native_shell_binary())
        .args(args)
        .env_remove("NATIVE_SHELL_ENV")
        .env_remove("BASH_ENV")
        .stdin(Stdio::null())
        .output()
        .expect("spawn native-shell")
}

#[test]
fn clustered_eo_pipefail_matches_the_separated_form() {
    let clustered = run(&["-eo", "pipefail", "-c", "true"]);
    let separated = run(&["-e", "-o", "pipefail", "-c", "true"]);
    assert!(
        clustered.status.success(),
        "`native-shell -eo pipefail -c true` failed: status {:?}, stderr: {}",
        clustered.status.code(),
        String::from_utf8_lossy(&clustered.stderr)
    );
    assert_eq!(
        clustered.status.code(),
        separated.status.code(),
        "clustered and separated -eo pipefail differ"
    );
    assert_eq!(clustered.stdout, separated.stdout);
}

#[test]
fn clustered_euo_pipefail_keeps_every_flag() {
    // `-euo pipefail` == `-e -u -o pipefail`: errexit must actually fire.
    let out = run(&["-euo", "pipefail", "-c", "false; echo unreachable"]);
    assert!(
        !out.status.success(),
        "errexit did not fire under -euo: stderr {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "errexit ran the following command: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn clustered_shopt_option_matches_the_separated_form() {
    let clustered = run(&["-eO", "extglob", "-c", "true"]);
    let separated = run(&["-e", "-O", "extglob", "-c", "true"]);
    assert!(
        clustered.status.success(),
        "`native-shell -eO extglob -c true` failed: status {:?}, stderr: {}",
        clustered.status.code(),
        String::from_utf8_lossy(&clustered.stderr)
    );
    assert_eq!(clustered.status.code(), separated.status.code());
}
