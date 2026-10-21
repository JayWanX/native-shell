//! Release pre-install manifest contract (native-shell#189).
//!
//! `scripts/release/preinstall.json` is release-CI policy, not tribal
//! memory: the workflow installs `packages` via `winuxcmd wpm install`,
//! materializes `shims` as native-cmd.exe hardlinks, and must never ship a
//! name recorded in `excluded`. Since native-shell#312 every package entry also
//! pins its exact artifact SHA-256 per arch — the release refuses to
//! install anything whose live-index digest differs from the pin, so a
//! manifest edit cannot silently change what a release ships. These tests
//! pin the schema and the owner-ordered exclusions so a manifest edit
//! cannot silently re-admit a forbidden package (compression tools, goawk,
//! `link`), drop the per-leg install-policy disclosure (fail-closed on
//! x64, fail-open on other legs), or drop the pins. The workflow
//! reads the same file; a schema break fails `cargo test` here before it

use serde_json::Value;
use std::path::PathBuf;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/release/preinstall.json")
}

fn load_manifest() -> Value {
    let raw = std::fs::read_to_string(manifest_path())
        .expect("preinstall manifest must exist at scripts/release/preinstall.json");
    serde_json::from_str(&raw).expect("preinstall manifest must be valid JSON")
}

fn string_list(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Every package/shim/exclusion entry is a lowercase wpm package name
/// (`^[a-z0-9][a-z0-9+-]*$` — the same shape the wpm index uses).
fn assert_name_shape(name: &str, context: &str) {
    assert!(
        !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '+' || c == '-'),
        "{context}: name {name:?} is not a valid wpm package name"
    );
}

/// native-shell#312: a pinned artifact digest is exactly 64 lowercase hex digits.
fn assert_sha256_pin(hash: &str, context: &str) {
    assert!(
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()),
        "{context}: sha256 pin {hash:?} is not a 64-hex-digit digest"
    );
    assert!(
        hash.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
        "{context}: sha256 pin {hash:?} must be lowercase hex (Get-FileHash output is normalized at pin time)"
    );
}

/// native-shell#312: a wpm package entry pins `version` and a per-arch
/// `sha256` map (keys `windows-x64` / `windows-arm64`). The release workflow
/// refuses to install a package without a pin for its arch, so a missing pin
/// here silently degrades the release — it must be a loud schema break instead.
///
/// A `fork-release` package is the exception: it never reaches wpm, so it has
/// no index digest to pin and is fetched from the fork's own release instead.
fn assert_package_pin(package: &Value) {
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .expect("package entries always carry a name when pins are checked");
    assert!(
        package
            .get("version")
            .and_then(Value::as_str)
            .is_some_and(|version| !version.trim().is_empty()),
        "package {name:?} needs a non-empty version next to its sha256 pin"
    );
    if package.get("source").and_then(Value::as_str) == Some("fork-release") {
        // Bypasses wpm: no index entry and no stored digest (the fork is rebuilt
        // by us, so a committed sha256 would go stale on every rebuild). It must
        // instead declare where the bytes come from.
        for key in ["repo", "asset_pattern"] {
            assert!(
                package
                    .get(key)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty()),
                "fork-release package {name:?} needs a non-empty {key:?}"
            );
        }
        assert!(
            package
                .get("files")
                .and_then(Value::as_array)
                .is_some_and(|files| !files.is_empty()),
            "fork-release package {name:?} needs a non-empty files mapping"
        );
        assert!(
            package.get("sha256").is_none(),
            "fork-release package {name:?} must NOT carry a sha256 pin"
        );
        return;
    }
    let pins = package
        .get("sha256")
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            panic!("package {name:?} needs a sha256 object keyed by arch (native-shell#312)")
        });
    assert!(
        !pins.is_empty(),
        "package {name:?} has an empty sha256 pin map; pin at least one arch"
    );
    for (arch, hash) in pins {
        assert!(
            arch == "windows-x64" || arch == "windows-arm64",
            "package {name:?}: pin arch {arch:?} is not a wpm arch key"
        );
        let hash = hash
            .as_str()
            .unwrap_or_else(|| panic!("package {name:?}: pin for {arch:?} must be a string"));
        assert_sha256_pin(hash, &format!("package {name:?} pin {arch:?}"));
    }
}

#[test]
fn manifest_schema_is_complete() {
    let manifest = load_manifest();

    assert_eq!(
        manifest.get("schema").and_then(Value::as_u64),
        Some(1),
        "schema version must be 1; bump requires updating this test file too"
    );
    for key in ["description", "packages", "shims", "excluded", "notes"] {
        assert!(
            manifest.get(key).is_some(),
            "manifest is missing required key {key:?}"
        );
    }

    // packages: non-empty, each with a name, purpose, and clean shape.
    let packages = manifest
        .get("packages")
        .and_then(Value::as_array)
        .expect("packages must be an array");
    assert!(
        !packages.is_empty(),
        "a release pre-install manifest with zero packages is pointless; delete the step instead"
    );
    let mut names = Vec::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .expect("every package entry needs a string name");
        assert_name_shape(name, "packages");
        assert!(
            package
                .get("purpose")
                .and_then(Value::as_str)
                .is_some_and(|purpose| !purpose.trim().is_empty()),
            "package {name:?} needs a non-empty purpose (why it ships)"
        );
        assert_package_pin(package);
        names.push(name.to_string());
    }
    let unique = names
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unique.len(),
        names.len(),
        "duplicate package entries: {names:?}"
    );
}

