# A01 CI baseline repair — 2026-11-13

Status: partial. Branch: `fix/a01-ci-baseline`, starting from `93b0ee0`.
Engine source is unchanged from master `ae492f1`; workflow, documentation and
smoke-script repairs are in the commit containing this record.

## Diagnosis and changes

Public run 43405013257 identifies failed steps: documentation build, cargo-dist,
saved UI authoring, and Browser export in the native editor. Public job pages
were retrieved; detailed step-log endpoints returned HTTP 404 without authenticated
log access. This is not evidence of the detailed hosted error text.

Local reproduction established these concrete faults:

- mdBook could not parse the space-containing Fixes Log link, then rejected
  repeated chapter files. The introduction and AI integration chapter referenced
  nonexistent files. Added a real introduction, repaired the link, retained one
  chapter per file, and removed the nonexistent AI chapter. The overview still
  documents engine structure. Pinned mdBook to the locally tested 0.4.52 and
  ignored its generated output.
- The saved-UI smoke opened Sprite Editor instead of UI Designer and subsequently
  missed the Add Label button. Updated menu, widget, drag and history coordinates
  against observed screenshots. Existing property/gameplay assertions remain.
- Native/browser export smokes used stale Build/Export menu and field positions.
  Updated actual widget clicks, including native cancellation. Existing save,
  artifact inventory, relocation and failure-preservation assertions remain.
- Reference games now run on master/main pushes and pull requests targeting those
  branches, as well as manual dispatch and the original reference-game branch.
  Removed path filters so sample/template or other build-input changes cannot
  silently skip game acceptance.

## Local validation

Linux cloud software graphics, isolated Xvfb displays, SwiftShader where available,
null audio; this establishes no physical GPU/audio/controller acceptance.

- `just check`: passed (fmt, strict all-target workspace clippy, workspace and audio sample checks).
- `cargo nextest run --workspace --locked`: 700 passed, zero skipped.
- `python3 -m unittest discover -s tests/native -p 'test_*.py'`: 29 passed.
- Native editor/player `--no-default-features` checks: passed.
- WASM strict clippy with default features and `--no-default-features`: passed.
- `mdbook build docs` with 0.4.52: passed. Generated book stays untracked.
- `actionlint -shellcheck= .github/workflows/*.yml`: passed with actionlint 1.7.7;
  shellcheck was disabled because it is not installed.
- `task7_editor_smoke.py`: passed. Added widget dragged to [104, 80], undo/redo,
  save/reopen, Play save/resume, health 66 and one script start verified.
- `browser_export_smoke.py`: passed. Inspector value 88 saved by export, previous
  output preserved on deliberate failure, exported inventory checked and relocated.
- `export_smoke.py`: passed. Same save/failure preservation, real cancellation,
  staging cleanup and relocation verified.

Local command logs/screenshots/results are under `target/a01-*`; generated files
are deliberately excluded from the commit.

## Still required before A01 acceptance

Retrieve authenticated cargo-dist and candidate/reference-game failure logs;
repair and reproduce remaining causes. The legacy cargo-dist.toml file is not loaded by cargo-dist 0.33.0. The next
batch below repairs supported metadata and separates experimental binary archives
from complete portable bundles and future installers.
Run the full three-game author/export/native/browser acceptance on this revision.
Require fresh hosted checks and artifact verification to pass before accepting
A01. Existing remote failures remain failures; local repairs do not close them.

## Second repair batch — 2026-11-13

- Reproduced the legacy distribution failure with cargo-dist 0.33.0: its ignored
  configuration left no configured targets. The first sandbox attempt also hit
  dependency download restrictions; fetching dependencies allowed the actual
  configuration error to be reproduced offline.
- Moved settings into supported Cargo workspace/package metadata. Only the three
  editor/CLI/player packages opt in; samples and demos are excluded. Removed the
  ignored file and unsupported DMG/AppImage settings. See ADR-0078: executable
  archives are experimental, complete portable bundles remain the supported
  distribution path, and full installers remain open.
- Corrected CI to invoke the installed `dist` executable and upload the actual
  `target/distrib` outputs, failing if absent. Removed the nonexistent, unreachable
  `cargo dist upload` step; publication remains outside this build workflow.
- Corrected the editor's repository/homepage and shared the real repository URL
  with all three distributable packages.
- Pinned general CI to the same Rust 1.99.1 and nextest 0.9.151 used by candidate
  workflows; use locked root dependency graphs. Updated cache/checkout/Node and
  artifact actions to the already-used pinned current versions. Both general CI
  and reference-game acceptance now trigger on `fix/**` pushes.
- Bounded failed-replay cleanup with termination/kill/reap and skip normal menu
  exit when handling an existing failure. Added actual child-process regressions
  for ignored termination and preserved exit status.
- Local validation: `just check`, 700 Rust tests (zero skipped), 31 Python tests,
  actionlint and mdBook passed. Real cargo-dist plans pass for Linux x64,
  Windows x64 and Intel macOS, each containing only the three intended tools.
  These are planning results, not actual target build/install acceptance.
- Repackaged current native/browser tools into a verified 1,546-file portable
  bundle (SHA-256 9442954eb9e39f5645fd9df490821fe7f32022c27d8bf7f00971809aba53919e).
  Reusing an older installed bundle first exposed a local tool mismatch; that
  result was discarded. The matched replay authored/exported Emberwatch and
  Switchyard. Skybridge initially exposed an input-binding replay mismatch; the corrected
  follow-up below passes while preserving the failed-run evidence.
