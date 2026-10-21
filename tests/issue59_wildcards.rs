use std::path::PathBuf;
use std::process::Command;

fn native_shell_binary() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_BIN_EXE_native-shell"));
    if p.exists() {
        return p;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "native-shell.exe"
        } else {
            "native-shell"
        })
}

#[test]
fn native_command_arguments_preserve_quoted_wildcards() {
    let output = Command::new(native_shell_binary())
        .arg("-c")
        .arg(r#"printf '<%s>\n' "a*b" "q?x" "/CN=test" --send-only"#)
        .env("NATIVE_SHELL_SKIP_WINUXCMD_ACTIVATION", "1")
        .output()
        .expect("spawn local native-shell");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "<a*b>\n<q?x>\n</CN=test>\n<--send-only>\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
}
