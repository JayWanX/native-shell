# Release runbook (operator)

The pipeline contract lives in [release.md](release.md) — this is the
operator's procedure for driving one release through it, with the iron
rules and the failure catalog earned by the v1.4.0 ignition campaign
(seven runs, 2026-10-13 → 10-14).

## Iron rules

1. **The tag is cut LAST — and it is one-shot.** A tag-push release
   checks out the tree the tag points at. Every product fix merged after
   the tag was cut is silently absent from the build — runs 1-5 of
   v1.4.0 all died on this one mistake in different costumes (missing
   tag, stale journey scripts, stale pins, missing self-check file).
   Sequence: merge everything → confirm `origin/master` SHA → cut the
   tag on that SHA → push. The `release-tags-are-immutable` ruleset
   (#425, docs/release-tag-ruleset.md) rejects moving or deleting a
   pushed `v*` tag for everyone except org admins, so the old
   delete-and-re-cut loop is gone: get the SHA right before pushing.
2. **Never move a tag while a run is in flight** — the ruleset now
   rejects it outright, and a run reading a moving tag produces a
   release that matches nothing anyway. To re-ignite a run against an
   unchanged tag, dispatch the workflow (iron rule 5); a tag that
   points at the wrong commit is corrected forward as the next patch
   version, not rewritten.
3. **The gate scripts ride `origin/master`, not the tag.** The journey
   job (`git checkout origin/master -- scripts/journey`) and the
   perfbudget job (`-- scripts/perf`) deliver their current scripts by
   git pathspec after checking out the tag. A gate-only fix therefore
   needs only a master merge — but if ANY product-tree file changed,
   re-cut the tag anyway (rule 1).
4. **A lightweight tag is fine** (the release body is hardcoded in the
   workflow template; nothing reads the annotation), but the tag NAME
   must match `^v\d+\.\d+\.\d+$` AND the `Cargo.toml` `version` at that
   commit.
5. **`workflow_dispatch` is not the release trigger, but it IS the
   re-fly path** — it needs no `tag` input (the version comes from
   `Cargo.toml`) and takes an optional `native_cmd_tag` (empty = the
   committed pin); it checks out an existing tag, and publishes through the same
   `release` job. Real releases go by tag-push; a failed run on a
   correct tag is re-flown by dispatching that tag, because re-pushing
   the tag is blocked by the ruleset (#425). A dispatch may also run
   **before** the tag exists: the version comes from `Cargo.toml`, the
   per-job `git checkout` then falls back to the checked-out ref, and
   `action-gh-release` creates the tag on `github.sha`.

## Pre-flight checklist (all boxes before cutting the tag)

- [ ] `origin/master` CI green (build-and-test, both repos if rubash
      moved — native-shell vendors rubash via `[patch]` + git dependency,
      and its lockfile must carry the matching rubash version; see the
      2026-10-10 lock incident: bump rubash version → one-line lock
      sync).
- [ ] Open issues at zero on the blocking repos (owner gate, 2026-10-12).
- [ ] Pins updated and mutually consistent:
      
      `scripts/release/preinstall.json` (package pins incl. native-git —
      a wpm-source index update without this pin = fail-open skip =
      verify throw, run 4).
- [ ] crates.io: the workspace versions bumped together, the
      `rubash`/`native-shell-runtime` requirement strings in dependents
      updated, and the release secret name verified against the repo
      settings (`CRATES_IO_TOKEN`, mapped in release.yml).

- [ ] Confirm which command-layer release the bundle will carry. Nothing is
      committed: the release follows `JayWanX/native-cmd`'s latest published
      release, so check that page first. Pass `native_cmd_tag` on a dispatch
      to pin a specific one.
- [ ] CHANGELOG folded for everything user-visible since the last tag.
- [ ] Local install smoke of the previous release still upgrades (the
      installer lane has its own R1-R7 risk list, docs/store/).

## Ignition sequence

```sh
# 0. everything merged, origin/master = <sha>, CI green
gh api repos/JayWanX/native-shell/commits/master --jq .sha   # note <sha>

# 1. cut the tag — one shot: the ruleset rejects moving a pushed v* tag
git ls-remote --tags origin vX.Y.Z   # must be EMPTY before you push
git tag vX.Y.Z <sha>
git push origin vX.Y.Z        # this push IS the trigger

#    re-fly a failed run on the same (correct) tag instead of re-pushing:
#    gh workflow run release.yml -R JayWanX/native-shell \
#      -f native_cmd_tag=vA.B.C

# 2. watch
gh run list --repo JayWanX/native-shell --workflow=release.yml --limit 1
```

Expected wall time ~45-60 min (perfbudget alone measured 2351 s on run
38489787748; the journey lane and the timing lane build concurrently).

## The gates and what their artifacts say

| Gate | Reads | Blocks on | Artifact |
| --- | --- | --- | --- |
| smoke (per build job) | the binary it just built | any of the 4 checks | job log |
| golden user journey | `scripts/journey` from origin/master | any unlabeled FAIL verdict (`test_verdict_exit.py` pins the contract: known-fail exits 0, unknown fail exits 1) | `journey-verdict` (verdict.json + raw transcripts) |
| per-asset timing | `scripts/perf` from origin/master | verdict `hard-fail`/`timeout` (past the hard cap); `over` inside the hard band is a printed warning, NOT a blocker (2026-10-14 calibration, #335/#337 — revert #337 to restore fail-on-over) | `perfbudget-verdict` (asset-timing.json) |

Journey failure triage: download `journey-verdict`, read
`verdict.json` → `steps[]` → `assertions[]` with `ok: false`. The raw
transcripts are the truth when a screen-shape assertion looks wrong —
the pyte viewport is a SAMPLE of a live ConPTY repaint, not an instant
(P8-S4's emptied signature, run 38489787748: the theme had rendered;
a partial repaint left the top row's letterless filler as the newest
row and the old single-instant extractor broke on it — #334/#336 made
the extractor sample and compare block shape).

## Failure catalog (v1.4.0 campaign — check here first)

| Symptom | Root cause | Fix |
| --- | --- | --- |
| checkout/journey fails on files that exist on master | tag cut before fixes merged (iron rule 1) | cut the NEXT patch tag on master — the pushed tag is immutable (#425) |
| `Invalid release tag` on dispatch | the `tag` input disagrees with `Cargo.toml` | leave it empty |
| preinstall verify throw, native-git missing | wpm index updated, `preinstall.json` pin not | sync the pin |
| crates publish "secret not found" | secret name mismatch | map `CRATES_IO_TOKEN` |
| crates "already uploaded" on retry | expected — tolerated by pattern in release.yml | no-op |
| armv7 "linker not found" | NDK r19+ name is `armv7a-linux-androideabi*-clang` | #327, keep |
| journey `SyntaxWarning` / script missing | tag-tree vs master delivery drift | #330/#333 pathspec delivery, keep |
| P8-S4 signature emptied | harness sampling artifact, not product | #336 sampling extractor, keep |
| perfbudget FAIL with all overs ≤1.05x | soft band = concurrent-lane load | #337: soft over warns |

## After green (the downstream chain, in order)

1. `wpm-source` index PR: native-shell vX.Y.Z entry (the release URL +
   SHA-256 from `SHA256SUMS`).
2. winget: upgrade the open manifest PR to vX.Y.Z in place
   (InstallerSha256 straight from `SHA256SUMS`; winget convention wants
   the latest version on the same PR).
3. Store material (docs/store/) — self-update must be compiled out for
   a store build (MS Store certification rejects it).
4. UX fleet (five lanes: first-contact / Git-Bash migrant / visual /
   docs / feel) — run against the real installed release.
5. Fold this run's lessons back into this runbook.
