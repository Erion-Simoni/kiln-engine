# Task 8 hosted verification — 2026-11-11

**Engine acceptance passed; artifact publication failed. Attestation was not reached.**

[GitHub run #1](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43088213774)
executed workflow commit `462d17f5cbede3af7993e260281215db8a0d65c3` on the feature
branch. hosted-observation.json preserves public job step conclusions and their
timestamps. It is an observation record, not a downloaded attestation or a claim
that the complete workflow succeeded.

## Observed successful gates

- Official pinned Rust toolchain, Node, Linux requirements and matching build tools.
- Native formatting/Clippy/check/tests and producer/replay/comparison tests.
- Fresh captured release build, then a fresh rebuild of frozen inputs.
- Actual comparison of all six editor/CLI/player/JS/WASM/descriptor payloads.
- Release browser material and physics behavior.
- Moved installation, real editor native/browser export and relocated gameplay.
- Clean Ubuntu acceptance without Rust/repository access, including installed
  editor/player materials. Candidate staging also succeeded.

## Publication blocker

Both **Upload accepted candidate** and **Retain acceptance and failure evidence**
reported failure. The dependent attestation stage was not reached. The owner later
supplied a screenshot of the run summary with two identical annotations:

> Failed to CreateArtifact: Received non-retryable error: Failed request: (404) Not Found: workflow run not found

The screenshot also warns that older Node 20 actions were forced onto Node 24.
These are direct diagnostics, not proof of quota exhaustion, token expiry or a
runtime-migration cause. The previous public observation JSON remains unchanged
as the historical observation made before receiving the screenshot.

Anonymous public pages exposed step conclusions but not detailed logs. Fresh
summary reads also encountered upstream 502 responses. The GitHub API request
was explicitly denied by the managed network proxy (CONNECT 403); the running
environment's enforced revision 1 does not allow api.github.com.

A restricted-network draft adding api.github.com was saved successfully,
requires_publish=true, and remains unapplied. Review/save/publish it through
environment settings, then refresh status and retry the public diagnostics request
with a fresh connection. See diagnostics-blocker.json. No new secret value is
requested for this domain-only draft. The old system gh CLI also lacks the
attestation command; use a current GitHub CLI for independent artifact verification.
The owner's screenshot answers the earlier request for the exact upload error,
allowing remediation work to proceed while API access remains unavailable.

No remote_desktop capability/cua_repl.js control tool was advertised. Public HTTPS
pages supplied the status observations above. No signed bundle or artifact upload
success is claimed.

## Local checks and remaining work

local-preflight.json was recorded before hosted execution: actionlint 1.7.7,
665 Rust tests and 19 Python tests passed; actual source capture includes the
known root lockfile once. Root Cargo.lock SHA-256 remains
`042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17`.

ADR-0068 updates official action pins to native Node 24 versions and adds a small
upload/download/byte-comparison preflight before the expensive build. Cargo tool
cache entries now include installation manifests. Local actionlint and just check
pass; 19 Python tests pass. Executing the actual probe scripts locally accepts
matching downloads and rejects changed/missing bytes. This does not exercise
GitHub's artifact service: hosted upload/attestation still require observation.

Next: observe the revised workflow's preflight, candidate upload and attestation.
If the same 404 recurs immediately, investigate GitHub's run/artifact service with
the exact run identity and error rather than spending another full build first.
Physical-device/reference-hardware and
other-platform checks, project licensing and OS installer signing remain open.
PHYSICAL_DEVICE_TEST.md supplies a manual checklist, not new device evidence.
The owner was asked to resolve README License:TBD versus workspace
MIT OR Apache-2.0; existing license metadata remains unchanged pending an answer.
Package size is informational. Task 9/full 3D remain unstarted.
