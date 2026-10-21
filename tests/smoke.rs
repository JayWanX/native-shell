//! Smoke tests with isolated HOME and oh-my-native-shell bundle.
//!
//! These tests spin up a temporary HOME, copy the oh-my-native-shell bundle into it,
//! run native-shell commands, and verify the setup wizard + plugin loading works end-to-end.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Path to the oh-my-native-shell bundle on J: drive (manual smoke test setup).
fn oh_my_native_shell_source() -> Option<PathBuf> {
    let candidate = PathBuf::from("J:/native-shell-smoke-test/oh-my-native-shell");
    if candidate.exists() {
        return Some(candidate);
    }
    let candidate = PathBuf::from("J:\\native-shell-smoke-test\\oh-my-native-shell");
    if candidate.exists() {
        return Some(candidate);
    }
    None
}

fn run_niu(script: &str, cwd: &Path, home: &Path) -> Output {
    let mut command = Command::new(native_shell_binary());
    command
        .arg("-c")
        .arg(script)
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home);
    command
        .output()
        .unwrap_or_else(|err| panic!("spawn native-shell: {err}"))
}

/// Run `native-shell -C` which loads .native-shellrc (interactive-mode command surface).
fn run_native_shell_interactive(script: &str, cwd: &Path, home: &Path) -> Output {
    let mut command = Command::new(native_shell_binary());
    command
        .arg("-C")
        .arg(script)
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home);
    command
        .output()
        .unwrap_or_else(|err| panic!("spawn native-shell -C: {err}"))
}

fn run_native_shell_script(script_path: &Path, cwd: &Path, home: &Path) -> Output {
    let mut command = Command::new(native_shell_binary());
    command
        .arg(script_path)
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home);
    command
        .output()
        .unwrap_or_else(|err| panic!("spawn native-shell: {err}"))
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{context} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nanos))
}

