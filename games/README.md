# Editor-authored 2D reference games

These three small games exercise Kiln's actual editor, scripts, saved projects,
menus, resources and exports. They use simple graphics and short levels. They
are reference games, not a claim that every engine release gate is complete.

| Game | Goal | Main controls |
| --- | --- | --- |
| [Emberwatch](emberwatch/README.md) | Defeat three hostile types, then secure the relay | Arrows/WASD; click FIRE |
| [Skybridge](skybridge/README.md) | Jump through two physical platforming levels | Left/Right or A/D, Up/W jumps; S save, L load, R replay |
| [Switchyard](switchyard/README.md) | Restore three five-lamp circuits | Left/Right or A/D selects; click the switch button |

Build/start the editor with `cargo run -p kiln-editor`. Use **File → Open Project**
and choose a game's `kiln.toml`. **Play Project Start** starts its first scene.
Stop restores authoring state. **Project → Script Editor** edits the game's
`.ks` sources. **View → Build / Export** saves and exports Native or Browser;
choose a new/empty output directory. Browser output must be served over HTTP(S).
Native players need no Rust, editor or source checkout.

All three projects were created through native editor Save As from existing
examples, given independent project identities in Settings, and scripted through
the editor's normal clipboard/text/save controls. Skybridge's animation, second
scene, camera copy, stage property, gate transform and input bindings were also
authored in editor widgets. Ordinary artwork was supplied as an asset. No game
manifest generator or game-specific engine entry point was added.

The repeatable GUI driver creates fresh projects, pastes the checked-in gameplay
sources, reproduces the saved projects, compares all serialized project values,
exports both targets and verifies matching runtime notices. Native and Chromium
drivers then play only exported artifacts; native source project paths are made
unavailable. See [Task 9 evidence](../docs/evidence/task9-games/README.md).

Code and Kiln artwork use the repository's MIT OR Apache-2.0 license. The copied
font retains its original grant in each game's `assets/fonts/LICENSE.txt`.
Physical GPU/audio/controller, independent tutorial, long-session/large-project
and broader platform/release checks remain separate. Package size is not a goal.
