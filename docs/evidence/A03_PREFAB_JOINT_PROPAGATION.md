# A03 reference-safe prefab Apply and propagation

2026-11-19, `fix/a01-ci-baseline`, following `0431883`. A03 stays PARTIAL.

Three controlled regressions fail before the fix: Apply stores an instance's joint
owner ID in its template; propagation leaves the template ID in destination joints;
and Apply accepts an endpoint missing from another scene. Raw failing evidence:
`target/a03-prefab-joints-before.log` (3 failed assertions, after fixture compilation).

ADR-0092 stages the complete project, maps source-owner joint references to the
prefab template and maps template-owner references to each instance. External
endpoints retain literal IDs and must resolve uniquely in each destination scene
composition. Newly changed joints validate against all staged scenes before any
project publication. Failure preserves definitions and scenes; unrelated unchanged
legacy joints remain repairable. The prefab panel validates its entire batch before
publishing definitions or updating live instances, and restores displayed definitions
on failure. Apply captures current unsaved scene data into its staged source before
applying, then records history only after success. No serialization/dependency change.

Six headless regressions verify remapping for Apply and propagation, atomic missing
and ambiguous endpoint refusal, unique nested endpoints, complete project save/reopen,
runtime-spawned joint identities, and unrelated legacy-joint tolerance. Existing
entity commands, hierarchy flags and explicit texture-clear propagation remain
covered by the workspace suite. Raw: `target/a03-prefab-joints-{after,integration,workspace,quality-final}.log`.

These are editor-model/reference and runtime-spawn checks. This checkpoint does not
claim a new physical physics/GUI/native/browser visual replay, scoped nested prefab
identities, prefab variants, general multi-file crash transactions or full A03 closure.
Full combined renamed-art/prefab/atlas/multiple-scene/export acceptance remains open.
