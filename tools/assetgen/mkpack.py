"""Turn a folder of generated 1024x1024 transparent cursor PNGs into a TCursor cursor pack:
crop to content, pad, resize to SIZE, derive hotspots by kind, write hotspots.json + pack.json,
and a contact sheet for review.  usage: python mkpack.py <src_dir> <pack_id> "<Pack Name>" <out_dir>"""
import sys, json, pathlib
from PIL import Image
SIZE = 256          # sprite canvas (square); export scales by the panel factor anyway
PAD = 0.06          # padding as a fraction of SIZE on each side
KINDS = ["arrow", "ibeam", "hand", "resize_ns", "resize_ew", "resize_nwse", "resize_nesw", "move", "busy"]
# Hotspot rule per kind: "tip" = topmost opaque pixel (arrow tip / fingertip); "center" = bbox centre.
RULE = {"arrow": "tip", "hand": "tip"}
def bbox(im, thr=40):
    a = im.getchannel("A").point(lambda v: 255 if v > thr else 0)
    return a.getbbox()
def tip(im, thr=40):
    a = im.getchannel("A"); w, h = im.size; px = a.load()
    for y in range(h):
        xs = [x for x in range(w) if px[x, y] > thr]
        if xs: return (sum(xs) / len(xs), y)
    return (w / 2, h / 2)
def build(src, pack_id, name, out):
    out = pathlib.Path(out) / pack_id; out.mkdir(parents=True, exist_ok=True)
    hots, tiles = {}, []
    for kind in KINDS:
        p = pathlib.Path(src) / f"{kind}.png"
        if not p.exists(): print("missing", p); continue
        im = Image.open(p).convert("RGBA")
        bb = bbox(im)
        if not bb: print("empty", p); continue
        crop = im.crop(bb)
        # fit into SIZE with padding, keep aspect, centred
        inner = int(SIZE * (1 - 2 * PAD)); s = min(inner / crop.width, inner / crop.height)
        rs = crop.resize((max(1, round(crop.width * s)), max(1, round(crop.height * s))), Image.LANCZOS)
        canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        ox, oy = (SIZE - rs.width) // 2, (SIZE - rs.height) // 2
        canvas.paste(rs, (ox, oy), rs)
        if RULE.get(kind) == "tip":
            tx, ty = tip(canvas); hx, hy = tx / SIZE, ty / SIZE
        else:
            hx, hy = (ox + rs.width / 2) / SIZE, (oy + rs.height / 2) / SIZE
        hots[kind] = [round(hx, 4), round(hy, 4)]
        canvas.save(out / f"{kind}.png"); tiles.append((kind, canvas))
    (out / "hotspots.json").write_text(json.dumps(hots, indent=2))
    (out / "pack.json").write_text(json.dumps({"id": pack_id, "name": name, "version": 1,
        "generated": "gpt-image-2.5-sunburst via OpenAI Images API, 2026-09-13; MIT, TCursor project"}, indent=2))
    # contact sheet on a mid-grey checker so both light and dark sprites read
    sheet = Image.new("RGBA", (SIZE * len(tiles), SIZE), (128, 128, 128, 255))
    for i, (kind, t) in enumerate(tiles):
        bg = Image.new("RGBA", (SIZE, SIZE), (96, 96, 96, 255) if i % 2 else (160, 160, 160, 255))
        bg.paste(t, (0, 0), t); sheet.paste(bg, (i * SIZE, 0))
    sheet.convert("RGB").save(out.parent / f"{pack_id}_sheet.png")
    print(pack_id, "->", len(tiles), "sprites;", "hotspots:", hots)
if __name__ == "__main__":
    build(*sys.argv[1:5])
