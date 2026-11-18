#!/usr/bin/env python3
"""Build a saved atlas, render its clip, rebuild changed art and reopen the recipe."""
import json
import os
from pathlib import Path
import shutil
import struct
import sys
import tempfile
import tomllib
import zlib
from task9_editor_games import Editor, REPO, color_matches, record_failure


def png(color):
    def chunk(kind, data):
        return struct.pack('>I', len(data))+kind+data+struct.pack('>I', zlib.crc32(kind+data))
    rows = (b'\0'+bytes(color)*16)*16
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB',16,16,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')


def main():
    output = Path(os.environ.get('KILN_ATLAS_EVIDENCE', REPO/'target/a03-atlas-recipe')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/vertical_slice', project)
    source = project/'art/hero.png'; source.write_bytes(png([255,0,0,255]))
    manifest = project/'kiln.toml'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')

    def saved():
        return tomllib.loads(manifest.read_text())

    def field(x, y, value):
        editor.held_click(x,y); editor.key('ctrl+a')
        editor.command('type','--clearmodifiers','--delay','2',str(value))

    def references_ready():
        width, height, frame=editor.frame()
        assert (width,height)==(1440,900)
        at=(487*width+1400)*3
        return color_matches(frame[at:at+3],(27,27,27))

    def pixels(channel):
        width, height, frame = editor.frame()
        assert (width,height) == (1440,900)
        return sum(1 for y in range(175,650) for x in range(400,1100)
                   if frame[(y*width+x)*3+channel] > 180
                   and all(frame[(y*width+x)*3+c] < 90 for c in range(3) if c != channel))

    def open_atlas(load=False):
        editor.panel(80,578); editor.enlarge(); editor.held_click(170,166)
        if load:
            editor.held_click(520,238); editor.held_click(490,266)

    def close_atlas():
        editor.held_click(966,180); editor.held_click(125,132)

    try:
        editor.start(); open_atlas()
        editor.value(490,323,'64'); editor.held_click(500,436); editor.held_click(525,502)
        editor.wait(lambda:(project/'art/atlas.png').exists(), description='Build generated PNG through atlas widgets')
        editor.panel(18,102)
        recipe = saved()['atlases']['atlas']
        assert recipe['sources'] == {'hero':'art/hero.png'} and recipe['size'] == 64
        assert recipe['built_regions']['hero'] == [2,2,16,16]
        assert saved()['animation_clips']['atlas:atlas:hero']['frames'][0]['rect'] == [2,2,16,16]
        identity = saved()['asset_ids']['art/atlas.png']
        editor.shot('recipe_built'); close_atlas()
        editor.held_click(100,196); editor.held_click(1200,528)
        editor.wait(references_ready,description='Render sprite reference fields before editing')
        field(1280,487,'art/atlas.png'); field(1280,627,'atlas:atlas:hero')
        editor.command('mousemove','--window',editor.window,1320,610,'click','--repeat','2','--delay','70','5')
        editor.held_click(1180,521); editor.panel(18,102)
        player = next(e for e in saved()['scenes']['main']['entities'] if e['id']=='player')
        sprite = next(c for c in player['components'] if c['type']=='Sprite')
        assert sprite['texture']=='art/atlas.png' and sprite['animation_clip']=='atlas:atlas:hero'
        editor.held_click(280,113)  # Hide selection gizmos before classifying scene pixels.
        red = editor.wait(lambda:pixels(0) if pixels(0)>200 else None, description='Render generated static clip, excluding atlas padding')
        editor.shot('generated_clip_rendered_red')
        source.write_bytes(png([0,0,255,255]))
        assert pixels(0)>200, 'Source reimport changed the baked texture before rebuild'
        open_atlas(load=True); editor.held_click(525,502); editor.panel(18,102); close_atlas()
        editor.wait(lambda:pixels(2)>200 and pixels(0)<red/2, description='Atlas rebuild refreshes actual scene pixels')
        assert saved()['asset_ids']['art/atlas.png']==identity
        assert saved()['atlases']['atlas']['built_regions']==recipe['built_regions']
        assert saved()['atlases']['atlas']['built_hash']!=recipe['built_hash']
        editor.shot('rebuilt_clip_rendered_blue')
        # Move the source through the same typed asset operation used by authors.
        editor.held_click(65,688); editor.panel(80,96); editor.large=False; editor.enlarge()
        editor.held_click(1055,166); editor.panel(18,102)
        source_id=saved()['asset_ids']['art/hero.png']
        editor.held_click(110,220); editor.held_click(528,220); editor.held_click(960,166)
        field(980,177,'art/hero-renamed.png'); editor.held_click(872,203); editor.panel(18,102)
        editor.wait(lambda:saved()['atlases']['atlas']['sources']['hero']=='art/hero-renamed.png',
                    description='Asset move updates saved recipe source without changing label')
        assert saved()['asset_ids']['art/hero-renamed.png']==source_id
        assert not source.exists(); source=project/'art/hero-renamed.png'
        source.write_bytes(png([0,255,0,255])); editor.held_click(90,132)
        green_before=pixels(1)
        open_atlas(load=True); editor.held_click(525,502); editor.panel(18,102); close_atlas()
        editor.wait(lambda:pixels(1)>green_before+200 and pixels(2)<100,
                    description='Rebuild follows renamed source and renders new pixels')
        assert saved()['asset_ids']['art/atlas.png']==identity
        assert saved()['atlases']['atlas']['built_regions']==recipe['built_regions']
        editor.shot('renamed_source_rebuilt_green')
        expected=saved(); sources={p.relative_to(project):p.read_bytes() for p in project.rglob('*.ks')}
        editor.close(); editor=Editor(REPO/'target/debug',project,output/'reopened',':86')
        editor.start(); editor.panel(18,102); assert saved()==expected
        editor.wait(lambda:pixels(1)>green_before+200 and pixels(2)<100,description='Restart renders rebuilt clip')
        open_atlas(load=True); editor.held_click(525,502); editor.panel(18,102)
        assert saved()==expected, 'Reloaded recipe rebuild changed stable regions, identities or properties'
        assert {p.relative_to(project):p.read_bytes() for p in project.rglob('*.ks')}==sources
        editor.shot('saved_recipe_rebuilt_after_restart'); close_atlas()
        result=dict(passed=True, widget_recipe_build=True, generated_clip_pixels=True, changed_source_rebuild_pixels=True,
                    stable_output_identity_and_regions=True, widget_source_rename=True, exact_recipe_restart_rebuild=True, source_bytes_preserved=True,
                    project=str(project), scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output,'atlas-recipe','widgets',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':
    main()
