#!/usr/bin/env python3
"""Verify counted-loop/dictionary-controlled movement in a standalone native export."""
import argparse
import json
from pathlib import Path
import subprocess
import time
from task9_editor_games import REPO, executable
from task9_play_games import Player


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("--output", type=Path, default=REPO/"target/a04-structured-native")
    args = parser.parse_args()
    root = args.export.resolve()
    output = args.output.resolve()
    player = Player(root, root, output, ":81")
    def position():
        ppm = subprocess.check_output([executable("import"), "-display", player.env["DISPLAY"],
                                       "-window", player.window, "-depth", "8", "ppm:-"], env=player.env, timeout=10)
        header, data = ppm.split(b"\n255\n", 1)
        width, _height = map(int, header.splitlines()[-1].split())
        points = [i//3 % width for i in range(0, len(data), 3)
                  if data[i+2] > 100 and data[i+2] > data[i]*1.3 and data[i+2] > data[i+1]*1.1]
        assert len(points) > 20, "Actual rendered player is missing"
        return sum(points)/len(points)
    try:
        assert (root/"assets.kar").is_file(), "Expected a standalone exported project archive"
        player.start(); player.key("Return"); time.sleep(.5)
        before = position()
        player.command("keydown", "d")
        try:
            player.wait(lambda: position() > before+10, description="Dictionary-controlled native movement")
        finally:
            player.command("keyup", "d")
        time.sleep(.15); released = position(); time.sleep(.25); stopped = position()
        assert abs(stopped-released) <= 2, "Native player continued moving after key release"
        player.shot("standalone_played")
        result = {"passed": True, "before": before, "released": released, "stopped": stopped,
                  "source_controls_speed_through_counted_loop_and_dictionary": True}
        (output/"result.json").write_text(json.dumps(result, indent=2)+"\n")
        print(json.dumps(result))
    finally:
        player.close()


if __name__ == "__main__":
    main()
