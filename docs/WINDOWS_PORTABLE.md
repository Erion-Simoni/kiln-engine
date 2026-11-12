# Windows portable editor candidate — Task 8

[Windows candidate run 43215982526](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982526)
completed successfully in 7m 29s for source
`3ff72d37f2a05641eac78098e5bc4e928ff8581f`. Packaging, moved installation, all
template exports, visible editor/player captures, normal close, candidate upload
and separate build-origin attestation/bundle upload passed on Windows 2025/DX12.
See [executed evidence](evidence/task8-windows/README.md) for scope and history.

The ZIP contains the matched x64 MSVC release editor, CLI, player, default
physics/material-enabled browser runtime, templates, examples and original
dependency/font/Rust notices. The Windows target statically links the MSVC CRT;
the workflow inspects all three binaries and rejects dynamic MSVC CRT imports.
Windows graphics/audio/controller APIs and drivers are supplied by the OS.

## Download and start

From the successful workflow run, download `kiln-windows-candidate-<source SHA>`
under Artifacts (GitHub login required). Extract that artifact, then extract the
complete `kiln-windows-x64.zip` inside it into a fresh directory. Keep every file
and subdirectory together. Launching the executable inside Explorer's compressed
folder does not install its sibling tools and resources.

In PowerShell, change into the extracted `kiln-0.1.0-windows-x86_64` directory:

```powershell
.\kiln-cli.exe verify-editor-bundle --input .
if ($LASTEXITCODE -ne 0) { throw 'Installation verification failed' }
.\kiln-editor.exe
```

Create/open working projects outside the installation, or copy
`examples\ui2d_editor` into a working directory first. The editor finds its
exporter/player/browser runtime beside its executable. Authoring, Native/Browser
export and exported-game startup need no Rust, Cargo, Node, Python or bindgen.
Serve browser outputs over HTTP(S). Preserve all exported files when moving a
game, and start its `game.exe` directly.
Current exporters include and verify the selected installation's complete
runtime notices automatically. Historical candidate 3ff72d3 predates this feature:
copy its installation's `licenses` directory beside exported game/browser files
before distribution. Retain your own asset notices. See
[the portable notice contract](PORTABLE_EDITOR.md#notices).

The installation manifest detects damage; it does not authenticate a publisher.
For GitHub origin verification, use a current authenticated GitHub CLI and verify
both `kiln-windows-x64.zip` and `preparation-record.json` before extraction:

```powershell
$candidateRepo = 'Erion-Simoni/kiln-engine'
$candidateSha = '3ff72d37f2a05641eac78098e5bc4e928ff8581f'
foreach ($file in @('kiln-windows-x64.zip', 'preparation-record.json')) {
    gh attestation verify $file --repo $candidateRepo `
      --signer-workflow "$candidateRepo/.github/workflows/windows-candidate.yml" `
      --signer-digest $candidateSha --source-digest $candidateSha `
      --deny-self-hosted-runners
    if ($LASTEXITCODE -ne 0) { throw "Origin verification failed: $file" }
}
```

Use the selected run's source SHA, even if the branch has later documentation
commits. Candidate/attestation artifacts retain 14 days; diagnostic logs and
screenshots retain seven. Select a later successful run after expiry.

## Acceptance and limits

The hosted Windows 2025 runner builds from the tracked Cargo.lock using pinned
Rust and matching bindgen. Compiler outputs may be cached by crate inputs,
lockfile, Rust and runner image; this is prebuilt preparation, not a fresh-build
reproducibility claim. The standard-library packager verifies the staged
installation before exclusively publishing its ZIP. Acceptance then extracts
and moves it outside the checkout to a path with spaces, uses an unrelated
working directory and removes Rust/Node/bindgen from child PATH. It checks:

- All three release tool identities, complete installation inventory and rejection
  of a changed player. Verification must pass after restoring it and after use.
- Creation of all three templates and actual Windows/browser exports. Reusing a
  nonempty project directory must fail without replacement.
- Relocation of a standalone game after its original project path disappears.
- Visible editor and exported-player startup, nonblank client-area captures,
  and normal window close with zero exit status. Failed startup/capture/close
  blocks candidate publication. Captures are retained for review; color variety
  does not establish semantic correctness of every UI/game property.

After acceptance, a separate job creates GitHub origin attestation without
checking out or executing engine code. `preparation-record.json` records actual
compiler/bindgen, native flags, lock/archive hashes, DLL observations and acceptance.
This is hosted prebuilt preparation; it does **not** extend the captured Linux
builder/replay contract or claim Windows reproducibility.
Public job conclusions establish successful execution; the artifact contents,
signatures and retained captures were not independently downloaded/reviewed in
this cloud environment. Consumer origin verification remains the procedure above.

Full Windows GUI author/save/undo/restart/export/gameplay acceptance, other GPUs,
physical Intel HD 520 performance, audible/controller tests and older Windows
versions remain separate. Follow [PHYSICAL_DEVICE_TEST.md](PHYSICAL_DEVICE_TEST.md)
on an actual machine. Executables lack Authenticode signing; GitHub origin
attestation is separate. Project code is MIT OR Apache-2.0; both texts and original
third-party notices accompany newly prepared candidates. See
[PORTABLE_EDITOR.md](PORTABLE_EDITOR.md#notices). Package size has no release goal.
