# Task 8 hosted verification — 2026-11-11

**Run #2 completed successfully: engine acceptance, candidate publication and build-origin attestation.**

[GitHub run #2](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43107026471)
executed source/workflow commit `0b9a67b21480a654e8d6fcc886e702dd1d2b1e33` on
`feat/2d-production-workflow`, completing in **47m 9s**. Later documentation-only
commits do not retrigger the candidate workflow or change this candidate's source.

## Run #2 — observed successful gates

- Artifact-service probe upload, download and actual byte comparison.
- Native formatting/Clippy/check/tests and producer/replay/comparison tests.
- Fresh captured release build, then a fresh rebuild of frozen inputs.
- Actual comparison of all six editor/CLI/player/JS/WASM/descriptor payloads.
- Release browser material and physics behavior.
- Moved installation, real editor native/browser export and relocated gameplay.
- Clean Ubuntu acceptance without Rust/repository access, including installed editor/player materials.
- Candidate staging, accepted-candidate upload and diagnostic-evidence upload.
- Separate attester download, GitHub build-origin attestation and attestation-bundle upload.

`hosted-run2-complete.json` records public job IDs, step conclusions/timestamps
and the overall completed/success status from the workflow listing. No required
gate failed or remains missing. `artifact-preflight-run2.json` is the earlier
observation made while the candidate was still building, preserved as history.
The candidate and attestation artifacts have 14-day retention; diagnostics have
7-day retention and the tiny probe one day.

These observations establish successful hosted execution, publication and
attestation creation. Candidate contents, hashes and signatures were **not
independently downloaded/verified in this cloud environment**. No remote test
count or artifact digest is inferred from a public step label. Use the
[completed-run download/verification recipe](../../BUILD_PROVENANCE.md#hosted-release-candidate-workflow)
with a current authenticated GitHub CLI. It verifies all five files against the
selected run's source and signer SHA before extracting/running the editor.

## Run #1 — historical failure and remediation

[GitHub run #1](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43088213774)
executed `462d17f5cbede3af7993e260281215db8a0d65c3`. `hosted-observation.json`
preserves the successful engine gates and failed candidate/evidence upload step
conclusions observed before exact error text became available. Its dependent
attestation stage was not reached. The owner later supplied a screenshot with
two identical annotations:

> Failed to CreateArtifact: Received non-retryable error: Failed request: (404) Not Found: workflow run not found

The screenshot also warned that old Node 20 actions were forced onto Node 24.
ADR-0068 updated immutable official action pins to native Node 24 releases,
added the early upload/download/byte-comparison gate and included Cargo tool
installation manifests in the cache. Run #2 passed after those changes. This
sequence does not establish quota exhaustion, token expiry or the original
404's root cause; it does establish that the revised workflow completed.

## Access and validation scope

Anonymous public pages expose conclusions but not authenticated detailed logs
or candidate downloads. The environment's enforced revision 1 still excludes
api.github.com. The earlier domain-only settings draft is unapplied and requires
owner review/save/publish before relying on API access; see diagnostics-blocker.json.
Git push and public status reads work. No credential values were extracted or
requested, no network policy was bypassed and no public GitHub Release was made.
The installed old gh CLI lacks attestation support; consumer verification requires
a current CLI and normal authentication/service access.

`local-preflight.json` records the original local validation: actionlint 1.7.7,
665 Rust tests and 19 Python tests passed. The subsequent action update also
passed just check/actionlint and those Rust/Python tests, plus actual local probe
scripts accepting matching files and rejecting changed/missing downloads.
This session reran just check and checked the documented Bash syntax, current
official GitHub CLI verification flags and actual release archive root. These
recipe checks are not an executed authenticated consumer download/verification.
Root Cargo.lock remains unchanged, SHA-256
`042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17`.

## Remaining Task 8 work

Physical/reference-hardware and other-platform checks, project licensing,
remaining SDK/dependency provenance and OS installer signing remain open.
PHYSICAL_DEVICE_TEST.md supplies a manual checklist, not physical evidence.
README:TBD conflicts with workspace MIT OR Apache-2.0; eight engine crates and
three sample packages lack license metadata and no root project license text is
present. The owner's earlier license-policy question remains unanswered, so no
grant was invented. Package size is informational. Task 9/full 3D remain unstarted.