/// Set up an isolated HOME with oh-my-native-shell bundle and a minimal .native-shellrc.
fn setup_isolated_home(name: &str) -> PathBuf {
    let temp = unique_temp_dir(name);
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    // Copy oh-my-native-shell bundle if available
    if let Some(bundle_src) = oh_my_native_shell_source() {
        let bundle_dst = home.join(".oh-my-native-shell");
        copy_dir_recursive(&bundle_src, &bundle_dst).expect("copy oh-my-native-shell bundle");
    }

    temp
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn smoke_version() {
    let temp = unique_temp_dir("smoke-version");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu("native-shell --version", &temp, &home);
    assert_success(&output, "version check");
    let stdout = normalize(&output.stdout);
    assert!(
        stdout.contains("Native Shell") || stdout.contains("native-shell"),
        "expected version output, got: {stdout:?}"
    );
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_basic_commands() {
    let temp = unique_temp_dir("smoke-basic");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu("echo hello world", &temp, &home);
    assert_success(&output, "echo");
    assert_eq!(normalize(&output.stdout), "hello world");

    let output = run_niu("x=42; echo $x", &temp, &home);
    assert_success(&output, "variable assignment");
    assert_eq!(normalize(&output.stdout), "42");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_bash_features() {
    let temp = unique_temp_dir("smoke-bash");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    // For loop
    let output = run_niu("for i in 1 2 3; do echo $i; done", &temp, &home);
    assert_success(&output, "for loop");
    let normalized = normalize(&output.stdout);
    let lines: Vec<&str> = normalized.lines().collect();
    assert_eq!(lines, vec!["1", "2", "3"]);

    // Function
    let output = run_niu("greet() { echo \"Hi, $1!\"; }; greet World", &temp, &home);
    assert_success(&output, "function");
    assert_eq!(normalize(&output.stdout), "Hi, World!");

    // Pipe
    let output = run_niu("echo hello | tr '[:lower:]' '[:upper:]'", &temp, &home);
    assert_success(&output, "pipe");
    assert_eq!(normalize(&output.stdout), "HELLO");

    // Exit code
    let output = run_niu("false; echo $?", &temp, &home);
    assert_success(&output, "exit code");
    assert_eq!(normalize(&output.stdout), "1");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_isolated_home_has_no_rc() {
    let temp = unique_temp_dir("smoke-no-rc");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    // Without .native-shellrc, shell should still work
    let output = run_niu("echo it works", &temp, &home);
    assert_success(&output, "no-rc shell");
    assert_eq!(normalize(&output.stdout), "it works");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_rc_loading() {
    let temp = unique_temp_dir("smoke-rc");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    // Write a custom rc
    fs::write(
        home.join(".native-shellrc"),
        "SMOKE_RC_LOADED=1\nexport SMOKE_RC_LOADED\n",
    )
    .unwrap();

    // Use -C (interactive command mode) which loads .native-shellrc
    let output = run_native_shell_interactive("echo $SMOKE_RC_LOADED", &temp, &home);
    assert_success(&output, "rc loading");
    assert_eq!(normalize(&output.stdout), "1");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_with_oh_my_native_shell_bundle() {
    let Some(_bundle) = oh_my_native_shell_source() else {
        eprintln!("skipping: oh-my-native-shell bundle not found on J: drive");
        return;
    };

    let temp = setup_isolated_home("smoke-omn");
    let home = temp.join("home");

    // The built-in plugin stack retired (native-shell#145): legacy rc lines are
    // legal assignments the host now ignores; the rc must still source
    // cleanly and reach interactive commands.
    let rc_content = r#"NATIVE_SHELL_DISABLE_DEFAULT_PLUGINS=0
export NATIVE_SHELL_DISABLE_DEFAULT_PLUGINS
NATIVE_SHELL_PLUGINS=(git)
export NATIVE_SHELL_PLUGINS
if [ -f "$HOME/.oh-my-native_shell/oh-my-native_shell.nsh" ]; then
  . "$HOME/.oh-my-native_shell/oh-my-native_shell.nsh"
fi
"#;
    fs::write(home.join(".native-shellrc"), rc_content).unwrap();

    // Verify the bundle is loaded
    let output = run_native_shell_interactive(
        "if [ -d \"$HOME/.oh-my-native-shell\" ]; then echo bundle-found; else echo bundle-missing; fi",
        &temp,
        &home,
    );
    assert_success(&output, "bundle existence");
    assert_eq!(normalize(&output.stdout), "bundle-found");

    // NATIVE_SHELL_PLUGINS stays a plain exported array (parse tolerance).
    let output = run_native_shell_interactive("echo ${NATIVE_SHELL_PLUGINS[*]}", &temp, &home);
    assert_success(&output, "plugin array");
    assert!(
        normalize(&output.stdout).contains("git"),
        "NATIVE_SHELL_PLUGINS assignments must stay visible shell variables"
    );

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_script_execution() {
    let temp = unique_temp_dir("smoke-script");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let script = temp.join("test.sh");
    fs::write(
        &script,
        "#!/usr/bin/env bash\necho script-line-1\necho script-line-2\nexit 0\n",
    )
    .unwrap();

    let output = run_native_shell_script(&script, &temp, &home);
    assert_success(&output, "script execution");
    let stdout = normalize(&output.stdout);
    assert!(stdout.contains("script-line-1"), "stdout: {stdout:?}");
    assert!(stdout.contains("script-line-2"), "stdout: {stdout:?}");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_arrays_and_maps() {
    let temp = unique_temp_dir("smoke-arrays");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu("arr=(a b c); echo ${arr[1]}", &temp, &home);
    assert_success(&output, "array access");
    assert_eq!(normalize(&output.stdout), "b");

    let output = run_niu("arr=(x y z); echo ${#arr[@]}", &temp, &home);
    assert_success(&output, "array length");
    assert_eq!(normalize(&output.stdout), "3");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_string_operations() {
    let temp = unique_temp_dir("smoke-strings");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu("s='hello world'; echo ${#s}", &temp, &home);
    assert_success(&output, "string length");
    assert_eq!(normalize(&output.stdout), "11");

    let output = run_niu("s='hello world'; echo ${s:0:5}", &temp, &home);
    assert_success(&output, "substring");
    assert_eq!(normalize(&output.stdout), "hello");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_arithmetic() {
    let temp = unique_temp_dir("smoke-arithmetic");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu("echo $((2 + 3))", &temp, &home);
    assert_success(&output, "arithmetic");
    assert_eq!(normalize(&output.stdout), "5");

    let output = run_niu("x=10; ((x += 5)); echo $x", &temp, &home);
    assert_success(&output, "arithmetic assignment");
    assert_eq!(normalize(&output.stdout), "15");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_conditional_expressions() {
    let temp = unique_temp_dir("smoke-conditionals");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let output = run_niu(
        "if [ 1 -eq 1 ]; then echo yes; else echo no; fi",
        &temp,
        &home,
    );
    assert_success(&output, "if/else");
    assert_eq!(normalize(&output.stdout), "yes");

    let output = run_niu(
        "case hello in hello) echo match;; *) echo no-match;; esac",
        &temp,
        &home,
    );
    assert_success(&output, "case");
    assert_eq!(normalize(&output.stdout), "match");

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_setup_wizard_generates_rc() {
    let temp = unique_temp_dir("smoke-wizard");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    // Run the wizard in reconfigure mode (non-interactive, just to test rc generation)
    // The wizard needs interactive stdin for prompt_yn/prompt_choice, so we pipe defaults
    let mut command = Command::new(native_shell_binary());
    command
        .arg("setup")
        .current_dir(&temp)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("spawn native-shell setup");

    // Feed default answers: Y/n for each prompt (all defaults)
    if let Some(stdin) = child.stdin.as_mut() {
        // Minimal preset (just Enter), then all the wizard prompts
        let answers = "\n\ny\n\n\n\n\n\n\n\n\n\n\n\n";
        use std::io::Write;
        let _ = stdin.write_all(answers.as_bytes());
    }

    let output = child.wait_with_output().expect("wait for setup");
    let stderr = normalize(&output.stderr);

    // The rc file should be created
    let rc_path = home.join(".native-shellrc");
    assert!(
        rc_path.exists(),
        "setup wizard should create .native-shellrc\nstderr: {stderr}"
    );

    if rc_path.exists() {
        let rc_content = fs::read_to_string(&rc_path).unwrap();
        // The wizard writes a clean rc (native-shell#145): no stack variables.
        assert!(
            !rc_content.contains("NATIVE_SHELL_THEME="),
            "rc must not contain NATIVE_SHELL_THEME: {rc_content}"
        );
        assert!(
            !rc_content.contains("NATIVE_SHELL_PLUGINS="),
            "rc must not contain NATIVE_SHELL_PLUGINS: {rc_content}"
        );
        assert!(
            rc_content.contains("USERPROFILE"),
            "rc should bootstrap HOME from USERPROFILE"
        );
    }

    // The setup-done marker should be created
    let marker = home.join(".native-shell").join(".setup-done");
    assert!(
        marker.exists(),
        "setup wizard should create .setup-done marker"
    );

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn smoke_piped_stdin_script() {
    let temp = unique_temp_dir("smoke-piped");
    let home = temp.join("home");
    fs::create_dir_all(&home).unwrap();

    let mut command = Command::new(native_shell_binary());
    command
        .current_dir(&temp)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().expect("spawn native-shell piped");
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let _ = stdin.write_all(b"echo piped-input\nexit\n");
    }

    let output = child.wait_with_output().expect("wait for piped");
    assert_success(&output, "piped stdin");
    assert!(normalize(&output.stdout).contains("piped-input"));

    let _ = fs::remove_dir_all(temp);
}
