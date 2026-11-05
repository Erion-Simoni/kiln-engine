"""Recreate the tiny original showcase sprites using only Python's stdlib."""
from pathlib import Path
import struct
import zlib


def png(name, width, height, pixel):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + bytes(c for x in range(width) for c in pixel(x, y)) for y in range(height))
    data = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
    Path(__file__).with_name(name).write_bytes(data + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b''))


png('player.png', 30, 50, lambda x, y: (74, 144, 217, 255))
png('ground.png', 680, 64, lambda x, y: (82, 137, 95, 255) if y < 8 else (61, 107, 74, 255))
png('rock.png', 64, 64, lambda x, y: (90, 90, 102, 255))
png('coin.png', 18, 18, lambda x, y: (217, 183, 63, 255 if (x-8.5)**2+(y-8.5)**2 < 81 else 0))
