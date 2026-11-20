"""读取 GGUF 文件头部的元数据，用于判断模型架构等关键信息。

用法 / Usage: python tools/gguf_info.py "<模型路径>"
只使用标准库，按 GGUF v2/v3 规范解析。
"""
import struct
import sys

# GGUF 元数据值类型
STRING = 8
ARRAY = 9


def read_str(f):
    n = struct.unpack("<Q", f.read(8))[0]
    return f.read(n).decode("utf-8", "replace")


def read_value(f, vtype):
    if vtype == 0:  # UINT8
        return struct.unpack("<B", f.read(1))[0]
    if vtype == 1:  # INT8
        return struct.unpack("<b", f.read(1))[0]
    if vtype == 2:  # UINT16
        return struct.unpack("<H", f.read(2))[0]
    if vtype == 3:
        return struct.unpack("<h", f.read(2))[0]
    if vtype == 4:
        return struct.unpack("<I", f.read(4))[0]
    if vtype == 5:
        return struct.unpack("<i", f.read(4))[0]
    if vtype == 6:
        return struct.unpack("<f", f.read(4))[0]
    if vtype == 7:
        return struct.unpack("<B", f.read(1))[0]
    if vtype == STRING:
        return read_str(f)
    if vtype == ARRAY:
        atype = struct.unpack("<I", f.read(4))[0]
        alen = struct.unpack("<Q", f.read(8))[0]
        items = []
        for _ in range(min(alen, 8)):
            items.append(read_value(f, atype))
        if alen > 8:
            for _ in range(alen - 8):
                read_value(f, atype)
        return f"<array type={atype} len={alen}> {items[:8]}"
    if vtype == 10:
        return struct.unpack("<Q", f.read(8))[0]
    if vtype == 11:
        return struct.unpack("<q", f.read(8))[0]
    if vtype == 12:
        return struct.unpack("<d", f.read(8))[0]
    raise ValueError(f"unknown gguf value type {vtype}")


def main():
    path = sys.argv[1]
    with open(path, "rb") as f:
        magic = f.read(4)
        if magic != b"GGUF":
            print("not a gguf file:", magic)
            return
        version = struct.unpack("<I", f.read(4))[0]
        tensors = struct.unpack("<Q", f.read(8))[0]
        kvs = struct.unpack("<Q", f.read(8))[0]
        print(f"magic=GGUF version={version} tensors={tensors} kv_count={kvs}")
        print("-" * 60)
        for _ in range(kvs):
            key = read_str(f)
            vtype = struct.unpack("<I", f.read(4))[0]
            val = read_value(f, vtype)
            show = val if not isinstance(val, str) or len(val) <= 120 else val[:120] + "..."
            print(f"{key} = {show}")


if __name__ == "__main__":
    main()
