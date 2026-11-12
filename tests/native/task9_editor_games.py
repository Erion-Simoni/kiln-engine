#!/usr/bin/env python3
"""Reproduce Task 9 game authoring using real editor widgets and normal clipboard.

Creates game manifests only through Save As and editor authoring controls. Source
text and ordinary artwork come from the checked-in games. No engine test hooks.
Requires Linux/Xvfb/xdotool/ImageMagick and built editor tools. Outputs stay outside Git.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import tomllib

from ui_smoke import REPO, TOOLS, executable
from export_notices import verify_export_notices

GAMES = ('emberwatch', 'switchyard', 'skybridge')


class Editor:
    def __init__(self, tools, project, output, display=':83'):
        self.tools, self.project, self.output = Path(tools).resolve(), Path(project).resolve(), Path(output).resolve()
        self.output.mkdir(parents=True, exist_ok=True)
        self.env = os.environ.copy()
        run = self.output/'profile'
        for folder in ('runtime', 'config/kiln'):
            (run/folder).mkdir(parents=True, exist_ok=True)
        (run/'runtime').chmod(0o700)
        (run/'alsa.conf').write_text('pcm.!default { type null }\n')
        (run/'config/kiln/project_hub.json').write_text(json.dumps([
            {'path': str(self.project/'kiln.toml'), 'name': 'Reference game', 'last_opened': 0}]))
        self.env.update(DISPLAY=display, PATH='', XDG_DATA_HOME=str(run/'data'),
                        XDG_CONFIG_HOME=str(run/'config'), XDG_CACHE_HOME=str(run/'cache'),
                        XDG_RUNTIME_DIR=str(run/'runtime'), LIBGL_ALWAYS_SOFTWARE='1',
                        ALSA_CONFIG_PATH=str(run/'alsa.conf'),
                        RUST_LOG='warn,kiln_editor=info,kiln_runtime::slice=info')
        if TOOLS.is_dir():
            self.env['LD_LIBRARY_PATH'] = str(TOOLS/'usr/lib/x86_64-linux-gnu')+':'+self.env.get('LD_LIBRARY_PATH', '')
        if Path('/usr/lib/chromium/vk_swiftshader_icd.json').exists():
            self.env['VK_ICD_FILENAMES'] = '/usr/lib/chromium/vk_swiftshader_icd.json'
        self.xdo = executable('xdotool')
        if Path('/tmp/.X11-unix', 'X'+display[1:]).exists():
            raise RuntimeError('Choose an unused display: '+display)
        self.server = subprocess.Popen([executable('Xvfb'), display, '-screen', '0', '1440x900x24', '-nolisten', 'tcp', '-ac'], env=self.env,
                                       stdout=(self.output/'xvfb.log').open('w'), stderr=subprocess.STDOUT)
        self.process = None
        self.window = None
        self.actions = []
        self.large = False

    def diagnostic(self):
        log=self.output/('player.log' if (self.output/'player.log').exists() else 'editor.log')
        return log.read_text()[-4000:] if log.exists() else 'No process log yet'

    def wait(self, predicate, seconds=15):
        until = time.monotonic()+seconds
        while time.monotonic()<until:
            value = predicate()
            if value:
                return value
            if self.process and self.process.poll() is not None:
                raise RuntimeError('Editor exited: '+self.diagnostic())
            time.sleep(.1)
        raise RuntimeError('Editor operation timed out: '+self.diagnostic())

    def start(self):
        self.wait(lambda: subprocess.run([self.xdo, 'getdisplaygeometry'], env=self.env, capture_output=True).returncode == 0)
        self.process = subprocess.Popen([str(self.tools/'kiln-editor')], cwd=self.output, env=self.env,
                                        stdout=(self.output/'editor.log').open('w'), stderr=subprocess.STDOUT)
        def find():
            result = subprocess.run([self.xdo, 'search', '--name', 'Kiln Editor'], env=self.env, capture_output=True, text=True)
            return result.stdout.strip().splitlines()[0] if result.returncode == 0 else None
        self.window = self.wait(find)
        self.command('windowfocus', '--sync', self.window)
        time.sleep(.7)
        return self

    def command(self, *args):
        subprocess.run([self.xdo, *map(str, args)], env=self.env, check=True, timeout=20)
        self.actions.append([str(value) for value in args])
        time.sleep(.10)

    def click(self, x, y):
        self.command('mousemove', '--window', self.window, x, y, 'click', '1')

    def key(self, key):
        self.command('key', '--clearmodifiers', key)

    def text(self, x, y, value):
        self.click(x, y); self.key('ctrl+a')
        self.command('type', '--clearmodifiers', '--delay', '2', str(value))

    def value(self, x, y, value):
        self.command('mousemove', '--window', self.window, x, y, 'click', '--repeat', '2', '--delay', '80', '1')
        self.key('ctrl+a'); self.command('type', '--clearmodifiers', str(value)); self.key('Return')

    def press(self, key, seconds=.18):
        self.command('keydown', key); time.sleep(seconds); self.command('keyup', key)
        # Leave a visible fixed-tick release before another axis-edge press.
        time.sleep(.25)

    def shot(self, name):
        time.sleep(.2)
        subprocess.run([executable('import'), '-display', self.env['DISPLAY'], '-window', self.window, str(self.output/(name+'.png'))], env=self.env, check=True, timeout=20)

    def panel(self, menu, row):
        self.click(menu, 12); self.click(menu+25, row)

    def enlarge(self):
        if self.large:
            return
        self.command('mousemove', '--window', self.window, 365, 512, 'mousedown', '1')
        for x, y in ((500,580),(700,680),(900,770),(1100,850)):
            self.command('mousemove', '--window', self.window, x, y)
        self.command('mouseup', '1')
        self.large = True

    def paste(self, source):
        clip = subprocess.Popen([sys.executable, str(Path(__file__).with_name('x11_clipboard.py')), str(source)], env=self.env,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            if 'Clipboard ready' not in clip.stdout.readline():
                raise RuntimeError('Cannot set editor clipboard: '+clip.stderr.read())
            self.click(200, 260); self.key('ctrl+a'); self.key('ctrl+v'); time.sleep(.3)
            self.actions.append(['clipboard-paste', str(source), hashlib.sha256(Path(source).read_bytes()).hexdigest()])
        finally:
            clip.terminate(); clip.wait(timeout=5)

    def source(self, relative, source):
        self.text(225,161,relative); self.click(392,161); self.paste(source)
        self.click(65,187); self.click(208,187)
        self.wait(lambda: (self.project/relative).exists() and (self.project/relative).read_bytes()==Path(source).read_bytes())

    def save(self):
        self.click(500,78); self.key('ctrl+s'); time.sleep(.2)

    def close(self):
        try:
            if self.process and self.process.poll() is None:
                self.panel(18,245)
                self.process.wait(timeout=10)
                if self.process.returncode != 0:
                    raise RuntimeError('Editor normal close failed')
        finally:
            if self.process and self.process.poll() is None:
                self.process.terminate(); self.process.wait(timeout=10)
            self.server.terminate(); self.server.wait(timeout=10)
            (self.output/'interactions.json').write_text(json.dumps(self.actions,indent=2)+'\n')


def author(tools, name, destination, evidence):
    base = REPO/('samples/physics2d_editor' if name=='skybridge' else 'samples/ui2d_editor')
    original = (base/'kiln.toml').read_bytes()
    canonical = REPO/'games'/name
    expected = tomllib.loads((canonical/'kiln.toml').read_text())
    editor = Editor(tools, base, evidence).start()
    try:
        editor.panel(18,120)
        editor.text(160,153,str(destination/'kiln.toml')); editor.click(102,178)
        editor.wait(lambda:(destination/'kiln.toml').exists())
        editor.project = destination
        editor.panel(118,42)
        editor.text(191,213,expected['name']); editor.text(177,238,expected['id']); editor.click(94,128); editor.save()
        editor.panel(118,68); editor.enlarge()
        for path in ('scripts/main.ks', 'scripts/hero.ks' if name=='skybridge' else 'scripts/ui.ks'):
            editor.source(path,canonical/path)
        editor.shot('source_saved'); editor.click(114,128)
        if name=='skybridge':
            (destination/'art').mkdir(exist_ok=True)
            shutil.copyfile(canonical/'art/hero.png',destination/'art/hero.png')
            editor.panel(80,173); editor.click(166,206)
            editor.text(180,233,'run'); editor.text(225,259,'art/hero.png'); editor.click(56,284)
            time.sleep(.4)  # Wait for the New Clip form to collapse before targeting frame controls.
            for attempt in range(3):
                editor.click(68,301); editor.save()
                count=len(tomllib.loads((destination/'kiln.toml').read_text())['animation_clips']['run']['frames'])
                if count==2:break
                if count!=1:raise RuntimeError('Unexpected animation frame count: '+str(count))
                time.sleep(.4)
            if count!=2:raise RuntimeError('Add Frame never produced the second saved frame')
            editor.value(52,354,16)
            editor.shot('animation_authored'); editor.click(140,128)
            editor.panel(80,615)
            editor.text(240,215,'Lower Gantry'); editor.text(240,240,'scripts/main.ks'); editor.click(114,128)
            editor.click(100,187); editor.click(1200,505)
            editor.text(1280,466,'art/hero.png'); editor.text(1280,600,'run'); editor.click(1177,648)
            editor.save(); editor.click(385,140)
            editor.panel(80,615); editor.text(240,215,'Upper Gantry'); editor.click(114,128)
            editor.click(100,187)
            editor.command('mousemove','--window',editor.window,1320,610,'click','--repeat','9','--delay','70','5')
            editor.text(1270,438,'stage'); editor.click(1199,464); editor.value(1208,439,2)
            editor.click(100,265)
            editor.command('mousemove','--window',editor.window,1320,400,'click','--repeat','12','--delay','50','4')
            editor.value(1262,195,1100); editor.save()
            editor.panel(80,485)
            # Input Rebinding sorts actions by stable ID.
            editor.click(80,206); editor.click(148,225); editor.press('s')
            editor.click(80,295); editor.click(148,313); editor.press('l')
            editor.click(80,383); editor.click(148,401); editor.press('r')
            editor.click(136,128); editor.click(459,140)
        editor.save()
        actual=tomllib.loads((destination/'kiln.toml').read_text())
        if actual!=expected:
            (evidence/'actual-project.json').write_text(json.dumps(actual,indent=2))
            (evidence/'expected-project.json').write_text(json.dumps(expected,indent=2))
            raise RuntimeError(name+' UI authoring did not reproduce the checked-in project')
        assert (base/'kiln.toml').read_bytes()==original, 'Source template changed'
        editor.shot('saved_project')
    finally:
        editor.close()
    return {'game':name,'project':str(destination),'manifest_created_by':'native editor Save As and real authoring widgets','source_pasted_and_byte_verified':True,'project_matches_checked_in_game':True,'template_unchanged':True}


def export(tools, name, project, evidence):
    editor=Editor(tools,project,evidence,':82').start()
    outputs={}
    try:
        editor.panel(80,199); editor.enlarge()
        for web in (False,True):
            output=evidence/('browser game' if web else 'native game')
            if web: editor.click(155,199)
            successes=(evidence/'editor.log').read_text().count('Export result: Success')
            editor.text(250,257,str(output)); editor.key('Return'); editor.click(75,326)
            editor.wait(lambda:(output/'export.json').exists(),60)
            editor.wait(lambda:(evidence/'editor.log').read_text().count('Export result: Success')>successes,60)
            verify_export_notices(tools,output,web)
            editor.shot('browser_exported' if web else 'native_exported')
            outputs['web' if web else 'native']=str(output)
            # This attempt must fail without changing the successful artifact.
            before=(output/'export.json').read_bytes()
            old=(evidence/'editor.log').read_text().count('Export result: Failed')
            editor.click(75,326)
            editor.wait(lambda:(evidence/'editor.log').read_text().count('Export result: Failed')>old)
            assert (output/'export.json').read_bytes()==before
        editor.click(122,128)  # Close the Build / Export tab.
    finally:
        editor.close()
    return {'game':name,'opened_saved_project':str(project),'outputs':outputs,'native_browser_notices_verified':True,'overwrite_preserved':True,'tools_absent_from_child_path':True}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--tools',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--author',action='store_true')
    args=parser.parse_args()
    root=args.output.resolve(); root.mkdir(parents=True,exist_ok=True)
    result={'scope':'Xvfb software GUI authoring/export; not physical devices','games':[]}
    for name in GAMES:
        destination=root/'projects'/name
        if args.author:
            destination.parent.mkdir(parents=True,exist_ok=True)
            authored=author(args.tools.resolve(),name,destination,root/name/'authoring')
        else:
            shutil.copytree(REPO/'games'/name,destination)
            authored=None
        exported=export(args.tools.resolve(),name,destination,root/name/'export')
        result['games'].append({'authoring':authored,'export':exported})
        (root/'result.json').write_text(json.dumps(result,indent=2)+'\n')
        print(name+' authoring/export passed',flush=True)
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
