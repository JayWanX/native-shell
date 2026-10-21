# Advanced Native Shell Usage

This guide covers the surfaces that matter after the first successful launch:
execution modes, startup files, prompt/theme plugins, command discovery, and
update/debug workflows.

For first-time setup, start with [Getting Started](getting-started.md).

## Execution Modes

Native Shell has three intentionally different execution paths:

```pwsh
native-shell                         # interactive REPL
native-shell -c 'pwd; echo "$SHELL"'  # quiet script/CI command mode
native-shell -C 'alias ll; pwd'       # one-shot REPL command
```

- Use the interactive REPL for normal shell work.
- Use `-c` for scripts, tests, CI, and coding agents. It does not load
  `~/.native-shellrc`, `~/.winshrc`, prompt plugins, or interactive lifecycle hooks.
- Use `-C` only when a one-shot command needs the same startup state as the
  interactive REPL. It loads `~/.native-shellrc` and lifecycle hooks, then exits.

This separation keeps automation deterministic while still allowing a rich
interactive shell.

### One-shot init file: `NATIVE_SHELL_ENV` / `BASH_ENV`

By default `-c` loads nothing — no rc, no plugins, no hooks. If your script,
CI job, or coding agent needs shell aliases, exported variables, or PATH
tweaks, point `NATIVE_SHELL_ENV` (or bash-compatible `BASH_ENV`) at a dedicated init
file. Only that one file is sourced, so the interactive-only content in
`~/.native-shellrc` stays out of the one-shot path.

```bash
# ~/.opencode.env — a minimal init file for agents
export PATH="$HOME/tools:$PATH"
alias ll='ls -la'
export DOCKER_CONTEXT=my-cluster
```

```bash
NATIVE_SHELL_ENV=~/.opencode.env native-shell -c 'll | head'
# or bash-compatible:
BASH_ENV=~/.opencode.env native-shell -c 'echo "$DOCKER_CONTEXT"'
```

`NATIVE_SHELL_ENV` takes precedence over `BASH_ENV` when both are set. A leading `~`
expands to your home directory. If the file does not exist, native-shell prints
an error to stderr and continues — matching GNU bash's `BASH_ENV` behavior.

With neither variable set, `-c` stays zero-load and fast.

## Startup And Config

Use `~/.native-shellrc` as the normal human-authored entry point:

```bash
# Optional floor knobs — they only shape the built-in default prompt and
# completion UI; an enabled external theme claims PS1 and wins.
# NATIVE_SHELL_PROMPT_CWD_STYLE='home'
# NATIVE_SHELL_COMPLETION_STYLE='gnu'   # gnu | ide | column | list | inline

alias ll='ls -la'
export EDITOR=vim

# starship (or any tool that sets PS1) owns the prompt while it is active:
# eval "$(starship init bash)"
```

