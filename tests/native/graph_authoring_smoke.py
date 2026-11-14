#!/usr/bin/env python3
"""Author a live gameplay graph through widgets; verify history, save, Play and restart."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from task9_editor_games import Editor, REPO, color_matches, executable, record_failure


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--web-runtime',type=Path)
    args=parser.parse_args()
    output=Path(os.environ.get('KILN_GRAPH_EVIDENCE',REPO/'target/a04-graph-authoring')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));project=run/'project'
    shutil.copytree(REPO/'samples/vertical_slice',project)
    source=project/'scripts/gameplay_graph.ks';manifest=project/'kiln.toml'
    editor=Editor(REPO/'target/debug',project,output/'editor',':85')
    def save_graph():editor.held_click(535,194)
    def constant_dialog_visible():
        capture=subprocess.check_output([executable('import'),'-display',editor.env['DISPLAY'],'-window',editor.window,'-depth','8','ppm:-'],env=editor.env,timeout=10)
        header,pixels=capture.split(b'\n255\n',1)
        width,height=map(int,header.splitlines()[-1].split());assert (width,height)==(1440,900)
        def pixel(x,y):return pixels[(y*width+x)*3:(y*width+x)*3+3]
        return color_matches(pixel(580,475),(47,51,59)) and color_matches(pixel(880,510),(35,38,44))
    def edit_constant(value,accept=True):
        editor.held_click(200,550);editor.held_click(465,262)
        editor.wait(constant_dialog_visible,description='Render constant parameter dialog before editing')
        editor.value(570,475,str(value));editor.held_click(565 if accept else 618,513)
    try:
        editor.start();editor.panel(80,180);editor.enlarge();editor.held_click(450,194)
        editor.held_click(150,262);editor.held_click(320,262)
        edit_constant(5)
        editor.held_click(210,696);editor.held_click(465,262);editor.held_click(600,461)
        editor.held_click(600,630);editor.held_click(565,499)
        # Value -> registered add_score argument; done -> graph Output.
        editor.held_click(292,506);editor.held_click(132,726)
        editor.held_click(292,766);editor.held_click(482,406);save_graph()
        editor.wait(lambda:source.exists() and 'add_score(' in source.read_text(),description='Save connected real host-call graph')
        initial=source.read_bytes();editor.shot('connected_graph')
        edit_constant(77,False);save_graph();assert source.read_bytes()==initial,'Cancel changed graph parameters'
        edit_constant(9);save_graph()
        editor.wait(lambda:source.read_bytes()!=initial,description='Commit graph constant edit')
        revised=source.read_bytes()
        editor.held_click(610,194);save_graph();editor.wait(lambda:source.read_bytes()==initial,description='Undo graph parameter edit')
        editor.held_click(663,194);save_graph();editor.wait(lambda:source.read_bytes()==revised,description='Redo graph parameter edit')
        # Multiple motion frames must form one undo gesture.
        editor.command('mousemove','--window',editor.window,200,495,'mousedown','1')
        for x,y in [(240,480),(280,465),(350,450)]:editor.command('mousemove','--window',editor.window,x,y)
        editor.command('mouseup','1');save_graph()
        editor.wait(lambda:source.read_bytes()!=revised,description='Drag graph node over multiple frames')
        editor.held_click(610,194);save_graph();editor.wait(lambda:source.read_bytes()==revised,description='Undo the complete drag gesture')
        editor.shot('history_and_cancel_verified')
        editor.held_click(95,166);editor.save()
        editor.wait(lambda:tomllib.loads(manifest.read_text())['scenes']['main']['script']=='scripts/gameplay_graph.ks',description='Assign saved graph to active scene')
        editor.held_click(125,132);editor.held_click(500,78);editor.held_click(678,46);time.sleep(.4);editor.key('F5')
        slot=project/'.kiln-save/slot_0.json'
        editor.wait(slot.exists,description='Save actual running graph gameplay')
        saved=json.loads(json.loads(slot.read_text())['data']);assert saved['sample_compatibility']['score']>=9
        assert saved['sample_compatibility']['score']%9==0
        assert any(value=={'Integer':9} for name,value in saved['root']['values'].items() if name.startswith('__kiln_graph:'))
        editor.shot('actual_graph_play');editor.held_click(678,46);editor.save()
        expected_manifest=manifest.read_bytes();expected_source=source.read_bytes();editor.close()
        editor=Editor(REPO/'target/debug',project,output/'reopened',':85');editor.start()
        editor.panel(80,180);editor.enlarge();editor.held_click(450,194);editor.shot('reopened_graph')
        assert source.read_bytes()==expected_source;assert manifest.read_bytes()==expected_manifest
        editor.close()
        result=dict(passed=True,widget_connections=True,real_host_arguments=True,cancel_preserves_values=True,
                    undo_redo=True,drag_is_one_undo=True,play_score=saved['sample_compatibility']['score'],
                    live_node_values=True,restart_equal=True,project=str(project))
        if args.web_runtime:
            env=dict(os.environ,WEB_SMOKE_PROJECT=str(project),WEB_SMOKE_GRAPH_PROJECT_ID=tomllib.loads(manifest.read_text())['id'])
            played=subprocess.run(['node',str(REPO/'tests/web/smoke.cjs'),str(args.web_runtime.resolve())],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            (output/'browser-play.log').write_text(played.stdout);print(played.stdout,end='',flush=True)
            if played.returncode:raise RuntimeError(played.stdout[-4000:])
            result['browser_graph_save_restore']=True
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'graph-authoring','widgets/browser',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
