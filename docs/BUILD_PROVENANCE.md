# Captured portable builds — Task 8

Use `just portable-release` to build a Linux x86_64 candidate from a captured
source snapshot into a fresh directory. It builds rather than accepting arbitrary
prebuilt binaries. This produces an unsigned local execution record; it does not
authenticate a publisher or establish byte-reproducible or hermetic builds.

## Prepare and build

Install Rust with the wasm32-unknown-unknown target, matching wasm-bindgen CLI,
Python 3.11+, Linux development headers and Rust standard-library copyright
documentation. Cache the workspace's locked host/WASM dependencies first. The
builder deliberately uses `--offline --locked` and fails if inputs are missing.
It performs no dependency installation or download. See PORTABLE_EDITOR.md for
the existing packaging/notices requirements.

```sh
just portable-release target/my-release-candidate
# Equivalent:
python3 tools/build_editor.py --profile release --output target/my-release-candidate --jobs 2
```

The output directory must not exist. Failed builds retain logs there; choose a
new directory for a retry. Native and WASM Cargo outputs go into its previously
empty target directory. The default release profile in the captured Cargo.toml
is used without silently inheriting profile/target/compiler-wrapper/rustflags
environment overrides. `--profile dev` is available for build-driver diagnostics.
Normal Cargo home, proxy/CA and platform SDK discovery remain available.

## Inspect the result

- `source/` and `source.tar.gz` contain tracked and nonignored untracked files
  including required Cargo.lock (also captured from older ignored snapshots). Missing tracked files, symlinks and
  special/oversized files fail capture. Executable modes are preserved. Git HEAD
  and dirty status describe the original checkout; the source inventory and
  archive hashes identify the actual inputs, including unfinished local edits.
- `build-record.json` records source paths/hashes/modes, lockfile hash, compiler/
  Cargo/wasm-bindgen versions, builder hash, selected SDK/build settings, Cargo
  configuration hashes, command/log hashes and all three native plus three
  prepared browser payload hashes. It contains no full environment dump or Cargo
  configuration contents. Logs and source may contain project content; review
  them before sharing. Configuration and system/toolchain dependencies are not
  fully captured or made hermetic by this record.
- `kiln-portable.tar.gz` includes the checked record. The packager requires exact
  source inventory, lockfile, artifact hashes, compiler version and compiled build
  profile agreement. Its normal tool/runtime/inventory verification still runs.
  A modified or incomplete payload cannot be assigned this record unnoticed.
- `result.json` is written only after build and package verification succeed. It
  records final archive/source/record hashes. Ordinary installation verification
  hashes the included record just like any other payload file. It does not trust
  its unsigned claims as authentication.

The original prebuilt `just portable-editor` workflow remains available. Its
manifest labels provenance `preparation-observations-only`; a checked local build
record labels it `unsigned-local-build`. Neither label is a signature. Both
bundles still require the functional clean-runtime checks.

Current packaging includes the owner-selected MIT OR Apache-2.0 project grants
and requires declared licenses for path packages. A historical frozen source
snapshot lacking those files/declarations cannot be repackaged with current
requirements unchanged. Use its matching historical builder to reproduce its
original payload, or capture the updated licensed source for a new candidate;
do not label old archives as containing the new grant files.

## Validate the candidate

```sh
python3 -m unittest discover -s tests/native -p 'test_*.py'
python3 tools/test_clean_linux.py target/my-release-candidate/kiln-portable.tar.gz
```

Also exercise material/physics/browser behavior using the resulting runtime.
The clean Linux runner includes native/player/editor material pixel acceptance
using the installed tools and shipped material playground.
Clean Ubuntu/software graphics is a functional userspace check, not physical
Intel HD 520 performance, audible hardware/controller, other OS/browser or mobile
acceptance. Startup/frame-time/memory must be measured on reference hardware.

For independent rebuilding, extract source.tar.gz, preserve Cargo.lock, use the
recorded compiler/bindgen and commands with new output paths, and compare payload
hashes. A successful first build does not prove two builds match. Trusted CI
attestation, dependency/SDK/toolchain provenance, repeated binary comparison,
licensing review and installer signing remain release work. The hosted run below
adds GitHub workflow-origin attestation for its specific candidate; it does not
authenticate arbitrary locally prepared builds. Archive size is
informational; no size ceiling applies.

## Replay and compare captured payloads

```sh
just portable-replay target/my-release-candidate target/my-second-build
just compare-builds target/my-release-candidate target/my-second-build target/my-comparison.json
```

Replay checks the completed prior record, source archive/content/modes and lockfile,
copies those frozen inputs to a new source directory, then runs fixed offline/locked
build commands with fresh outputs. It requires the recorded profile, host, compiler/
Cargo/bindgen versions, selected SDK settings and observed Cargo configuration to
match. It never executes command strings from a record. The replaying driver can
be newer than the engine snapshot: its own hash/copy is retained independently as
build-driver.py beside its logs, and rebuild_of identifies the prior record.

The comparison command rehashes actual source, lockfile, driver, command diagnostics,
completed record/archive receipts and all six native/browser payload files. It
compares source/toolchain/settings and command inputs after normalizing the build
directory arguments. Missing/changed files, different input contracts or unequal
payloads produce passed=false and a failing process exit. Existing reports are
never replaced. A saved hash alone is not treated as proof that a file still matches.

