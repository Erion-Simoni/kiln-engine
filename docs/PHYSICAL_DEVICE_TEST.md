# Physical device acceptance — Task 8

Use this checklist on the actual test machine with its normal interactive desktop,
speakers/headphones and controller. Intel Core i7-6600U / Intel HD 520 is a useful
reference configuration. A cloud VM, Xvfb, SwiftShader/Mesa CPU adapter, null audio
or simulated gamepad result remains software evidence.

The current candidate is Linux x86_64 with Ubuntu 24.04 / GLIBC 2.39 requirements.
Windows/macOS need matching native binaries/bundles before running this checklist.
Record each result as **pass**, **fail** or **not run**, with the actual device and
method. This document prepares the checks; no physical-device pass is implied.

## Identify the installation and device

Keep the extracted installation intact. Verify it with its included CLI:

```sh
/path/to/installation/kiln-cli verify-editor-bundle --input /path/to/installation
/path/to/installation/kiln-editor --tool-info
/path/to/installation/kiln-player --runtime-info
```

Record archive SHA-256/source revision, OS/kernel, CPU, RAM, display resolution/
scale, graphics driver/backend, audio output and controller model/connection.
Confirm the actual rendering device. wgpu adapter enumeration can be logged with
`RUST_LOG=warn,wgpu_core::instance=info,kiln_editor=info`; enumeration may list
multiple adapters, so leave selected-GPU verification open if the active device
cannot be established. Note any software-rendering, ICD, virtual-display or null-
audio overrides in the session. Use device/driver activity diagnostics as needed.

## Author, export and play

1. Copy `examples/ui2d_editor` outside the installation into a fresh working
   directory. Launch the installed editor from the real desktop, open the copy,
   edit an entity, save, reopen, undo/redo and use Play/Stop. Reopen after an editor
   restart and verify the authored values. Keep the working project/logs as evidence.
2. Use actual Native Save & Export. Move the output to a path with spaces, make the
   working project copy unavailable, then launch its `game` executable with a fresh
   dedicated `--game-name` profile. Check title, settings, movement, pause, health/
   inventory, save, process restart and Continue. The demo README describes F5/F9
   and its menu actions. Record failures and the displayed/logged error.
3. Listen for title music and both overlapping effects from Take 7 damage. Change
   the sound setting and confirm audible level changes persist after restart.
   Record the physical output route and audible result.
4. Connect the physical controller. Check menu navigation/activation and intended
   gameplay actions, including release and reconnect. Record the model and results
   separately from keyboard/mouse. Mark unavailable hardware as not run.
5. Open/export `examples/materials2d`: compare the default/custom colors, named
   textures, transparent order, flipped stripe and rotated stripe in editor and
   player. S saves, N changes scenes, L restores. Open/export the physics example
   and exercise its floor/sensor/joints/movement/save controls from its README.
6. Use Browser Save & Export and serve the moved output over HTTP(S). On the actual
   browser/device check loading, explicit start/audio consent, input, music/effects,
   settings, save, reload and Continue. Keep browser/version-specific results.
   Test available touch/controller hardware and retry/failure feedback separately.
7. Verify the installed bundle again after use. Working projects belong outside
   the installation, so the installed file inventory can still match.

## Measure and record

Use the active [budgets](BUDGETS.md): desktop startup to first rendered frame,
empty-project editor idle memory and Low reference-device frame time. Record exact
project/scene, tier, resolution, power mode, cold/warm state, measurement tool and
duration. Include a representative play/edit session and any recovery/data loss.

Startup runs from process launch to the first presented frame; whole-process elapsed
time from `/usr/bin/time` measures session duration. OS RSS/working-set measurements
need their units and scope. A visible FPS counter is a coarse observation; frame-
time distribution and percentile claims require a real capture. Leave measurements
unmeasured until an appropriate instrument/capture exists. Software CI timings
cannot supply these physical reference-device measurements.

| Check | Device/method | Result | Evidence/observed value |
| --- | --- | --- | --- |
| Active graphics adapter/driver | | Not run | |
| Editor author/save/restart/undo/Play | | Not run | |
| Relocated native export/save/Continue | | Not run | |
| Material and physics examples | | Not run | |
| Audible music/effects/settings | | Not run | |
| Physical controller/reconnect | | Not run | |
| Browser loading/consent/input/save | | Not run | |
| Cold/warm startup to first frame | | Not run | |
| Empty editor idle memory | | Not run | |
| Frame times/session/recovery | | Not run | |
| Installation still verifies | | Not run | |

Keep platform-specific gaps explicit. Installer signing, project licensing and
GitHub build-origin attestations have separate evidence and completion criteria.
