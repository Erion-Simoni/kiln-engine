# 2D audio and player settings

Task 7's native audio checkpoint implements import, project persistence, preview,
real decoding/mixing, music, effects, buses and player preferences. Saved native runtime UI is covered in [UI authoring](UI_AUTHORING_2D.md).
General scene/entity/script game saves are still unfinished. The browser
continues to use its older WebAudio callback; native mixer/settings parity there
is pending.

## Author audio

1. Save/open a project and open **Audio Editor** from the editor's panel menu.
2. Enter a unique audio name and a project-relative or absolute source path.
   Enable Music for tracks; click **Import audio**. Default builds validate by
   decoding before changing the project. WAV, OGG Vorbis, MP3 and FLAC are
   supported by the existing decoder; Opus is not promised.
3. External files are copied to `assets/audio/`. Existing internal files retain
   their paths. Name/file collisions report an error and preserve existing data.
   Copying does not remove the source. Save Project to persist the library.
4. Assign gain and a bus. Empty routing chooses SFX or Music; custom buses can
   be added. Master multiplies the bus and asset gains. Mute suppresses a bus;
   when any bus is solo, other buses are silent. Master is reserved.
5. Click **Preview** outside Play mode. Preview opens its own default output
   device, displays decoder/device errors and stops with **Stop preview**.
   Starting Play or replacing the project stops preview. Music's Loop setting
   controls repetition; gain/routing changes take effect on the next preview.
   Mixer changes affect an already playing preview.

Audio edits participate in project dirty tracking/save/recovery through the
existing project model. During Play the authoring panel is locked. Script
settings are player overrides; preview always uses authored defaults.

## Use in scripts

```ks
// Inside an entity's start callback, or guarded scene initialization:
play_music("theme");
play_sound("hit");
play_sound("hit");  // A separate overlapping voice.
set_music_volume(0.4);
set_bus_volume("Combat", 0.7);
setting_set("language", "sq");
let language = setting_get("language", "en");
```

The scene and all attached entity callbacks use one audio context. A track is
registered once at session creation, using project-relative files in editor Play
and archive resource bytes in the native player. A new Play session starts new
playback. Scene transitions reinitialize playback and retain saved preferences.
`play_music` replaces the previous track; missing/wrong-category names do not
start a voice. `stop_sound(name)` stops all matching voices; pause/resume applies
to matching voices. `stop_all_sounds` stops music and effects. Completed effects
are reaped; the pool is capped at 128 voices, evicting the oldest effect first.
No spatial audio, streaming music, output selection or hot-plug recovery is
implemented in this checkpoint. Full-file decoding shares PCM among voices.

## Persistence and errors

`preferences.json` has schema version 1, optional audio overrides, and arbitrary
typed settings. It lives in the editor project's `.kiln-save/` directory or the
native player's save directory. Use a distinct `--game-name` for each standalone
game; the existing player's default name shares a profile. Preferences are a
separate document from authored `kiln.toml` and `slot_*.json` game saves.

Volume setters save overrides immediately when configured by a native host.
`setting_set` requires a configured directory; an unchanged value skips writing.
Settings support I64 (including integers beyond 2^53), finite F64, booleans,
strings and arrays. Keys are limited to 256 bytes and 4096 distinct entries.
Write failures leave the previous setting intact and return a script error.
Files are written/flushed completely to a sibling temporary file before rename.
Missing preferences use author defaults. Corrupt or newer-version files produce
an error and block writes; repair/remove the file and restart Play to reload.
There is no silent reset that overwrites an unreadable user's settings.

`state_get`/`state_set` are still per-instance memory. The older `save_game` API
still saves the vertical-slice fields; it does **not** save these instance values,
world entities or scene transitions. Generic game restoration is the next save
checkpoint and must be completed before Task 7 is checked off.

## Reproduce acceptance

```bash
just check
just test
KILN_TASK7_SAMPLE_OUT=samples/audio2d_editor cargo test -p kiln-player --test audio_workflow audio_authoring_roundtrip_and_packed_player_produce_identical_pcm
cargo run -p kiln-editor
cargo run -p kiln-player -- --archive samples/audio2d_editor --game-name audio2d-demo
cargo run --manifest-path samples/audio/Cargo.toml -- samples/audio2d_editor/assets/audio/hit.wav
```

The sample generator writes two small synthesized tones and a scene/entity
script fixture; no external licensed media is needed. In the editor, use Open
Project and choose `samples/audio2d_editor/kiln.toml` before pressing Play.

Unit tests pull real decoded/mixed PCM from the same Rodio mixer used by native
output: overlap/gain, looping/replacement/completion, pause/resume/stop/replay,
solo/mute and voice limits. Integration tests import external resources, save and
reopen the project, pack a real KAR, load it with the player's loader, execute
scene/entity VMs and compare the direct/packed PCM. Settings tests cover restart,
precision and corrupt/future/unwritable documents, plus callback error recovery.
None of these checks certifies audible speakers/headphones, native GUI clicks,
controller-operated saved UI or clean-machine exports. Those acceptance gates
remain open; see STATE.md for executed local evidence.
