//! Static guard over the release-archive surface (native-shell#470).
//!
//! The v1.4.2 x64 zip was 57.1 MB and 61% of it was duplicated payload: the
//! staged tree's hardlinks (wpm's own install layout — the whole `git.exe`
//! family is one binary, `awk`/`gawk` likewise) are flattened by the
//! Copy-Item staging sweep, and Compress-Archive then stored every copy in
//! full. The Windows payload therefore ships as a `.tar.gz`, the only
//! archive format measured to record AND restore NTFS hardlinks (bsdtar:
//! 54.47 MB -> 29.63 MB on the v1.4.2 tree). The classic zip and the `.7z`
//! are still produced inside the job as measured baselines — the release
//! body compares their sizes — but neither is published.
//!
//! A placeholder-and-relink design (zero-byte stand-ins plus a manifest plus
//! a rebuild script) was REJECTED by the owner: manual unzip breaks, wpm
//! upgrade integration is a landmine, antivirus flags zero-byte executables,
//! and the symlink fallback breaks on FAT32/non-admin. These tests pin the
//! chosen design AND the absence of the rejected one, so neither can drift
//! silently.
//!
//! Like `installer_surface.rs`, this is a source-level review leg: the
//! runtime round-trip proof lives in the release workflow's
//! "Verify hardlink-native archives" step, which these pins reference.

use std::fs;
use std::path::PathBuf;

fn repo_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel} must exist next to the crate: {e}"))
}

fn packager() -> String {
    repo_file("scripts/package-release.ps1")
}

fn release_workflow() -> String {
    repo_file(".github/workflows/release.yml")
}

#[test]
fn packager_relinks_identical_staged_files_before_compression() {
    let text = packager();
    assert!(
        text.contains("function Complete-StagedHardlinks"),
        "package-release.ps1 must define Complete-StagedHardlinks (the staged-tree \
         hardlink restoration); without it every archive stores duplicated payload \
         in full (native-shell#470)"
    );
    // The alignment must happen BEFORE the zip is written, and the
    // hardlink-native archives must be produced by the same script run.
    let align = text
        .find("Complete-StagedHardlinks -StageDir $stageDir")
        .expect("the packager must invoke Complete-StagedHardlinks on the staged tree");
    let compress = text
        .find("Compress-Archive -LiteralPath $stageDir")
        .expect("package-release.ps1 must still produce the classic zip");
    let targz = text
        .find("tar -czf $tarGzPath -C $distDir $packageName")
        .expect("package-release.ps1 must produce the hardlink-native .tar.gz (bsdtar)");
    let seven = text
        .find("-snh -mx=9")
        .expect("package-release.ps1 must produce the .7z companion");
    assert!(
        align < compress,
        "hardlink alignment must precede Compress-Archive"
    );
    assert!(
        align < targz,
        "hardlink alignment must precede the tar.gz pack"
    );
    assert!(align < seven, "hardlink alignment must precede the 7z pack");
}

#[test]
fn packager_hardlink_alignment_is_content_neutral_and_loud_when_degraded() {
    let text = packager();
    // Only SHA256-equal files may be linked — this is the whole safety story.
    assert!(
        text.contains("Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256"),
        "the alignment must group by SHA-256; linking non-identical files would \
         corrupt the package"
    );
    // A volume without hardlink support degrades to copies with a warning,
    // never a silent divergence — and only when the caller opted in: the
    // default path THROWS, so a flattened package can never ship by accident.
    assert!(
        text.contains("keeping independent copies"),
        "the alignment must warn loudly and keep copies when hardlinks are \
         unavailable (FAT32/network staging volumes)"
    );
    assert!(
        text.contains("pass -AllowUnlinked to ship independent copies"),
        "unlinked staging must fail closed by default: -AllowUnlinked is the \
         only way to ship independent copies"
    );
}

#[test]
fn packager_keeps_the_zip_escape_hatch() {
    let text = packager();
    assert!(
        text.contains("[switch]$SkipHardlinkArchives"),
        "-SkipHardlinkArchives must stay available for callers that want the \
         legacy zip-only output shape"
    );
}

