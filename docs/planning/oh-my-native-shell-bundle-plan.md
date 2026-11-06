---
tags: [native-shell, plugins, oh-my-native-shell, bundle, roadmap]
created: 2026-07-30
status: draft
---

# Oh My Native Shell Bundle Plan

`oh-my-native-shell` is the official Native Shell plugin bundle.

The synchronized execution plan is in
[Plugin System Roadmap](plugin-system-roadmap.md).

The goal is to make first-party Native Shell plugins feel bundled, versioned,
updatable, and reversible, while keeping the core shell architecture stable:
rubash owns shell semantics, reedline owns interactive editing, and native-cmd is
integrated through PATH.

## Target State

```text
native-shell release
  native-shell.exe
  cmd/
  bundles/
    oh-my-native-shell/
      bundle.toml
      packs/
```

Users should be able to install a release zip and immediately have a local
baseline bundle. Network access is only needed for later bundle updates.

## Repository Reset Policy

The existing `unixwin/oh-my-winuxsh` repository is legacy content. Do not
delete repository history. Instead:

1. clone the repository;
2. tag the current state as `legacy-pre-native-shell-plugin-system`;
3. create a `legacy` branch from the old state;
4. rebuild `main` around the new official bundle layout;
5. document the old repository content as legacy.

This preserves trust for open-source users while making the product direction
clean.

## Bundle Layout

```text
oh-my-native-shell/
  README.md
  bundle.toml
  index.toml
  packs/
    git/plugin.toml
    docker/plugin.toml
    kubectl/plugin.toml
    npm/plugin.toml
    zoxide/plugin.toml
    direnv/plugin.toml
    dotenv/plugin.toml
    fzf/plugin.toml
    command-not-found/plugin.toml
    last-working-dir/plugin.toml
    thefuck/plugin.toml
    process-echo/plugin.toml
    keybindings/plugin.toml
    prompts/plugin.toml
  aliases/
    git.toml
    docker.toml
    kubectl.toml
    npm.toml
  completions/
    git.toml
    docker.toml
    kubectl.toml
    npm.toml
  prompts/
    segments.toml
  keybindings/
    common.toml
    emacs.toml
    vi.toml
  docs/
    design.md
    migration.md
```

## Bundle Metadata

```toml
name = "oh-my-native-shell"
version = "1.0.0"
publisher = "unixwin"
api = "native-shell:plugin-bundle@0.1.0"
min_native-shell = "0.8.3"
channel = "stable"

[update]
source = "github-release"
repo = "unixwin/oh-my-winuxsh"
asset = "oh-my-native-shell-{version}.zip"

[packs]
default = ["git", "prompts", "keybindings"]
available = [
  "git",
  "docker",
  "kubectl",
  "npm",
  "zoxide",
  "direnv",
  "dotenv",
  "fzf",
  "command-not-found",
  "last-working-dir",
  "thefuck",
  "prompts",
  "keybindings",
]
```

## Pack Categories

| Category | Packs | Default |
| --- | --- | --- |
| Daily dev | `git`, `docker`, `kubectl`, `npm` | `git` only |
| Navigation | `zoxide`, `fzf`, `last-working-dir` | Off |
| Environment | `direnv`, `dotenv` | Off |
| UX presets | `prompts`, `keybindings` | On if safe |
| Hints/workflow | `command-not-found`, `thefuck` | Off |

Packs that read project files, mutate environment, change cwd, or execute
external commands must stay explicit opt-in.

## Local Install State

```text
%LOCALAPPDATA%/Native Shell/bundles/oh-my-native-shell/<version>/
%LOCALAPPDATA%/Native Shell/bundles/oh-my-native-shell/current
<native-shell install dir>/bundles/oh-my-native-shell
~/.native-shell/plugin-lock.toml
```

The app-bundled path is the offline baseline that ships with Native Shell installers
and portable zips. User-managed bundle installs and the lock file take priority
so `native-shell plugin update oh-my-native-shell ...` can move independently of app
updates.

`plugin-lock.toml` should record:

- bundle name;
- version;
- source URL or release ID;
- checksum;
- installed path;
- active path;
- previous version for rollback.

Release publication should also include `CHANGELOG.md` and
`docs/compatibility.md` from the bundle repository so API, protocol, minimum
host, and semver policy are visible next to each zip.

## CLI Surface

Initial commands:

```sh
native-shell plugin list
native-shell plugin info git
native-shell plugin search devtools
native-shell plugin themes
native-shell plugin plan enable git
native-shell plugin install git
native-shell plugin enable git
native-shell plugin disable zoxide
native-shell plugin update oh-my-native-shell --from dist\oh-my-native-shell-1.0.0.zip --checksum-file dist\oh-my-native-shell-1.0.0.zip.sha256
native-shell plugin update oh-my-native-shell --github-release latest
native-shell plugin update oh-my-native-shell --github-release v1.0.0 --json
native-shell plugin rollback oh-my-native-shell
```

The plan/apply behavior should mirror safe managed-config updates:
preview first, write only managed TOML blocks, create backups, and keep rollback
instructions explicit.

## Config Boundary

The manifest-backed registry records managed plugin CLI state:

```toml
[plugins]
enabled = true
bundles = ["oh-my-native-shell"]
load = ["git", "prompts", "keybindings"]

[plugins.git]
enabled = true
permissions = ["cwd:read", "process:run:git"]
```

Normal user-authored plugin/theme selection and shell code belongs in
`~/.native-shellrc`:

```sh
alias ll='ls -la'
export EDITOR=vim
```

The legacy TOML rc path is removed. RC is the human entry point, while the
plugin system still needs deterministic, auditable, machine-editable manifest
state for CLI-managed permissions, bundle versions, tests, and rollback.

## Migration From Current Native Shell

1. Keep current behavior working.
2. Add plugin registry entries for existing built-in packs.
3. Add `[plugins]` config as the managed machine-editable surface.
4. Update docs and CLI help to say "official Native Shell plugins".
5. Add `oh-my-native-shell` bundle update support.

## Repository Status

The remote repository has been cloned locally at:

```text
C:\Users\caomengxuan\repo\oh-my-native-shell
```

Current branch:

```text
codex/rebuild-official-plugin-bundle
```

Legacy state has been preserved locally as `legacy-pre-native-shell-plugin-system`.
The active branch is being rebuilt as the official Native Shell plugin bundle.