- Git push access works. GitHub CLI API access returned Forbidden, so this session
  cannot create a PR or fetch authenticated hosted logs. Fix-branch push triggers
  allow checks to run without using that API. Hosted acceptance remains open.

Skybridge follow-up: retained a screenshot after opening Input Rebinding, allowing
its floating-panel layout to settle before clicking Add Action. Its fresh authored
manifest now equals the checked-in project, including all three input bindings.
The earlier mismatched manifest remains in local failed-run evidence. No expected
properties or gameplay assertions were removed.

Final gameplay acceptance: all three standalone exports pass from unrelated working
directories while source projects are hidden and build tools are absent from PATH.
All three browser exports pass nested HTTP deployment in Chromium/SwiftShader.
Checks cover victories, saved progress, position restoration, scene transitions
and restart/reload continuation. The compact durable results are in
[A01_REFERENCE_GAMES.json](A01_REFERENCE_GAMES.json). No physical-device or hosted
acceptance is inferred from these local passes.

## Third repair batch — 2026-11-13

Public run [43507314005](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43507314005)
for aeda8dd shows documentation, Rust tests and native editor/UI smoke passing.
Its Windows binary-archive job failed at Run cargo-dist, while Install cargo-dist
reported success. Other archive platforms were canceled by matrix fail-fast.
Reference-game run 43507313970 and other CI work were still running at observation.

- Reproduced the installer URL failure: `/v0.33.0/installer.sh` and `installer.ps1`
  return HTTP 404. The old curl-to-sh pipeline masked the failed download. The
  actual published assets are `cargo-dist-installer.sh` / `.ps1`; both downloaded
  successfully. The official Unix installer installed cargo-dist 0.33.0 into the
  workspace tool directory and its executable reports the exact expected version.
- CI now downloads to a file before execution, uses Bash on Unix and PowerShell
  on Windows, and requires the pinned executable version before building. The
  Windows installer asset was verified, but execution needs the Windows runner.
- A real local `dist build --artifacts=local --target=x86_64-unknown-linux-gnu`
  reached cargo and failed with `profile dist is not defined`. Added that profile
  inheriting existing release settings. Complete native archive compilation is
  still pending fresh hosted builds; planning alone was insufficient evidence.
- Disabled archive matrix fail-fast so independent platforms can finish and report
  their own failures. Capture build output with Bash pipefail behavior and retain
  distribution diagnostic artifacts even when a build fails.
- Enabled the full Linux and Windows portable candidate workflows on fix/** pushes
  so this branch can receive candidate validation without authenticated PR APIs.

The complete moved-editor acceptance passed using the matched development bundle:
all installed tools verified; tamper/missing-file/permissions/mixed-version/symlink
rejection; empty_2d/platformer/topdown initialization and both exports; actual GUI
native/browser exports; cancellation/failed-output preservation; relocated native
and browser gameplay/save restoration; unchanged installation and preserved notices.
See [A01_PORTABLE_ACCEPTANCE.json](A01_PORTABLE_ACCEPTANCE.json). This is relocation
in the same Linux userspace, not a clean machine or physical device. `just check`,
31 Python tests, actionlint and mdBook also passed for this batch. Rust behavior is
unchanged; the preceding 700-test pass still applies to the same engine source.

## Fourth repair batch — 2026-11-13

Observed public, source-pinned hosted results for 4b7f488:

- General CI 43509387416 **SUCCESS**: all three OS check/build/archive matrices,
  documentation, Rust/Python regressions, native UI and browser/portable checks.
  This establishes actual desktop archive builds, superseding planning-only evidence.
- Windows candidate 43509387540 **SUCCESS**, including candidate upload and origin
  attestation. This is the workflow's hosted contract, not physical/GUI certification.
- Reference games 43509387450 **FAILURE** in real editor authoring/export (step 9);
  quality checks and matching tool preparation passed; gameplay steps were skipped.
  Exact exception is unavailable through the public step-log endpoints.
- Linux candidate 43509387420 still **IN PROGRESS** on a fresh follow-up read;
  artifact-service preflight passed. Acceptance/attestation remains unconfirmed.

See [A01_HOSTED_STATUS.json](A01_HOSTED_STATUS.json) for the snapshot. Do not reuse
these results to accept a newer source revision without corresponding evidence.

Replay changes address known synchronization weaknesses; they are not a claim
that the unread hosted exception has been diagnosed:

- Observe the floating panel title bar with a bounded wait before measuring and
  dragging it; screenshot subprocesses have a timeout.
- Save and verify every newly created input action before binding it, then save
  and verify its exact binding before advancing. All writes still occur through
  real editor widgets; the project files are only read as postconditions. No retry
  creates duplicate actions, and full-project equality remains required.
- Retain failure.json with game/phase/type/message and expose an escaped GitHub
  error annotation. Project mismatch errors include a bounded expected/actual diff;
  binding waits name the action. Retain export failure JSON in workflow artifacts.
- Added two regressions covering local/hosted diagnostics and preventing multiline
  exception content from injecting a second workflow command.

Local fourth-batch validation passed: updated three-game real editor authoring,
exact full-project comparisons, six native/browser exports with verified notices
and output preservation on deliberate failures. Skybridge's new saved-action and
binding postconditions passed. `just check`, 33 Python tests, actionlint and mdBook
passed. Engine behavior is unchanged; prior native/browser gameplay evidence is
retained separately and was not rerun for these harness-only changes. Fresh hosted
reference-game acceptance and the Linux candidate result remain open.
