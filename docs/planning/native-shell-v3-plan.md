# Native Shell v3 Plan

Native Shell v3 is about making the shell feel complete as a Windows-native Bash
environment while keeping extensions maintainable.

## Architecture

```text
native-shell = rubash + winuxcmd + reedline + oh-my-native-shell
```

- `rubash` owns shell language semantics.
- `winuxcmd` owns bundled Unix-style commands.
- `reedline` owns interactive editing primitives.
- Native Shell owns Windows host integration, prompt rendering, plugin loading, and
  configuration.
- `oh-my-native-shell` owns first-party plugin assets and reviewed `.winux` helper
  scripts.

## Configuration

- `~/.native-shellrc` is the primary interactive startup file.
- `~/.winshrc` remains an old fallback only when `~/.native-shellrc` does not exist.
- Plugin CLI operations, bundle versions, tests, and advanced overrides use
  the manifest-backed registry.

## Plugin Model

- Load named packs from `oh-my-native-shell`.
- Keep pack manifests auditable.
- Use asset packs for aliases, completions, themes, and keybinding metadata.
- Use `.winux` source packs for reviewed shell mutation.
- Use process adapters for external tools.
- Keep prompt layout owned by the selected theme; segment providers only supply
  data.

## Non-Goals

- No second shell language runtime.
- No arbitrary rc-file sourcing as plugin code.
- No plugin access to parser/executor internals.
- No long-term host-side workaround for rubash language bugs.
