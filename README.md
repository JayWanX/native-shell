<p align="center">
  <img src="assets/native-shell-banner.svg" alt="native-shell — real Bash, native on Windows, Linux, and macOS." />
</p>

> **Real Bash, native on Windows, Linux, and macOS.** "Windows-only" is the
> old story: five platforms build from this tree now, and Windows remains
> the flagship — no WSL. No VM. No `/mnt/c`. No cmdlet dialect. One binary:
> the shell your fingers already know, and the one your AI agent actually
> speaks.

<div align="center">

[English](README.md) · [中文](README-zh.md)

[![native-shell CI](https://github.com/JayWanX/native-shell/actions/workflows/ci.yml/badge.svg)](https://github.com/JayWanX/native-shell/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/JayWanX/native-shell)](https://github.com/JayWanX/native-shell/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)](https://github.com/JayWanX/native-shell)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange)](https://github.com/JayWanX/native-shell)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Stars](https://img.shields.io/github/stars/JayWanX/native-shell)](https://github.com/JayWanX/native-shell/stargazers)

</div>

**native-shell** is a Bash-compatible shell implemented natively in Rust. On
Windows, the flagship platform, one `native-shell.exe` bundles everything: the
[rubash](https://github.com/unixwin/rubash) language engine, real Unix
commands from [winuxcmd](https://github.com/unixwin/winuxcmd), a git-aware
prompt, and a permission-modeled plugin system. Git comes from
[niu-git](https://github.com/unixwin/niu-git): the real git-for-windows C
source built with MSVC, so the `git` on PATH *is* the git binary — no MSYS
runtime, no launcher hop, no bundled perl. `git --version` starts in **~14
ms** where Git Bash's `git` takes 45-103 ms, and the shell-script parts of
git (hooks, `git bisect`, `git mergetool`) run on native-shell's own `sh`. Agents
launched from a native-shell session inherit it as their shell through `SHELL`; see
[Coding agents on Windows](docs/src/agents.md) for which shell each agent runs
and how to point the rest at native-shell. The
Linux and macOS builds
are portable `native-shell` binaries that use your system's own tools. native-shell is not
MSYS2, Cygwin, Git Bash, or WSL — there is no POSIX emulation layer and no
path-conversion heuristics. The full per-platform contract, including the
honest status of every target, lives in
[platform-support.md](docs/src/platform-support.md).

## Platforms

| Platform | Status | Ships as |
|---|---|---|
| Windows x64 / ARM64 | Released; smoke-gated every release | installer `.exe` + portable `.tar.gz`/`.7z`/`.zip`, Unix commands bundled |
| Linux x86_64 / aarch64 (glibc 2.35+) | Released; smoke-gated every release | portable `.tar.gz`, uses system tools |
| macOS aarch64 / x86_64 | Released; smoke-gated every release | portable `.tar.gz`, uses system tools |
| Android aarch64 / armv7 | CI build only, no release | NDK-linked zip on workflow runs, not smoke-tested |
| OpenHarmony (`aarch64-unknown-linux-ohos`) | CI check only, no release | kept compiling by CI, never run on a device |

"Smoke-gated" means the artifact is built, then run on its own OS against
four checks (`--version`, `-c` execution, a pipeline through native
coreutils, the offline plugin stack) before upload. Android and OpenHarmony
have not passed that gate: Android artifacts are build-verified only,
OpenHarmony is compile-checked only, and neither claims device readiness.
The per-platform contract and the full status matrix live in
[platform-support.md](docs/src/platform-support.md).

Quick answers:

- **Does native-shell require WSL?** No. native-shell is a native build on every platform it
  ships for: on Windows there is no Linux VM, no emulation layer, no
  `/mnt/c`.
- **What platforms does native-shell run on?** Released builds cover Windows (x64,
  ARM64), Linux (x86_64, aarch64; glibc 2.35+), and macOS (aarch64,
  x86_64). Android (aarch64, armv7) and OpenHarmony (aarch64) build in CI
  and have no release yet.
- **Does native-shell run real Bash scripts?** Yes. The rubash engine is gated on
  GNU Bash's own upstream test suites; the record lives in the
  [compatibility matrix](docs/src/rubash-bash-compat-matrix.md).
- **Can I install native-shell on Android or HarmonyOS today?** Not yet. Both
  targets compile in CI, neither has passed a runtime smoke test, so there
  is nothing to install.

## Receipts

Each claim below links to the artifact that produced it: a committed log, a
baseline file, a CI gate.

**6,336 real-world shell assets, harvested and replayed one by one.** The
pipeline pulls shell scripts, frameworks, themes, and dotfiles from GitHub
and sources each one under native-shell, recording a per-asset verdict: 4,891 OK,
350 slow, 224 that GNU Bash also fails, 101 hangs, 698 that never fetched.
The full log is committed
([eco-harvest.py](scripts/harvest/eco-harvest.py),
[eco-test.py](scripts/harvest/eco-test.py), snapshot
[wt92-local-20261005](scripts/harvest/snapshots/wt92-local-20261005/)).

**Framework assets are sourced and timed, not just "supported."** The
timing baseline loads 437 real oh-my-bash and bash-it assets (166 themes
plus plugins, completions, aliases) and measures three phases per asset
(source, first prompt, re-render) as a median of three runs. That includes
the nvm assets of both frameworks, and real users' dotfiles that load
nvm.sh go through the same harness with per-phase timings on record
([baseline](scripts/perf/baselines/asset-timing-baseline.json),
[snapshot](scripts/harvest/snapshots/wt92-local-20261005/)).

**A slow theme shows up within a day.** Every asset in
[budgets.toml](scripts/perf/budgets.toml) carries a measured budget
(healthy themes re-render in 11-35 ms; the budget line is 120 ms; 5x over
budget is a hard fail). The nightly `perfbudget-trend` job in
[ci.yml](.github/workflows/ci.yml) re-measures every asset against the
committed baseline and records the full per-asset trend.

The GNU Bash upstream golden-suite record belongs here too; it returns
once the fix currently in flight lands.

## Quick start

Windows: grab `native-shell-v*-win-*-setup.exe` from
[Releases](https://github.com/JayWanX/native-shell/releases) and run it — no
admin rights; PATH and a Windows Terminal profile are set up for you.
Prefer portable? Take the `.tar.gz` — it ships the bundled git/awk payload
as real hardlinks, so it downloads and unpacks much smaller than the `.zip`
(`tar -xf native-shell-v*-win-x64.tar.gz`, built into Windows 10+). The `.zip`
stays for tooling that only speaks zip.

Linux (x86_64, aarch64; glibc 2.35+) and macOS (aarch64, x86_64): take the
portable tarball, untar, run:

```sh
tar -xzf native-shell-v*-linux-x86_64.tar.gz && native-shell-v*-linux-x86_64/native-shell
```

From source (Rust 1.95+):
`git clone https://github.com/JayWanX/native-shell.git && cd native-shell && cargo build --release`

Configuration is one file, `~/.native-shellrc`, in plain Bash syntax — start
with [Getting started](docs/src/getting-started.md).

## Documentation

[Docs site](https://unixwin.github.io/native-shell/) ·
[Why native-shell](docs/src/why-native-shell.md) (the long version, with the agent
casualties) · [Platform support](docs/src/platform-support.md) ·
[Bash compatibility matrix](docs/src/rubash-bash-compat-matrix.md) ·
[Roadmap](docs/src/native-shell-roadmap.md)

---

If native-shell saved you a Linux VM, a mangled quote, or an argument PowerShell
ate, [star the repo](https://github.com/JayWanX/native-shell) and tell a
developer. ★

## License

MIT. See [LICENSE](LICENSE).
