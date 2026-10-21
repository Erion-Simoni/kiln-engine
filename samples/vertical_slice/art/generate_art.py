import struct, zlib

def chunk(tag, data):
    c = tag + data
    return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

def write_png(path, w, h, rows):
    raw = b''.join(b'\x00' + bytes(v for px in r for v in px) for r in rows)
    png = (b'\x89PNG\r\n\x1a\n'
           + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
           + chunk(b'IDAT', zlib.compress(raw, 9))
           + chunk(b'IEND', b''))
    open(path, 'wb').write(png)

def grid(w, h):
    return [[0, 0, 0, 0] for _ in range(h) for _ in range(w)]

def rows_of(px, w, h):
    return [px[y * w:(y + 1) * w] for y in range(h)]

def put(px, w, h, x, y, r, g, b, a=255):
    if 0 <= x < w and 0 <= y < h:
        px[y * w + x] = [r, g, b, a]

# Hero: 16x16 blue robot with a visor.
W = H = 16
px = grid(W, H)
body, dark, visor = (58, 110, 220), (28, 58, 130), (240, 240, 255)
for y in range(4, 15):
    for x in range(3, 13):
        put(px, W, H, x, y, *body)
for y in range(4, 6):
    for x in range(2, 14):
        put(px, W, H, x, y, *dark)
for y in range(7, 10):
    for x in range(5, 11):
        put(px, W, H, x, y, *visor)
for x in range(6, 10):
    put(px, W, H, x, 11, *dark)
for x in (4, 11):
    put(px, W, H, x, 15, *dark)
write_png('samples/vertical_slice/art/hero.png', W, H, rows_of(px, W, H))

# Coin: 16x16 gold disc.
W = H = 16
px = grid(W, H)
gold, shine = (240, 196, 60), (250, 226, 120)
for y in range(3, 14):
    for x in range(3, 14):
        if (x - 8) ** 2 + (y - 8) ** 2 <= 26:
            put(px, W, H, x, y, *gold)
for y in range(5, 12):
    for x in range(5, 12):
        if (x - 8) ** 2 + (y - 8) ** 2 <= 9:
            put(px, W, H, x, y, *shine)
write_png('samples/vertical_slice/art/coin.png', W, H, rows_of(px, W, H))

# Ground tile: 16x16 grass.
W = H = 16
px = grid(W, H)
grass, alt, dirt = (86, 150, 74), (74, 134, 66), (120, 92, 58)
for y in range(H):
    for x in range(W):
        c = alt if (x + y) % 7 == 0 else grass
        put(px, W, H, x, y, *c)
for x in range(0, W, 5):
    put(px, W, H, x, 15, *dirt)
write_png('samples/vertical_slice/art/ground.png', W, H, rows_of(px, W, H))
