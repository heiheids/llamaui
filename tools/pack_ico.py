"""把各尺寸的 RGBA 像素打包成多尺寸 Windows ICO。

为了兼容性：
  16/24/32/48/64 用经典 BMP(DIB) 条目（任务栏、标题栏最稳）
  128/256 用 PNG 条目（体积小，Vista 以上支持）

用法: python tools/pack_ico.py
输入: assets/icon-<size>.rgba （每个像素 R,G,B,A，自上而下）
输出: assets/icon.ico
"""
import os
import struct

SIZES_BMP = [16, 24, 32, 48, 64]
SIZES_PNG = [128, 256]
ASSETS = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets")


def read_rgba(size):
    path = os.path.join(ASSETS, f"icon-{size}.rgba")
    with open(path, "rb") as f:
        data = f.read()
    if len(data) != size * size * 4:
        raise SystemExit(f"bad rgba size for {size}: {len(data)}")
    return data


def bmp_entry(size):
    """构造 32bpp DIB：BITMAPINFOHEADER + 自下而上的 BGRA + AND 掩码"""
    rgba = read_rgba(size)
    header = struct.pack(
        "<IiiHHIIiiII",
        40,               # biSize
        size,             # biWidth
        size * 2,         # biHeight（颜色 + 掩码）
        1,                # biPlanes
        32,               # biBitCount
        0,                # biCompression = BI_RGB
        size * size * 4,  # biSizeImage
        0, 0, 0, 0,
    )
    # 自下而上
    rows = []
    for y in range(size - 1, -1, -1):
        row = bytearray()
        base = y * size * 4
        for x in range(size):
            r, g, b, a = rgba[base + x * 4: base + x * 4 + 4]
            row += bytes((b, g, r, a))
        rows.append(bytes(row))
    color = b"".join(rows)
    # AND 掩码：1bpp，行按 4 字节对齐；32bpp 下基本不用，全 0
    mask_row = ((size + 31) // 32) * 4
    mask = b"\x00" * (mask_row * size)
    return header + color + mask


def png_entry(size):
    path = os.path.join(ASSETS, f"icon-{size}.png")
    with open(path, "rb") as f:
        return f.read()


def main():
    images = []
    for s in SIZES_BMP:
        images.append((s, bmp_entry(s)))
    for s in SIZES_PNG:
        images.append((s, png_entry(s)))

    out = bytearray()
    out += struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    for size, data in images:
        w = 0 if size >= 256 else size
        h = 0 if size >= 256 else size
        out += struct.pack(
            "<BBBBHHII",
            w, h, 0, 0,
            1,      # planes
            32,     # bit count
            len(data),
            offset,
        )
        offset += len(data)
    for _, data in images:
        out += data

    dst = os.path.join(ASSETS, "icon.ico")
    with open(dst, "wb") as f:
        f.write(out)
    print(f"wrote {dst} ({len(out)} bytes, {len(images)} images)")


if __name__ == "__main__":
    main()
