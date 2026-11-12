# Task 8 Windows portable candidate — 2026-11-12

**Windows candidate preparation, installed acceptance, publication and origin attestation passed.**

[Successful run 43215982526](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982526)
executed source/workflow `3ff72d37f2a05641eac78098e5bc4e928ff8581f` on
feat/2d-production-workflow. Overall status is completed/success, duration 7m 29s.
Both jobs completed, with no failed or missing required gates. Later documentation
commits do not change the candidate source or retrigger the build.

## Executed Windows gates

- Pinned Rust 1.99.1/Python 3.13 and lockfile-matched bindgen; native Node 24 actions.
- Windows ZIP validation tests, including original-name/case/link/traversal checks
  and the canonical-uppercase environment regression.
- Matched MSVC release editor/exporter/player and prepared default browser runtime.
- Actual DLL import inspection rejecting dynamic MSVC CRT requirements.
- Windows ZIP with complete project and original dependency/font/Rust notices;
  staged installation verification before exclusive archive publication.
- Installation extraction and relocation outside the checkout to a path with spaces;
  unrelated working directory and Rust/Node/bindgen absent from child PATH.
- Installed tool/profile identities, corruption rejection, restoration and final
  installation verification. All three templates initialize/export Windows/browser;
  reusing a nonempty project destination is rejected.
- Standalone game relocation after its source project path disappears.
- Visible editor/player startup, foreground client-area captures with nonblank color
  diversity, and normal window close with zero process status. Startup logs reject
  panic/GPU-validation failures; these are not full semantic GUI/gameplay tests.
- Accepted-candidate and diagnostic uploads. A separate no-checkout job downloaded
  the candidate, created GitHub build-origin attestation and uploaded its bundle.

hosted-success.json preserves public job IDs, step conclusions/timestamps and
GitHub's overall success. These observations establish hosted execution and
publication. Candidate files/signatures and retained PNG captures were **not
independently downloaded/reviewed here**; no artifact digest or exact remote test
count is inferred from public step names. Follow
[WINDOWS_PORTABLE.md](../../WINDOWS_PORTABLE.md) for artifact download, full ZIP
extraction, installation integrity and both-file origin verification. Candidate /
attestation retention is 14 days; diagnostic logs/captures retain seven days.

## Preparation and scope

Windows uses hosted prebuilt preparation observations. The record includes
compiler/bindgen, target flags, runner image, lock/archive hashes, DLL requirements
and acceptance. Compiler outputs may be cached by Rust, image, lock and crate
inputs. Cache hits do not skip downstream gates. This is not the captured Linux
builder/replay contract, a fresh independent build, byte-reproducible Windows PE
proof, hermetic SDK/dependency audit or Authenticode signing.

The Windows 2025/DX12 desktop result does not certify a physical Intel HD 520,
audible devices/controllers, older Windows versions or full editor GUI author /
save/undo/restart/export/gameplay. PHYSICAL_DEVICE_TEST.md covers remaining manual
checks. Runtime notice propagation into game outputs is currently a manual copy
of the complete installed licenses directory; automation remains Task 8 work.

## Failures diagnosed and fixed

- Run 43205341357 at c812b1a failed ZIP validation. Windows ZipInfo normalizes
  backslashes; inspect original central-directory names and preserve malformed
  raw names in test fixtures. A local Windows-style normalization reproduction
  failed before the fix and passed afterward; replacement hosted validation passed.
- Run 43206825541 at b1cae0c compiled native/WASM releases and passed static-CRT
  inspection, then failed packaging. Existing .gitattributes already protects
  pinned reference bytes, so the newline hypothesis was ruled out. Fixed
  locale-dependent metadata reads to explicit UTF-8 and added a Unicode/CP1252
  inventory roundtrip regression. Exact first packaging exception was not exposed
  publicly. Run 43211211300 at 1afc792 then passed packaging.
- That run stopped at installed acceptance. Run 43213975426 at 6947224 exposed
  the actual annotation: KeyError SystemRoot in the screenshot helper. A plain
  copy of Windows os.environ has uppercase keys. Use SYSTEMROOT consistently;
  a mocked Windows API/uppercase-only dictionary regression exercises this case.
- After that fix, 43215982526 passed the complete Windows workflow. Earlier
  first-run.json, packaging-passed-acceptance-failed.json and installed-diagnostic.json
  remain historical observations. No failed run is presented as accepted.

## Local validation and project license

just check passed; local Rust suite passed 665 tests with zero failures/skips.
All 29 Python preparation/build/comparison/ZIP/notice tests pass. Actionlint 1.7.7,
Bash snippets and build/signing permission checks pass. No Rust source/dependency
or root Cargo.lock changed; lock SHA-256 remains
042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17.

The owner delegated the license choice. MIT OR Apache-2.0 is retained with full
root LICENSE-MIT/LICENSE-APACHE, consistent metadata for all 34 workspace packages
and unchanged third-party/font grants. Actual local prebuilt Linux packaging
verified 1544 installation files and both exact project grant payloads. The
historical local-validation.json records its actual preparing checkout/dirty flag;
it is not a Windows artifact hash or a fresh captured build claim.

A refreshed Linux candidate run for source 3ff72d3 is still in progress at this
checkpoint. Earlier accepted Linux run 43107026471 at 0b9a67b remains historical
software evidence with its original pre-license-cleanup inventory. Task 8 remains
open for runtime-notice automation, full/other-platform tests, physical hardware,
SDK/dependency review and OS signing. Task 9/full 3D remain unstarted.
