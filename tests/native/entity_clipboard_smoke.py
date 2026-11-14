#!/usr/bin/env python3
"""Exercise real entity Copy/Paste, two scenes, undo/redo and disk restart."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib
from task9_editor_games import Editor, REPO


def main():
    output = Path(os.environ.get("KILN_CLIPBOARD_EVIDENCE_DIR", REPO/"target/a02-entity-clipboard")).resolve()
    output.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="run-", dir=output))
    project = run/"project"
    shutil.copytree(REPO/"samples/ui2d_editor", project)
    manifest = project/"kiln.toml"
    original = tomllib.loads(manifest.read_text())
    editor = Editor(REPO/"target/debug", project, output/"editor", ":84")
    def saved():
        return tomllib.loads(manifest.read_text())
    def count_main(count):
        return len(saved()["scenes"]["main"]["entities"]) == count
    try:
        editor.start()
        editor.held_click(100, 166)  # Player in the actual hierarchy.
        editor.key("ctrl+c")
        editor.key("ctrl+v")
        editor.save()
        editor.wait(lambda: count_main(3), description="Paste and save an entity")
        pasted = saved()["scenes"]["main"]["entities"][-1]
        assert pasted["name"] == "Player Copy"
        assert pasted["id"] not in {entity["id"] for entity in original["scenes"]["main"]["entities"]}
        transform = next(c for c in pasted["components"] if c["type"] == "Transform")
        assert (transform["x"], transform["y"]) == (56.0, 76.0)
        editor.shot("same_scene_pasted")
        editor.key("ctrl+z"); editor.save()
        editor.wait(lambda: count_main(2), description="Undo the entire paste")
        editor.key("ctrl+y"); editor.save()
        editor.wait(lambda: count_main(3), description="Redo the same entity identity")
        assert saved()["scenes"]["main"]["entities"][-1] == pasted
        editor.held_click(261, 145)  # Add an actual second scene.
        editor.key("ctrl+v"); editor.save()
        editor.wait(lambda: len(saved()["scenes"]) == 2 and any(len(s["entities"]) == 1 for key, s in saved()["scenes"].items() if key != "main"), description="Paste into another scene")
        before_restart = saved()
        other = next(scene for key, scene in before_restart["scenes"].items() if key != "main")
        assert other["entities"][0]["id"] != pasted["id"]
        assert other["entities"][0]["components"] == pasted["components"]
        editor.shot("second_scene_pasted")
        editor.close()
        editor = Editor(REPO/"target/debug", project, output/"reopened", ":84")
        editor.start(); editor.save()
        assert saved() == before_restart, "Restart changed supported project properties"
        editor.shot("reopened")
        result = {"passed": True, "same_scene_paste": True, "undo_redo_identity": True,
                  "cross_scene_unique_identity": True, "save_restart_properties_equal": True,
                  "scope": "Real editor widgets and clipboard under Xvfb/software graphics"}
        (output/"result.json").write_text(json.dumps(result, indent=2)+"\n")
        print(json.dumps(result))
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == "__main__":
    main()
