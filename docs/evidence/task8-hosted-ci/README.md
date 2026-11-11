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
reported failure. The dependent attestation stage was not reached. Their exact
error text/root cause is **not established**; storage/quota, permissions and
service faults must not be guessed from the failed-step label alone.

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
The owner was also asked for the exact upload-step error text as a direct
diagnostic input, which can be used while API access remains unavailable.

No remote_desktop capability/cua_repl.js control tool was advertised. Public HTTPS
pages supplied the status observations above. No signed bundle or artifact upload
success is claimed.

## Local checks and remaining work

local-preflight.json was recorded before hosted execution: actionlint 1.7.7,
665 Rust tests and 19 Python tests passed; actual source capture includes the
known root lockfile once. Root Cargo.lock SHA-256 remains
`042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17`.

Next: read actual upload diagnostics after approved API access, repair the cause
and verify candidate upload/attestation. Physical-device/reference-hardware and
other-platform checks, project licensing and OS installer signing remain open.
PHYSICAL_DEVICE_TEST.md supplies a manual checklist, not new device evidence.
The owner was asked to resolve README License:TBD versus workspace
MIT OR Apache-2.0; existing license metadata remains unchanged pending an answer.
Package size is informational. Task 9/full 3D remain unstarted.
