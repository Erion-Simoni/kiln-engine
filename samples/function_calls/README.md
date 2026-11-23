# Shared function calls

A red square becomes green when imported `evaluate(2)` returns 7 and nested
`bump(bump(0))` returns 2. `evaluate` calls the same imported body twice, and the
entry script calls that body again with nested argument evaluation. There is no
engine-specific sample logic. Play or export this ordinary project with matching
native/browser tools.

Native relocated pixel replay:

```sh
KILN_ATLAS_PROJECT=samples/function_calls KILN_PIXEL_SCENARIO=function-calls KILN_ATLAS_PLAYER_EVIDENCE=target/a04-functions-native python3 tests/native/atlas_player_smoke.py
```

Browser relocated pixel replay (after preparing matching web-runtime):

```sh
KILN_ATLAS_PROJECT=samples/function_calls KILN_PIXEL_SCENARIO=function-calls WEB_ATLAS_EVIDENCE=target/a04-functions-browser node tests/web/atlas.cjs target/a04-functions-web-runtime
```

The shared green-pixel observers also serve atlas checks; the scenario flag uses
generic rendered-scene receipts. Both hide the authoring copy before playback.
