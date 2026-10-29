# Native Shell Agent Rules

## Command Runner

- Do not use PowerShell or pwsh as the project command language.
- If the active terminal/tool session is already Native Shell, run project commands directly. Do not wrap ordinary work in `native-shell -c`.
- Use `native-shell -c '<command>'` only when launching Native Shell from an external host process, or when deliberately testing fresh script-mode semantics.
- Ensure `native-shell` resolves to the intended installed user binary, usually from `PATH` or a user tools directory outside this repository such as `~/tools`. Do not commit developer-local absolute runner paths.
- Do not let `PATH` resolve `native-shell` to this repository's `target/release` or `target/debug` unless deliberately testing that exact build; those binaries can be stale or locked during builds.
- Use checked-out binaries only when deliberately testing that exact build, for example `target/debug/native-shell.exe --version` after `cargo build`.
- Keep commands Windows-native. Use normal Windows paths (`C:/Users/<user>/...` or `C:\Users\<user>\...`) and do not introduce MSYS2, Git Bash, Cygwin, or WSL assumptions.
- If `ls`, `grep`, `tr`, or similar Unix commands are missing, fix the winuxcmd command-link setup rather than switching shells. A release-style bundle needs `winuxcmd.exe` plus generated command links in `PATH`.

## Product Direction

- Native Shell is a Windows-native, non-isolated bash-compatible shell for humans and agents.
- Rubash is the shell language engine and is embedded as a library. Parser, executor, builtins, functions, redirects, pipelines, and job semantics belong upstream in `unixwin/rubash`.
- Rubash comes from the upstream `unixwin/rubash` tree. This repository is a personal renamed fork with **no write access to the unixwin organization**: upstream fixes cannot be pushed from here, so a needed engine change is carried as a patch against our own `JayWanX/rubash` fork and pinned by `scripts/release/rubash-pin.json`, never as a host-side workaround in native-shell's own semantics. Prefer an upstream-shaped patch that can be dropped once upstream adopts the same change.
- WinuxCmd stays integrated through PATH injection and command links. Do not reintroduce FFI/DLL command routing.
- The plugin system is Native Shell-native and built into native-shell. `oh-my-native-shell` is the official bundled plugin distribution.
- Use `~/.native-shellrc` as the primary-only interactive user entry point for plugin lists, theme selection, exports, aliases, functions, and shell startup logic. Do not read, migrate, delete, or modify the legacy `~/.winuxshrc` and `~/.winshrc` startup files. An explicitly opted-in `~/.bashrc` fallback is a future plan only, not supported behavior. Startup must not automatically run setup or plugin sync, create the primary rc, or rewrite user rc files. Issue #359 case A (1.4.2): plugin management never writes the rc at all — the manifest (`plugins.toml` + `sources/registry.toml`) drives the generated loader `~/.native-shell/generated/plugins.nsh`, which the host sources read-only BEFORE the user rc; `native-shell setup` still writes the rc (create/whole-file) until its 1.4.3 create-only convergence. Never re-attach the retired managed-block writers (`assets::write_managed_block`/`remove_managed_block`) to any verb — the 359.12 structure ratchet fails the build's test suite if you do.
- Runtime environment variables use the `NATIVE_SHELL_` prefix. During the transition, native-shell also sets/reads the deprecated `WINUXSH_SHELL` and `WINUXSH_ROOT` names because current rubash upstream still reads them; these bridges are marked with comments and must be removed once rubash renames its readers. (The `*_SHELL_PATH_STYLE` pair was retired rubash-side in rubash#556 / rubash#537: the engine always renders `$PWD`/`pwd` in the POSIX persona form, and native-shell no longer exports the variables.)
- Keep structured plugin and bundle assets in their manifest-backed TOML files; user startup configuration belongs in `~/.native-shellrc`.
- Use the manifest-backed registry as the control plane for every runtime. Use `source` packs with bundle-local `.nsh` scripts for reviewed shell helpers, keep `builtin` for host-owned native behavior and fallback, and keep process plugins for external-tool adapters and debug bridges.

## Development Rules

- Preserve quiet, deterministic non-interactive behavior for direct Native Shell project commands, `native-shell -c`, and script execution: no banners, stable stdout/stderr, exact exit-code propagation.
- Keep interactive UX features in native-shell/reedline unless they require shell semantics; shell semantics move to rubash.
- Keep compatibility tests honest: `tests/compat.rs` requires winuxcmd command links in the Windows process `PATH`, not just a bare `winuxcmd.exe`. When prepending a local WinuxCmd build for `cargo test`, use the Windows separator (`;`), for example `PATH="C:/path/to/WinuxCmd/build-vs-release;$PATH"`.
- Keep one authoritative dependency lock at the repository root. Do not let `crates/native-shell-runtime/Cargo.lock` drift from the binary build.
- When changing rubash, update the root lockfile and verify native-shell through the root package.
- Never run destructive git operations against the rubash working tree. If the rubash checkout shows any uncommitted changes (modified, staged, or untracked files), treat it as someone else's in-progress work. Prohibited commands include git stash, git stash push, git reset, git checkout that discards edits, git restore, git clean, and any checkout or switch that would hide local edits. When rubash's local state blocks your build or verification, stop and report the blocker to the user. Do not stash, reset, restore, clean, or rewind the rubash tree to make it compile.

## Verification

- Fast loop: `cargo fmt --check -p native-shell; cargo build --locked; cargo test --workspace --locked`
- Rename brand gate: `sh scripts/check-rename-clean.sh` must pass (residue is an exact baseline diff; runs under Git Bash and `winuxsh -c` alike). Intentional residue change → regenerate `scripts/check-rename-clean.baseline` via `sh scripts/check-rename-clean.sh --regen` and review the diff.
- Runtime library: `cargo test -p native-shell-runtime --lib --locked`
- Host contract requiring winuxcmd command links: ensure command links are in the Windows process `PATH`, then run `PATH="C:/path/to/WinuxCmd/build-vs-release;$PATH" WINUXCMD_PATH="C:/path/to/WinuxCmd/build-vs-release/winuxcmd.exe" cargo test --test host_contract --locked -- --ignored`
- Compat suite: ensure command links are in the Windows process `PATH`, then run `PATH="C:/path/to/WinuxCmd/build-vs-release;$PATH" WINUXCMD_PATH="C:/path/to/WinuxCmd/build-vs-release/winuxcmd.exe" cargo test --test compat --locked -- --ignored`
- Local GNU Bash upstream gate: `BASH_RUNNER="${BASH_RUNNER:-bash}"; "$BASH_RUNNER" scripts/run-bash-upstream-with-native-shell.sh` must report `86` total, `86` passed, `0` failed for the Native Shell binary under test. If `bash` is not in Native Shell's `PATH`, run the same gate with an explicit Git Bash path such as `C:/Progra~1/Git/bin/bash.exe`. Keep this local-only; do not add it to normal CI, and do not vendor Bash upstream tests into this repo. See `docs/planning/bash-upstream-local.md`.
