"""Reproduce public synthetic .g4e and .torrent examples, using only Python stdlib."""
from pathlib import Path
import copy
import hashlib
import json

base = Path(__file__).resolve().parent
originals = json.loads((base.parent / "g4-format/fixtures/manifest.json").read_text(encoding="utf-8"))
payload = {"docs/日本語.txt": "Preservação ∞\n".encode("utf-8"),
           "empty.bin": b"", "game/hello.txt": b"Game4 package fixture\n"}
for entry in originals["files"]:
    data = payload[entry["path"]]
    assert str(len(data)) == entry["sizeBytes"]
    assert hashlib.sha256(data).hexdigest() == entry["sha256"]

def bencode(value):
    if isinstance(value, bytes):
        return str(len(value)).encode() + b":" + value
    if isinstance(value, str):
        return bencode(value.encode("utf-8"))
    if isinstance(value, int):
        return b"i" + str(value).encode() + b"e"
    if isinstance(value, list):
        return b"l" + b"".join(map(bencode, value)) + b"e"
    if isinstance(value, dict):
        return b"d" + b"".join(bencode(k) + bencode(value[k]) for k in sorted(value)) + b"e"
    raise TypeError(value)

original_info = {
    "files": [{"length": int(f["sizeBytes"]), "path": f["path"].split("/")}
              for f in originals["files"]],
    "name": "Game4-synthetic-originals", "piece length": 16384,
    "pieces": hashlib.sha1(b"".join(payload[f["path"]] for f in originals["files"])).digest(),
}
raw_package = (base.parent / "g4-format/fixtures/v1-minimal.g4").read_bytes()
packaged = copy.deepcopy(originals)
packaged["files"] = [{"path": "synthetic-collection.g4", "sizeBytes": str(len(raw_package)),
                     "sha256": hashlib.sha256(raw_package).hexdigest()}]
package_info = {"length": len(raw_package), "name": "synthetic-collection.g4", "piece length": 16384,
                "pieces": hashlib.sha1(raw_package).digest()}
for name, manifest, info, mode, descriptor_id in [
    ("original-files", originals, original_info, "original_files", "22345678-1234-4234-9234-123456789abc"),
    ("package", packaged, package_info, "g4_package", "32345678-1234-4234-9234-123456789abc"),
]:
    metainfo = bencode({"info": info})
    target = base / "examples" / (name + ".torrent")
    target.write_bytes(metainfo)
    info_hash = hashlib.sha1(bencode(info)).hexdigest()
    document = {
        "$schema": "urn:game4:ever-manifest:1", "format": "game4-ever-manifest", "schemaVersion": 1,
        "id": descriptor_id, "representation": mode, "manifest": manifest,
        "torrents": [{"infoHashV1": info_hash, "magnet": "magnet:?xt=urn:btih:" + info_hash,
                      "metainfoFile": {"path": target.name, "sizeBytes": str(len(metainfo)),
                                       "sha256": hashlib.sha256(metainfo).hexdigest()}}],
        "extensions": {},
    }
    (base / "examples" / (name + ".g4e")).write_text(
        json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
print("Generated synthetic .g4e descriptors and trackerless .torrent companions; no network access.")