Themes and plugins are enabled through `native-shell plugin`, which appends managed
blocks at the end of the rc (see [Plugin Workflow](#plugin-workflow)); your
lines outside those blocks are never rewritten.

The legacy files still exist, but they should not be the primary user path:

- `~/.winshrc` is a fallback only when `~/.native-shellrc` is absent.
- Plugin CLI records, migration blocks, bundle versions, tests, and advanced
  overrides are internal managed state, not a user configuration file.

Do not put automation-critical behavior only in an interactive rc file. Pass
needed environment variables directly to `native-shell -c` or the script process, or
use `NATIVE_SHELL_ENV` / `BASH_ENV` to source a dedicated one-shot init file (see
[One-shot init file](#one-shot-init-file-native_shell_env--bash_env)).

## Completion Styles

TAB completion comes in five shapes. The default since 1.4.1 is `gnu`
(GNU readline factory behavior); before that it was `ide`. Pick one with
`native-shell config set completion.style <name>` (persisted in
`~/.native-shell/config.toml`) or `NATIVE_SHELL_COMPLETION_STYLE=<name>` in
`~/.native-shellrc` or the environment — the env variable wins over the
persisted value:

- `gnu` (default) — GNU readline: TAB completes the longest common prefix
  (a lone candidate completes outright); pressing TAB again on an unchanged
  line prints the candidates as width-adaptive columns next to the prompt
  and repaints it. The list is printed output, not a menu: nothing is
  selected, arrow keys stay on history, no popup.
- `ide` — multi-column popup with descriptions, VS Code style. Column count
  adapts to the terminal width; the description of the selected candidate
  renders beside it.
- `column` — plain multi-column grid (like zsh `compinit`).
- `list` — vertical list with descriptions (like fish).
- `inline` — first match inserted directly, Tab cycles (like bash
  `menu-complete`).

`show-all-if-ambiguous` / `show-all-if-unmodified`-style readline variables
are not implemented yet; the double-TAB listing is the fixed shape.

Flag completions (`ls --a<Tab>` → `--all` with its description) come from the
bundle's generated completion assets and work in every style; `ide` and
`list` show the descriptions inline.

## WinuxCmd Applet Completions

Every WinuxCmd applet ships with flag completion built into the `native-shell` binary:
`grep --col<Tab>` offers `--color` / `--colour` with descriptions,
`ls --color <Tab>` offers `always` / `auto` / `never`, and `wpm <Tab>`
lists the package-manager subcommands (`install`, `search`, `uninstall`,
...).

These definitions are generated from `winuxcmd --help` (WinuxCmd 1.1.5
transcripts) by `scripts/generate-winuxcmd-completions.py`, stored as one
TOML per applet under `crates/native-shell-runtime/assets/completions/winuxcmd/`,
and compiled into the runtime — so they work on a fresh install with no
plugin enabled. When WinuxCmd gains applets or options, regenerate against
the new binary:

```bash
python scripts/generate-winuxcmd-completions.py --winuxcmd <path-to-winuxcmd.exe>
python scripts/test-winuxcmd-completions.py   # golden check: corpus vs committed assets
```

A second compiled layer covers the common external commands that are not
WinuxCmd applets — git, tar, curl, ssh, docker, kubectl, and npm (issue
#305): static subcommand/flag tables under
`crates/native-shell-runtime/assets/completions/common/`, so `git co<Tab>` offers
`checkout`/`commit`/`config` and `kubectl get -<Tab>` offers `-o`/
`--all-namespaces` on a fresh install.

Priority is layered: the embedded applet and common-command defaults are the
base layer, and completion definitions loaded later override them per
command — a bundle/pack TOML or a user completion-dir TOML whose file stem
(or `command` key) matches the built-in name replaces the built-in
definition entirely, so trimming or extending a command's flags is a plain
file drop, not a fight with the defaults.

## AI Agent Skill Bundle

For AI hosts (Claude, ZCode, Cursor, ...) that drive your shell, native-shell
ships a skill bundle describing its capability surface and dialect
contract: `skills/native-shell/SKILL.md` plus `references/`. Install it without
hunting for the zip:

```bash
native-shell skill install                       # default: ~/.claude/skills/native-shell/
native-shell skill install --target all          # claude + zcode + cursor
native-shell skill install --target C:/agents/skills   # generic: <dir>/native-shell/
native-shell skill status                        # current / outdated per target (byte compare + sha256)
```

`native-shell doctor` reports an advisory `agent skill` row. The released
`native-shell-skill-v*.zip` contains the same tree (WinuxCmd-skill layout), so
unpacking the zip into an agent skills directory is equivalent.

The command tables inside the bundle are generated — hand-written lists
rot. `scripts/generate-skill.py` fills the GENERATED-marked regions of
`SKILL.md` and `references/quickref.md` from the engine's own surfaces:
a captured `help -s '*'` transcript (builtin table), a captured
`native-shell --help` transcript (launcher/plugin verbs), and the applet completion
inventory (180 winuxcmd applets). After changing the launcher help or the
builtin table, refresh and re-check:

```bash
cargo build && python scripts/generate-skill.py --capture --native-shell target/debug/native-shell.exe
python scripts/test-skill-bundle.py   # golden check: regions + embed manifest
```

## Prompt And Themes

Defaults are a floor, not an identity. The built-in prompt (a reedline
template: `user@host cwd symbol`) renders only while nothing claims `PS1`.
Whoever sets `PS1` last owns the prompt completely — your own rc line, an
enabled oh-my-bash theme, or starship:

```bash
PS1='\u@\h \w \$ '                # your own prompt: simplest possible claim
eval "$(starship init bash)"      # starship claims PS1 on its first prompt
```

Disabling the claim (`native-shell plugin disable oh-my-bash`, or a plain
`unset PS1`) lets the built-in floor render again. `PROMPT_COMMAND` alone
is not a claim — it is a pre-prompt hook; hooks that want the prompt set
`PS1` themselves.

The floor itself is shaped by `NATIVE_SHELL_PROMPT_CWD_STYLE` (`home`, `full`, or
`basename`; see [Getting Started](getting-started.md)). External themes
style themselves through the bash-compatible `PS1` channel — colors,
powerline glyphs, and git segments come from the theme, not the shell.

## History Modes

Set `NATIVE_SHELL_HISTORY_MODE` in `~/.native-shellrc` when multiple shells share a
history file:

```bash
NATIVE_SHELL_HISTORY_MODE=private
export NATIVE_SHELL_HISTORY_MODE
```

- `shared` (default) refreshes navigation from other shells.
- `session` keeps the startup snapshot stable for navigation while builtins can
  observe later file updates.
- `private` loads the complete history file at startup, then keeps later
  navigation changes local to the current shell while appending its own commands.

## Custom Key Widgets

A shell function can act as a line editor widget. Declare it in
`~/.native-shellrc` with `NATIVE_SHELL_BINDKEYS` (one `key:widget` entry per line):

```bash
native_shell_fzf_file() {
    local file
    file="$(fd -t f | fzf)" || return 0
    NATIVE_SHELL_WIDGET_RESULT="ni $file"
    NATIVE_SHELL_WIDGET_ACCEPT=1
}
NATIVE_SHELL_BINDKEYS="Ctrl+X:native_shell_fzf_file"
```

When the key fires, the function runs with the current editor state:

- In: `NATIVE_SHELL_WIDGET_BUFFER` (current buffer) and `NATIVE_SHELL_WIDGET_CURSOR` (byte
  offset). Both are temporary and restored afterwards.
- Out: `NATIVE_SHELL_WIDGET_RESULT` replaces the buffer (unset or absent keeps it;
  an empty string clears the line), `NATIVE_SHELL_WIDGET_CURSOR_RESULT` moves the
  cursor to a byte offset, and `NATIVE_SHELL_WIDGET_ACCEPT=1` submits the buffer as
  if Enter had been pressed.

Keys are single-key sequences such as `Ctrl+X`, `Alt+G`, a plain character,
or escape forms like `^X`. The function runs as ordinary shell code: aliases,
PATH, and your rc all apply. Unknown widget names in bundle bindkeys follow
the same contract, so plugins can ship function widgets too.

## Shell-Function Completions

A shell function can provide completions for a command's arguments. Declare
it in `~/.native-shellrc` with `NATIVE_SHELL_COMPDEFS` (one `command:function` entry per
line):

```bash
native_shell_git_comp() {
    local branches
    branches="$(git branch --format='%(refname:short)')" || return 0
    NATIVE_SHELL_COMP_RESULT="$branches"
}
NATIVE_SHELL_COMPDEFS="git:native_shell_git_comp"
```

When completing arguments for a registered command, the function runs with:

- In: `NATIVE_SHELL_COMP_WORDS` (space-joined words, matching bash `COMP_WORDS`) and
  `NATIVE_SHELL_COMP_CWORD` (bash `COMP_CWORD` semantics). Both are temporary.
- Out: `NATIVE_SHELL_COMP_RESULT`, one candidate per line, shaped `value` or
  `value<TAB>description`.

A `NATIVE_SHELL_COMPDEFS` line can also name a directory (issue #306): every line
that resolves to an existing directory is loaded as a completion-definition
directory — `<cmd>.toml` files in the same schema as the embedded applet
definitions, and `<cmd>.bash` standard bash completion scripts (read as
data, never executed). Remaining lines are `command:function` pairs:

```bash
NATIVE_SHELL_COMPDEFS="C:/agents/completions"
NATIVE_SHELL_COMPDEFS="$NATIVE_SHELL_COMPDEFS
git:native_shell_git_comp"
```

Compdef functions run synchronously during completion, while the line editor
owns the terminal. They must not print to stdout: compute with command
substitution and write `NATIVE_SHELL_COMP_RESULT` instead. Bundle keybindings TOML and
`NATIVE_SHELL_BINDKEYS` share the same widget vocabulary, so plugins and users can
compose the two contracts freely.

## Git Prompt Performance

Git status should be consumed as a coherent prompt snapshot, not rendered by
blocking every prompt draw with fresh Git processes. The intended shape is:

- prompt/theme plugins render the latest available snapshot;
- the host keeps git status work warm in the background;
- late git work updates the next prompt instead of repainting the active input
  line.

If the prompt flickers or repaints the current line, debug the lifecycle and
git snapshot path rather than adding more inline Git calls to the theme.

## Plugin Workflow

Use the CLI to inspect the ecosystem instead of relying on stale docs:

```sh
native-shell plugin list          # sources, their assets, activation state
native-shell plugin discover      # read-only overview, including not-yet-installed managers
```

Enablement is per source or per asset (a theme), and it edits your rc, not
an internal database:

```sh
native-shell plugin enable oh-my-bash   # guarded loader block appended to ~/.native-shellrc
native-shell plugin enable agnoster     # a theme asset -> sets OSH_THEME in that block
native-shell plugin disable oh-my-bash  # removes the block; the built-in floor returns
```

`native-shell plugin update|sync|restore|clean` are the lockfile-style maintenance
verbs for installed sources.

## Third-Party Sources

Beyond the curated catalog (`oh-my-bash`, `bash-it`, `bash-completion`),
native-shell can install any git repository or local path that follows a known
plugin-manager layout. New sources are **untrusted by default**: nothing
they ship runs until you explicitly trust them.

```sh
native-shell plugin add https://github.com/someone/oh-my-bash-fork.git
# review the cloned source under ~/.native-shell/sources/<id>
native-shell plugin trust oh-my-bash-fork
native-shell plugin enable oh-my-bash-fork
native-shell plugin source remove oh-my-bash-fork   # delete tree + registry entry
```

Notes:

- Sources are cloned to `~/.native-shell/sources/<id>` and registered in
  `~/.native-shell/sources/registry.toml`; the registry records the origin, ref,
  path, and trust state.
- `native-shell plugin update <id>` re-fetches the registered origin;
  `native-shell plugin rollback <id>` returns to the previous version.
- An untrusted source never activates: enable refuses until
  `native-shell plugin trust <id>` passes the review gate.
- A git ref can be pinned at add time with `native-shell plugin add <id> --ref <ref>`.
- `NATIVE_SHELL_PLUGIN_SOURCES_ROOT` overrides the sources root (portable setups and
  tests).

## Command Discovery And WPM

Native Shell resolves Unix-style commands through normal Windows `PATH`. When a
command is missing or comes from the wrong provider, inspect the active
installation:

```bash
command -v native-shell
command -v winuxcmd.exe
command -v ls
winuxcmd.exe wpm index status
winuxcmd.exe wpm search jq
winuxcmd.exe wpm links rebuild --force
```

Do not assume `/usr/bin` exists. Native Shell is a Windows process using Windows
executables and command links.

## Elevated Commands

Native Shell disables Rubash's experimental `sudo` builtin by default. Windows
elevation is delegated to the WPM `gsudo` package, which owns UAC, process
creation, environment forwarding, and console handling.

```bash
command -v gsudo
gsudo --version
gsudo your-command args
```

Install or repair it through the active WPM provider when necessary:

```bash
wpm search gsudo
wpm install gsudo
wpm links rebuild --force
```

Do not alias `sudo` automatically in shared scripts. A user who wants the
Unix spelling interactively can add this to `~/.native-shellrc`:

```bash
alias sudo='gsudo'
```

The embedded Rubash elevation builtin remains available only for host
integrators. Set `NATIVE_SHELL_ENABLE_RUBASH_SUDO=1` before starting Native Shell, or run
`enable sudo` in a shell that supports an elevation handler. This is not the
recommended Windows path.

## Windows Paths And Home

Prefer durable Windows paths in scripts:

```bash
cd C:/Users/you/repo
ls "C:\Program Files"
cd ~
```

Prompt display should normally render the home directory as `~` and descendants
as `~/path`, but internal process paths remain native Windows paths. Treat
`/c/Users/...` as compatibility input, not the primary model.

## Updating

Keep the three update planes separate:

```bash
native-shell --self-update --check
native-shell --self-update

winuxcmd.exe wpm update winuxcmd

native-shell plugin update oh-my-bash
native-shell plugin rollback oh-my-bash
```

- `native-shell --self-update` updates the shell.
- `wpm update winuxcmd` updates command packages and command links.
- `native-shell plugin update <source-id>` updates an installed plugin source;
  `rollback` returns it to the previous version.

## Debug Checklist

For shell issues, capture the active binary and execution path first:

```bash
native-shell --version
command -v native-shell
command -v winuxcmd.exe
echo "$SHELL"
native-shell -c 'echo command-mode:$SHELL'
native-shell -C 'echo repl-command:$SHELL'
```

For repository changes, run focused tests before broad suites:

```bash
cargo test --test repl_command --locked
cargo test -p native-shell-runtime --lib --locked
cargo test --test plugin_inventory --locked
```

Use [Plugin System Direction](../planning/plugin-system-direction.md) for architecture and
[Plugin System Roadmap](../planning/plugin-system-roadmap.md) for execution order.
