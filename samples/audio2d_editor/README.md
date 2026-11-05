# Native audio authoring fixture

Open `kiln.toml` using Open Project in the editor. In Audio Editor, preview `hit`
and `theme`, change the Combat/Music bus controls, save and reopen. Press Play
for the looping 220 Hz music plus two overlapping 880 Hz effects. Stop Play to
stop playback. The scene has no authored graphics; this fixture tests audio.

Run the standalone player from the repository root:

```bash
cargo run -p kiln-player -- --archive samples/audio2d_editor --game-name audio2d-demo
```

Choose Start Game. The scene starts music; the attached emitter sets master
volume to 0.5 and Combat to 0.5, stores language `sq`, and triggers both effects.
Those values demonstrate script setting writes; change the emitter script for
other user defaults. Saved mixer defaults affect editor preview; player overrides
are in `preferences.json`, separate from the project. This fixture does not
implement Task 7's saved UI or generic game/world save restoration.

Regenerate the manifest/scripts/tones with:

```bash
KILN_TASK7_SAMPLE_OUT=samples/audio2d_editor cargo test -p kiln-player --test audio_workflow audio_authoring_roundtrip_and_packed_player_produce_identical_pcm
```

The tiny WAV tones are synthesized by that acceptance test with no external
media. Its default run also authors, saves/reopens, packs/loads a real KAR, and
compares direct and packed-player PCM without opening a device. See
`docs/AUDIO_AUTHORING_2D.md` for supported controls and remaining acceptance.
