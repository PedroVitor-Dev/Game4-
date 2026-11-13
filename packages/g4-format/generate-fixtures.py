"""Regenerate only the public synthetic ZIP32/ZIP64 fixtures (Python stdlib)."""
from pathlib import Path
import io
import json
import hashlib
import struct
import zipfile

base = Path(__file__).resolve().parent / "fixtures"
manifest = json.loads((base / "manifest.json").read_text(encoding="utf-8"))
payload = {
    "docs/日本語.txt": "Preservação ∞\n".encode("utf-8"),
    "empty.bin": b"",
    "game/hello.txt": b"Game4 package fixture\n",
}
for entry in manifest["files"]:
    data = payload[entry["path"]]
    assert entry["sha256"] == hashlib.sha256(data).hexdigest()
    assert entry["sizeBytes"] == str(len(data))

def make(force_zip64):
    stream = io.BytesIO()
    with zipfile.ZipFile(stream, "w", compression=zipfile.ZIP_STORED) as archive:
        entries = [("game4.json", (base / "descriptor.json").read_bytes()),
                   ("manifest.json", (base / "manifest.json").read_bytes())]
        entries += [("data/" + name, data) for name, data in sorted(payload.items())]
        for name, data in entries:
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            with archive.open(info, "w", force_zip64=force_zip64) as target:
                target.write(data)
    return stream.getvalue()

(base / "v1-minimal.g4").write_bytes(make(False))
data = make(True)
end = len(data) - 22
count, size, offset = struct.unpack_from("<HII", data, end + 10)
zip64 = struct.pack("<4sQHHIIQQQQ", b"PK\x06\x06", 44, 45, 45, 0, 0,
                    count, count, size, offset)
locator = struct.pack("<4sIQI", b"PK\x06\x07", 0, end, 1)
eocd = bytearray(data[end:])
struct.pack_into("<HHII", eocd, 8, 65535, 65535, 0xffffffff, 0xffffffff)
(base / "v1-zip64.g4").write_bytes(data[:end] + zip64 + locator + eocd)
print("Generated public synthetic .g4 fixtures (40 payload bytes).")
