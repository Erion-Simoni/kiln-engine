# Kiln Session Handoff

**Active milestone:** A01 — Restore trustworthy CI and baseline (PARTIAL).
**Branch:** fix/a01-ci-baseline; previous pushed repair aeda8dd.
**Owner preferences:** batch small tasks. No release-size ceiling; keep performance
and resource budgets. MIT OR Apache-2.0 settled. Full 3D gated on G2D.

## Done this batch

- Observed fresh hosted run 43507314005/aeda8dd: documentation, Rust tests and native
  editor/UI smoke passed. Windows archive step failed; other archive platforms
  canceled by fail-fast. Reference games 43507313970 and other CI work were still
  running when inspected. These observations do not establish full acceptance.
- Reproduced the broken cargo-dist installer URL (HTTP 404). curl-to-sh masked the
  download error. Use actual cargo-dist-installer.sh/.ps1 assets, download first,
  run in platform-native shells and require cargo-dist 0.33.0 before building.
- Official Unix installer verified into workspace tools. Windows installer and
  Windows/macOS/Linux archive assets downloaded successfully; native Windows
  execution remains for hosted verification.
- Real local archive build exposed missing `profile.dist`. Added it inheriting
  release. Complete archive compilation remains unverified after this correction.
- Archive platforms now finish independently, preserving failure/build logs.
- Linux and Windows candidate workflows now cover fix/** pushes so this repair
  branch can receive full bundle validation without authenticated PR API access.
- Full moved-editor acceptance passed: installed-tool integrity and tamper rejection,
  three templates with native/browser exports, both actual GUI export paths,
  cancellation/output preservation, relocated gameplay/save restoration and notices.

## Validation / evidence

- just check passed, 31 Python tests passed, actionlint and mdBook passed.
- Engine Rust behavior unchanged; prior 700 Rust tests passed with zero skipped.
- Latest complete portable acceptance: docs/evidence/A01_PORTABLE_ACCEPTANCE.json.
- Prior fresh three-game author/export/native/browser acceptance:
  docs/evidence/A01_REFERENCE_GAMES.json. All six gameplay cases passed.
- Combined diagnosis: docs/evidence/A01_CI_BASELINE.md. Raw logs/screenshots under
  target/a01-third-*; no generated artifacts committed.
- Existing matched development bundle SHA-256:
  9442954eb9e39f5645fd9df490821fe7f32022c27d8bf7f00971809aba53919e.
- Software graphics/Xvfb/Chromium/null audio and relocated Linux userspace are not
  clean-machine, signed installer or physical GPU/audio/controller certification.

## Next / known issues

1. Push this batch and inspect fresh general CI, reference games, Linux candidate
   and Windows candidate runs. A01 stays open until source-pinned required hosted
   checks, native archives and candidate/artifact verification actually pass.
2. Git push and public GitHub HTML reads work through approved network commands.
   GitHub CLI API returned Forbidden; authenticated step logs/PR creation unavailable.
   Do not treat an unread failure log as a diagnosed cause.
3. Full portable workflows remain separate from experimental binary archives.
   Windows/macOS execution, actual archive compilation, full installers/signing
   and physical devices still need their respective evidence. No release gate closed.
4. After A01 acceptance, A02 covers durable authoring and unsaved source recovery.
   docs/ENGINE_COMPLETION_PLAN.md remains the authoritative plan.
