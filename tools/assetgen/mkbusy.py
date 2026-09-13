"""Animated busy state for a built pack (out/<pack_id>/busy.png -> busy_00..NN.png + pack.json 'busy').
Kinds: spin (rotate the frame; rings, spinners), flip (stepped 180-degree flips; hourglasses),
pulse (breathing scale 1.0 -> 1.06 -> 1.0; sleeping cat, pocket watch, anything that must not rotate).
usage: python mkbusy.py <pack_id> <spin|flip|pulse> [frames=24] [fps=24]"""
import sys, json, pathlib, math
from PIL import Image
def frames_for(src, kind, n):
    w, h = src.size; c = (w / 2, h / 2)
    for i in range(n):
        t = i / n
        if kind == "spin":
            yield src.rotate(-360 * t, resample=Image.BICUBIC, center=c)
        elif kind == "flip":
            # hold, then a quick 180 flip: 70% of the cycle static, 30% turning (eases in/out)
            u = 0.0 if t < 0.7 else (1 - math.cos(math.pi * (t - 0.7) / 0.3)) / 2
            yield src.rotate(-180 * u, resample=Image.BICUBIC, center=c)
        else:  # pulse
            s = 1.0 + 0.06 * (1 - math.cos(2 * math.pi * t)) / 2
            sw, sh = round(w * s), round(h * s)
            big = src.resize((sw, sh), Image.LANCZOS)
            f = Image.new("RGBA", src.size, (0, 0, 0, 0)); f.paste(big, ((w - sw) // 2, (h - sh) // 2), big)
            yield f
def build(pack_id, kind, n=24, fps=24):
    d = pathlib.Path("out") / pack_id
    src = Image.open(d / "busy.png").convert("RGBA")
    for old in d.glob("busy_*.png"): old.unlink()
    for i, f in enumerate(frames_for(src, kind, n)): f.save(d / f"busy_{i:02d}.png")
    meta = json.loads((d / "pack.json").read_text())
    meta["busy"] = {"frames": n, "fps": fps, "anim": kind}
    (d / "pack.json").write_text(json.dumps(meta, indent=2))
    # preview gif on grey
    gif = []
    for i in range(n):
        f = Image.open(d / f"busy_{i:02d}.png").convert("RGBA")
        bg = Image.new("RGBA", f.size, (120, 120, 120, 255)); bg.paste(f, (0, 0), f); gif.append(bg.convert("P", palette=Image.ADAPTIVE))
    gif[0].save(d.parent / f"{pack_id}_busy.gif", save_all=True, append_images=gif[1:], duration=int(1000 / fps), loop=0)
    print(pack_id, kind, n, "frames @", fps, "fps")
if __name__ == "__main__":
    a = sys.argv[1:]; build(a[0], a[1], int(a[2]) if len(a) > 2 else 24, int(a[3]) if len(a) > 3 else 24)
