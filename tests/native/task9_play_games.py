#!/usr/bin/env python3
"""Play editor exports from unrelated cwd with their source projects unavailable."""
import argparse
import json
from pathlib import Path
import subprocess
import time
from task9_editor_games import Editor


class Player(Editor):
    def start(self):
        self.wait(lambda: subprocess.run([self.xdo,'getdisplaygeometry'],env=self.env,capture_output=True).returncode==0)
        return self.launch()

    def launch(self):
        self.process=subprocess.Popen([str(self.tools/'game')],cwd=self.output,env=self.env,
                                      stdout=(self.output/'player.log').open('a'),stderr=subprocess.STDOUT)
        def find():
            result=subprocess.run([self.xdo,'search','--name','Kiln Game'],env=self.env,text=True,capture_output=True)
            return result.stdout.strip().splitlines()[0] if result.returncode==0 else None
        self.window=self.wait(find)
        self.command('windowfocus','--sync',self.window)
        time.sleep(.5)
        return self

    def log(self):
        return (self.output/'player.log').read_text()

    def saved(self):
        candidates=list((self.output/'profile/data').rglob('slot_0.json'))
        if not candidates:
            return None
        return json.loads(json.loads(candidates[0].read_text())['data'])

    def stop_player(self):
        if self.process and self.process.poll() is None:
            self.process.terminate();self.process.wait(timeout=10)
        self.process=None

    def close(self):
        self.stop_player();self.server.terminate();self.server.wait(timeout=10)
        (self.output/'interactions.json').write_text(json.dumps(self.actions,indent=2)+'\n')


def play(name,root,evidence):
    player=Player(root,root,evidence,':81').start()
    result={'game':name,'standalone_without_archive_argument':True,'source_project_unavailable':True,'tools_absent_from_path':True}
    try:
        player.shot('title')
        if name=='skybridge':
            player.key('Return');time.sleep(.5)
            player.command('keydown','Right')
            for _ in range(12):
                player.press('Up',.12);time.sleep(.5)
                if 'SKYBRIDGE_LEVEL_TWO' in player.log():break
            player.command('keyup','Right')
            assert 'SKYBRIDGE_LEVEL_TWO' in player.log(),'First real gate did not transition'
            player.shot('level_two')
            player.press('s')
            snapshot=player.wait(player.saved)
            assert snapshot['scene']['id']=='scene_1'
            assert snapshot['game_data']['stage']=={'Float':2.0}
            def hero_x(data):
                entity=next(entity for entity in data['scene']['entities'] if entity['id']=='hero')
                return next(component['x'] for component in entity['components'] if component['type']=='Transform')
            before=snapshot
            player.press('Right',.4);player.press('l');time.sleep(.25)
            player.press('s')
            resumed=player.wait(lambda:player.saved() if player.saved() and player.saved()['root']['frame']>before['root']['frame'] else None)
            assert abs(hero_x(resumed)-hero_x(before))<4, 'Load did not restore the saved player position'
            player.command('keydown','Right')
            for _ in range(14):
                player.press('Up',.12);time.sleep(.5)
                if 'SKYBRIDGE_WON' in player.log():break
            player.command('keyup','Right')
            assert 'SKYBRIDGE_WON' in player.log(),'Second real gate did not win'
            player.press('s')
            snapshot=player.wait(lambda:player.saved() if player.saved() and player.saved()['game_data']['won']=={'Boolean':True} else None)
            player.shot('won')
            result.update(two_physical_levels_won=True,jumping_and_animation=True,saved_stage=snapshot['game_data']['stage'],save_load_completed=True)
        elif name=='switchyard':
            player.click(320,180)
            for stage,solution in [(1,[0,1,2]),(2,[1,2]),(3,[0,2])]:
                selected=0
                for switch in solution:
                    while selected!=switch:
                        player.press('Right');selected=(selected+1)%3
                    player.click(320,276)
                player.wait(lambda:player.log().count('SWITCHYARD_WON')>=stage)
                player.shot('circuit_'+str(stage)+'_won')
                if stage<3:player.click(320,180)
            snapshot=player.wait(player.saved)
            assert snapshot['game_data']['stage']=={'Integer':3}
            assert snapshot['game_data']['moves']=={'Integer':2}
            player.stop_player();player.launch();player.click(320,276)
            player.wait(lambda:'SWITCHYARD_RESUMED' in player.log())
            player.shot('fresh_process_continue')
            result.update(three_circuits_won=True,saved_moves=2,fresh_process_continue=True)
        else:
            player.click(320,180)
            for _ in range(16):
                player.click(320,276);time.sleep(.7)
                if player.log().count('EMBERWATCH_HIT')>=6:break
            assert player.log().count('EMBERWATCH_HIT')>=6,'Projectiles did not defeat all hostiles'
            player.shot('hostiles_cleared')
            player.press('Right',2.8)
            player.wait(lambda:'EMBERWATCH_WON' in player.log())
            snapshot=player.wait(player.saved)
            assert snapshot['game_data']['outcome']=={'Text':'won'}
            assert snapshot['game_data']['cleared']=={'Integer':3}
            player.shot('relay_secured')
            result.update(all_hostiles_defeated=True,relay_trigger_won=True,victory_saved=True)
        log=player.log()
        for error in ('panicked at','scene script failed','validation error'):
            assert error not in log.lower(),error
    finally:
        player.close()
    return result


def main():
    parser=argparse.ArgumentParser();parser.add_argument('exports',type=Path);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();report=json.loads(args.exports.read_text());results=[]
    for item in report['games']:
        exported=item['export'];name=exported['game']
        project=Path(exported['opened_saved_project'])
        hidden=project.with_name(project.name+'-source-unavailable')
        project.rename(hidden)
        try:
            result=play(name,Path(exported['outputs']['native']),args.output/name)
            results.append(result);print(name+' standalone gameplay passed',flush=True)
            args.output.mkdir(parents=True,exist_ok=True)
            (args.output/'result.json').write_text(json.dumps({'scope':'Xvfb/SwiftShader/ALSA null; no physical-device claim','games':results},indent=2)+'\n')
        finally: hidden.rename(project)


if __name__=='__main__':main()