/// native-shell#209 acceptance, pinned in CI policy: the must-install bundle is
/// gawk (native-shell#189) plus native-git / ripgrep / fd. Dropping any of them from
/// the manifest silently re-breaks `native-shell setup` full distro on a gitless
/// machine, so their presence is a contract, not a preference.
#[test]
fn must_install_bundle_is_present() {
    let manifest = load_manifest();
    let package_names = string_list_of(&manifest, "packages");
    for required in ["gawk", "native-git", "ripgrep", "fd"] {
        assert!(
            package_names.iter().any(|name| name == required),
            "must-install package {required:?} is missing from the manifest (native-shell#189/#209)"
        );
    }
}

#[test]
fn shims_target_installed_packages_and_do_not_collide() {
    let manifest = load_manifest();
    let package_names: Vec<String> = string_list_of(&manifest, "packages");
    let shims = manifest
        .get("shims")
        .and_then(Value::as_array)
        .expect("shims must be an array");

    for shim in shims {
        let name = shim
            .get("name")
            .and_then(Value::as_str)
            .expect("every shim entry needs a string name");
        assert_name_shape(name, "shims");
        let target = shim
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(
            package_names.contains(&target),
            "shim {name:?} targets {target:?}, which is not in packages"
        );
        assert_ne!(
            name, target,
            "shim {name:?} would shadow its own target {target:?}"
        );
        assert!(
            !package_names.contains(&name.to_string()),
            "shim {name:?} collides with a package of the same name"
        );
        assert!(
            shim.get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| !reason.trim().is_empty()),
            "shim {name:?} needs a non-empty reason (why the extra hardlink exists)"
        );
    }
}

#[test]
fn excluded_records_owner_policy_and_never_reappears() {
    let manifest = load_manifest();
    let package_names = string_list_of(&manifest, "packages");
    let shim_names = string_list_of(&manifest, "shims");

    let exclusions = manifest
        .get("excluded")
        .and_then(Value::as_array)
        .expect("excluded must be an array");
    let mut excluded_names = Vec::new();
    for entry in exclusions {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .expect("every excluded entry needs a string name");
        assert_name_shape(name, "excluded");
        assert!(
            entry
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| !reason.trim().is_empty()),
            "excluded {name:?} needs a non-empty reason (the exclusion IS the policy)"
        );
        excluded_names.push(name.to_string());
    }

    for name in &package_names {
        assert!(
            !excluded_names.contains(name),
            "package {name:?} is simultaneously excluded and installed"
        );
    }
    for name in &shim_names {
        assert!(
            !excluded_names.contains(name),
            "shim {name:?} is simultaneously excluded and materialized"
        );
    }

    // Owner-ordered exclusions must stay recorded even if the package list
    // never changes: compression tools (users wpm-install them), goawk
    // (gawk only), and `link` (MSVC link.exe collision — forbidden forever).
    for forbidden in ["bzip2", "gzip", "goawk", "link"] {
        assert!(
            excluded_names
                .iter()
                .any(|entry| entry.as_str() == forbidden),
            "owner exclusion {forbidden:?} vanished from the manifest"
        );
        assert!(
            !package_names.contains(&forbidden.to_string()),
            "forbidden package {forbidden:?} re-entered packages"
        );
        assert!(
            !shim_names.contains(&forbidden.to_string()),
            "forbidden name {forbidden:?} re-entered shims"
        );
    }
    // link is forbidden forever — its reason must say why, so a future
    // editor cannot mistake it for a stylistic exclusion.
    let link_reason = exclusions
        .iter()
        .find_map(|entry| {
            let name = entry.get("name").and_then(Value::as_str)?;
            (name == "link")
                .then(|| entry.get("reason").and_then(Value::as_str))
                .flatten()
        })
        .expect("link exclusion must exist");
    assert!(
        link_reason.to_ascii_lowercase().contains("msvc"),
        "the link exclusion reason must name the MSVC link.exe collision"
    );
}

#[test]
fn fail_closed_x64_contract_is_part_of_the_manifest() {
    // The per-leg install policy is a property of the manifest, not of the
    // workflow (native-shell#468): x64 is the primary user-facing leg, so a
    // failed pin check, install, or shim materialization there must stop
    // the release — the notes must keep telling release editors that. The
    // non-primary legs keep the fail-open disclosure contract.
    let manifest = load_manifest();
    let notes = string_list(&manifest.get("notes").cloned().unwrap_or(Value::Null));
    assert!(
        notes.iter().any(|note| {
            let note = note.to_ascii_lowercase();
            note.contains("fail-closed") && note.contains("x64")
        }),
        "manifest notes must record the x64 fail-closed contract (native-shell#468)"
    );
    assert!(
        notes
            .iter()
            .any(|note| note.to_ascii_lowercase().contains("fail-open")),
        "manifest notes must record the non-x64 fail-open contract"
    );
    assert!(
        manifest
            .get("notes")
            .and_then(Value::as_array)
            .is_some_and(|notes| !notes.is_empty()),
        "manifest notes must not be empty"
    );
}

#[test]
fn pin_contract_is_recorded_in_the_notes() {
    // native-shell#312: the sha256 pins are only as durable as their story. The
    // notes must say where the pins came from so the next bump is a
    // deliberate, re-recorded change instead of a mystery constant.
    let manifest = load_manifest();
    let notes = string_list(&manifest.get("notes").cloned().unwrap_or(Value::Null));
    assert!(
        notes
            .iter()
            .any(|note| note.contains("native-shell#312")
                && note.to_ascii_lowercase().contains("pin")),
        "manifest notes must record the native-shell#312 pin contract"
    );
}

fn string_list_of(manifest: &Value, key: &str) -> Vec<String> {
    manifest
        .get(key)
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.get("name").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