#[test]
fn workflow_verifies_archive_round_trip_before_upload() {
    let text = release_workflow();
    assert!(
        text.contains("Verify hardlink-native archives"),
        "release.yml must verify the hardlink-native archives after packaging; \
         an archive that unpacks to anything other than the staged bytes must \
         fail the release"
    );
    // Byte-identity for BOTH companions...
    assert!(
        text.contains(".tar.gz round-trip is not byte-identical"),
        "the workflow must hard-fail when the .tar.gz round-trip diverges from \
         the staged tree"
    );
    assert!(
        text.contains(".7z round-trip is not byte-identical"),
        "the workflow must hard-fail when the .7z round-trip diverges from the \
         staged tree"
    );
    // ...and actual hardlink restoration for the tar.gz (the format that
    // measured as hardlink-faithful; 7-Zip is exempt by documented limit).
    assert!(
        text.contains(".tar.gz did not restore hardlinks"),
        "the workflow must assert link counts on the .tar.gz extraction; content \
         identity alone cannot see a flattened link family"
    );
    assert!(
        text.contains("Get-HardLinkCount"),
        "the workflow needs the fsutil-based link counter for the restoration probe"
    );
}

#[test]
fn workflow_publishes_exactly_one_windows_payload_format() {
    let text = release_workflow();
    // The published payload: the hardlink-native tarball. A second portable
    // format would be a second 30-60 MB download of the same tree.
    let tar =
        "dist/native-shell-v${{ needs.resolve-tag.outputs.version }}-win-${{ matrix.arch }}.tar.gz";
    assert!(
        text.contains(tar),
        "release.yml must upload the hardlink-native Windows tarball: {tar}"
    );
    assert!(
        !text.contains("artifacts/**/*.7z"),
        "the .7z is a measured baseline inside the job, not a published asset \
         (it stores the payload once but cannot restore the hardlinks)"
    );
    assert!(
        !text.contains("dist/native-shell-win-${{ matrix.arch }}.tar.gz"),
        "the version-less 'latest-compatible' asset aliases are gone: \
         --self-update falls through to the versioned installer plus its \
         .sha256 sidecar, so a second copy of every asset buys nothing"
    );
    assert!(
        text.contains("-o -name '*.tar.gz'"),
        "SHA256SUMS must cover the shipped tarballs (native-shell#311: every \
         shipped artifact is hashed)"
    );
}

#[test]
fn release_body_sells_the_tar_gz_and_discloses_the_7z_limitation() {
    let text = release_workflow();
    assert!(
        text.contains("`*.tar.gz` is the only portable archive shipped"),
        "the release body must name the tarball as the ONE portable Windows \
         format and say why it is hardlink-native (download size + on-disk \
         footprint) — native-shell#470"
    );
    assert!(
        text.contains("degrades gracefully to independent byte-identical copies"),
        "the release body must document the harmless degradation on filesystems \
         without hardlink support (FAT32/exFAT)"
    );
    assert!(
        !text.contains("prefer `*.tar.gz` — it is hardlink-native"),
        "the release body must not offer a CHOICE of portable formats any more: \
         the zip and the .7z are neither published nor attached"
    );
}

#[test]
fn rejected_placeholder_design_must_not_return() {
    let packager_text = packager();
    let workflow_text = release_workflow();
    for (what, text) in [
        ("scripts/package-release.ps1", packager_text.as_str()),
        (".github/workflows/release.yml", workflow_text.as_str()),
    ] {
        for needle in ["relink-manifest", "placeholder"] {
            assert!(
                !text.contains(needle),
                "{what} mentions '{needle}': the zero-byte-placeholder + manifest \
                 + relink-script design was REJECTED for the release archives \
                 (manual unzip breaks, wpm upgrade integration is a landmine, \
                 antivirus flags zero-byte executables, symlink fallback breaks \
                 on FAT32/non-admin). The shipped design keeps every staged file \
                 a complete real file — hardlinks only share storage."
            );
        }
    }
}