Only all-six-payload equality with compatible inputs passes. Archive equality is
reported separately: per-execution local records/log hashes can make archives
different even when their executables, JS/WASM and runtime descriptor match. This
is same-host/toolchain payload repetition, not independent-builder authentication,
general cross-directory/platform reproducibility or a hermetic dependency audit.
Keep physical/platform/performance and trusted signing gates separate.

Executed on 2026-11-11: two fresh source/target directories on the same host
produced identical editor, CLI, player, JS, WASM and runtime descriptor bytes.
Archive equality was separately false because their execution records differ.
See [the comparison evidence](evidence/task8-repeated-build/README.md) for exact
inputs/hashes and scope. This closes the local repeated-payload check for that
snapshot while independent trusted builders and production platform gates remain.

## Hosted release-candidate workflow

The root Cargo.lock is tracked with the dependency versions/checksums used by the
local release evidence. Nested standalone workspace lockfiles retain their
existing ignore behavior. The release-candidate workflow runs on selected build/
test changes pushed to master, main and feat/2d-production-workflow, and supports
manual dispatch. Its tested Rust/nextest versions and official Actions commits
are pinned; bindgen is selected from Cargo.lock. Official actions use their native
Node 24 runtime. A five-minute artifact-service preflight first uploads/downloads
this run/attempt/source identity and compares the actual bytes. A service failure
stops engine builds early; the one-day probe does not certify later publication.
Cargo's cached tool executables include their installation manifests.

The read-only build job runs quality/tests, two fresh captured builds, actual
payload comparison, browser material/physics, moved installation/export/gameplay
and clean Ubuntu acceptance. Successful output is a GitHub Actions candidate
artifact; diagnostic evidence uploads independently on success or failure.
Only afterward does a separate job obtain OIDC/attestation permission, download
fixed candidate files and invoke the official build-provenance action. It does
not check out or execute engine code. A failed build/acceptance job cannot run
the attestation job. This workflow does not create a GitHub Release.

Use a current, authenticated GitHub CLI that supports `gh attestation verify`.
Select a completed successful **Release candidate verification** run in this
repository. Its source SHA is the candidate's revision; a later documentation-only
branch commit need not have a candidate of its own. Download into a new directory
and verify all five files before extracting or running the editor:

```bash
set -euo pipefail
candidate_repo=Erion-Simoni/kiln-engine
candidate_run=43107026471 # Select a successful release-candidate run.
test "$(gh run view "$candidate_run" --repo "$candidate_repo" --json conclusion --jq .conclusion)" = success
candidate_sha=$(gh run view "$candidate_run" --repo "$candidate_repo" --json headSha --jq .headSha)
candidate_dir="kiln-candidate-${candidate_run}"
mkdir "$candidate_dir"
gh run download "$candidate_run" --repo "$candidate_repo" \
  --name "kiln-linux-candidate-${candidate_sha}" --dir "$candidate_dir"
for candidate_file in kiln-portable.tar.gz source.tar.gz build-record.json build-driver.py comparison.json; do
  gh attestation verify "$candidate_dir/$candidate_file" \
    --repo "$candidate_repo" \
    --signer-workflow "$candidate_repo/.github/workflows/release-candidate.yml" \
    --signer-digest "$candidate_sha" --source-digest "$candidate_sha" \
    --deny-self-hosted-runners
done
mkdir "$candidate_dir/installation"
tar -xzf "$candidate_dir/kiln-portable.tar.gz" -C "$candidate_dir/installation"
cd "$candidate_dir/installation/kiln-0.1.0-linux-x86_64"
./kiln-cli verify-editor-bundle --input .
./kiln-editor
```

The success check deliberately stops the recipe for a failed or still-running
workflow. The first mkdir refuses to reuse an existing download directory.
Repository, workflow and both source/signer digest checks bind verification to
the selected run's commit. Do not substitute the current branch HEAD when it
differs from that run. CLI authentication and access to GitHub's attestation
service are required; local checksums are not a substitute if verification fails.
The extraction directory above matches current version 0.1.0; use the archive's
versioned root when consuming a later version.

Match the verified source revision with build-record.json and the workflow run.
Attestations establish GitHub workflow origin and subject digests, not OS code
signing, physical compatibility, complete licensing or hermetic dependency/SDK
provenance. Availability depends on repository Actions/attestation settings.
The workflow must finish successfully before hosted results are claimed.

Observed run #1 for 462d17f passed the hosted native/repeated-build/comparison and
browser/moved-installation/clean-Ubuntu acceptance gates. Both artifact uploads
failed, so attestation was not reached. The owner's screenshot supplies the exact
CreateArtifact error: HTTP 404, "workflow run not found". This does not establish
quota exhaustion, token expiry or a Node runtime cause. The workflow now updates
the artifact client and fails early on a failed upload/download probe (ADR-0068).
See [hosted evidence](evidence/task8-hosted-ci/README.md).

[Run #2](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43107026471) at
`0b9a67b21480a654e8d6fcc886e702dd1d2b1e33` completed successfully in 47m 9s.
All engine/rebuild/comparison/browser/relocation/clean-Ubuntu gates passed, both
candidate/evidence uploads succeeded, and the separate job downloaded the
candidate, created the build-origin attestation and uploaded its bundle.
hosted-run2-complete.json records public step conclusions and the overall success.
The candidate files and signatures were not independently downloaded/verified in
this cloud environment; the verification recipe above is the consumer procedure.
This closes the observed hosted publication/attestation execution gate for that
source revision. It does not prove the original 404's cause, independent-builder
reproducibility, licensing, OS signing or physical/other-platform acceptance.
