#!/usr/bin/env python3
"""Generate bounded, source-only A05 offscreen/capacity projects; no binaries checked in."""
import argparse
from pathlib import Path


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    parser.add_argument('--count',type=int,default=16448)
    parser.add_argument('--visible',action='store_true')
    args=parser.parse_args()
    if not 1<=args.count<=65536:parser.error('count must be 1..65536')
    args.output.mkdir(parents=True,exist_ok=False)
    with (args.output/'kiln.toml').open('w') as out:
        out.write('id="sprite-stress"\nname="Sprite Stress"\nversion="0.1.0"\nfeatures=[]\n[window]\nwidth=320\nheight=240\n[scenes.main]\nid="main"\nname="Sprite capacity"\nnested_scenes=[]\n')
        def sprite(name,x,y,color,size):
            out.write(f'\n[[scenes.main.entities]]\nid="{name}"\nname="{name}"\ncomponents=[{{type="Transform",x={x}.0,y={y}.0,rotation=0.0,scale=1.0}},{{type="Sprite",color={color},width={size}.0,height={size}.0,layer=0.0}}]\n')
        for n in range(args.count):sprite(f'background-{n}',160 if args.visible else 10000,120 if args.visible else 10000,[0.2,0.2,0.2,1.0],2)
        sprite('actor',160,120,[0.0,1.0,0.0,1.0],64)
    print(args.output)


if __name__=='__main__':main()
