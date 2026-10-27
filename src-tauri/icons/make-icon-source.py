# Writes a 1024x1024 RGBA app icon in the macOS style: a teal squircle
# (superellipse) inset with a transparent margin, with a white chip on it.
import struct, sys, zlib

N = 1024
MARGIN = 100                      # Apple's template: ~824 px art in 1024
HALF = (N - 2 * MARGIN) / 2       # squircle half size
CX = CY = N / 2
EXP = 5.0                         # superellipse exponent (squircle-like)
TEAL = (0, 135, 143)
WHITE = (255, 255, 255)
PIN = (200, 236, 238)

def inside_squircle(x, y):
    u = abs(x - CX) / HALF
    v = abs(y - CY) / HALF
    return u ** EXP + v ** EXP <= 1.0

def chip_color(x, y):
    """Color of the chip artwork at a pixel center, or None for background."""
    # Body: a white square with a teal "die" in the middle.
    b0, b1 = 352, 672
    if b0 <= x < b1 and b0 <= y < b1:
        if 432 <= x < 592 and 432 <= y < 592:
            return TEAL
        return WHITE
    # Pins: 4 on each side.
    for i in range(4):
        p0 = 376 + i * 80
        if p0 <= x < p0 + 32:
            if 272 <= y < 352 or 672 <= y < 752:
                return PIN
        if p0 <= y < p0 + 32:
            if 272 <= x < 352 or 672 <= x < 752:
                return PIN
    return None

SS = 4  # supersampling for the squircle's edge
rows = []
for y in range(N):
    row = bytearray([0])
    for x in range(N):
        # Coverage of the squircle; only sample edge pixels finely.
        u = abs(x + 0.5 - CX) / HALF
        v = abs(y + 0.5 - CY) / HALF
        f = u ** EXP + v ** EXP
        if f < 0.97:
            cov = 1.0
        elif f > 1.03:
            cov = 0.0
        else:
            hits = 0
            for sy in range(SS):
                for sx in range(SS):
                    if inside_squircle(x + (sx + 0.5) / SS, y + (sy + 0.5) / SS):
                        hits += 1
            cov = hits / (SS * SS)
        if cov == 0.0:
            row += bytes((0, 0, 0, 0))
            continue
        c = chip_color(x + 0.5, y + 0.5) or TEAL
        row += bytes((c[0], c[1], c[2], round(255 * cov)))
    rows.append(bytes(row))

def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)

png = (b"\x89PNG\r\n\x1a\n"
       + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0))
       + chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
       + chunk(b"IEND", b""))
open(sys.argv[1], "wb").write(png)
