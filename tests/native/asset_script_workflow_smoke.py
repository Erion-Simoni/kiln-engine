#!/usr/bin/env python3
"""Real external texture refresh plus editor-authored counted-loop/dictionary gameplay."""
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import tomllib
import zlib
from task9_editor_games import Editor, REPO, executable, record_failure


def png(color):
    def chunk(kind, data):
        return struct.pack(">I", len(data))+kind+data+struct.pack(">I", zlib.crc32(kind+data))
    return (b"\x89PNG\r\n\x1a\n"+chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 1, 8, 6, 0, 0, 0))
            +chunk(b"IDAT", zlib.compress(b"\0"+bytes(color)*2))+chunk(b"IEND", b""))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--web-runtime", type=Path, help="Also verify the authored source in the real browser runtime")
    args = parser.parse_args()
    output = Path(os.environ.get("KILN_ASSET_SCRIPT_EVIDENCE_DIR", REPO/"target/a03-a04-workflow")).resolve()
    output.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    project = run/"project"
    shutil.copytree(REPO/"samples/vertical_slice", project)
    manifest = project/"kiln.toml"
    original_manifest = manifest.read_bytes()
    artwork = project/"art/hero.png"
    original_art = artwork.read_bytes()
    editor = Editor(REPO/"target/debug", project, output/"editor", ":85")
    def red_pixels():
        ppm = subprocess.check_output([executable("import"), "-display", editor.env["DISPLAY"],
                                       "-window", editor.window, "-depth", "8", "ppm:-"], env=editor.env, timeout=10)
        header, pixels = ppm.split(b"\n255\n", 1)
        width, height = map(int, header.splitlines()[-1].split())
        assert (width, height) == (1440, 900)
        return sum(1 for y in range(170, 650) for x in range(250, 1100)
                   if pixels[(y*width+x)*3] > 180 and pixels[(y*width+x)*3+1] < 100
                   and pixels[(y*width+x)*3+2] < 100)
    try:
        editor.start()
        before = red_pixels()
        artwork.write_bytes(png([255, 32, 32, 255]))
        editor.wait(lambda: red_pixels() > before+100, seconds=15, description="External PNG refresh updates the actual scene pixels")
        editor.shot("reimported_art")
        assert manifest.read_bytes() == original_manifest, "File refresh modified the project document"
        assert tomllib.loads(manifest.read_text())["scenes"]["main"]["entities"][1]["components"][1]["texture"] == "art/hero.png"
        artwork.write_bytes(original_art)
        editor.wait(lambda: red_pixels() <= before+10, description="Restored texture refreshes without reopening")
        source = (project/"scripts/main.ks").read_text()
        code = source.replace("let speed = 120.0;", """enum Stage { Ready, Waiting }
if !state_get("task-started",false) {
    task_schedule("stage",0.0,Stage.Ready);
    timer_start("cooldown",0.0);
    state_set("task-started",true);
}
if task_ready("stage") {
    state_set("task-ok",task_take("stage",Stage.Waiting)==Stage.Ready);
}
if timer_finished("cooldown") {state_set("timer-ok",true);}
let movement = dict_new();
for (let i=0; i<4; i+=1) {
    if i==1 { continue; }
    movement = dict_set(movement, "speed", dict_get(movement, "speed", 0.0)+40.0);
}
let speed = 0.0;
if state_get("task-ok",false) && state_get("timer-ok",false) {
    speed = dict_get(movement, "speed", 0.0);
}""")
        assert code != source
        draft = run/"authored-main.ks"; draft.write_text(code)
        editor.panel(118, 68); editor.enlarge()
        editor.source("scripts/main.ks", draft)
        editor.shot("highlighted_counted_loop_dictionary")
        editor.save()
        assert (project/"scripts/main.ks").read_text() == code
        result = {"passed": True, "external_reimport_scene_pixels": True,
                  "stable_reference_and_untouched_manifest": True,
                  "script_authored_through_widgets": True, "project": str(project),
                  "scope": "Real editor under Xvfb/software graphics; exported play is recorded separately"}
        (output/"result.json").write_text(json.dumps(result, indent=2)+"\n")
    except Exception as error:
        record_failure(output,"structured-scripting","authoring",error)
        try:
            if editor.process and editor.process.poll() is None:
                editor.shot("failure")
        except Exception as capture_error:
            (output/"capture-error.txt").write_text(str(capture_error))
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)
    if args.web_runtime:
        result["passed"] = False
        (output/"result.json").write_text(json.dumps(result, indent=2)+"\n")
        env = dict(os.environ, WEB_SMOKE_PROJECT=str(project))
        played=subprocess.run(["node", str(REPO/"tests/web/smoke.cjs"), str(args.web_runtime.resolve())],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (output/"browser-play.log").write_text(played.stdout)
        print(played.stdout,end="",flush=True)
        if played.returncode:
            error=RuntimeError(played.stdout[-4000:])
            record_failure(output,"structured-scripting","browser",error)
            raise error
        result["browser_played_authored_source"] = True
        result["passed"] = True
        (output/"result.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
