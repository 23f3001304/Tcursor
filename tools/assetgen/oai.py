import json, sys, base64, urllib.request, pathlib, time, os
SP = pathlib.Path(__file__).resolve().parent
KEY = os.environ["OPENAI_API_KEY"]  # never a file in the repo
H = {"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"}
def call(path, body=None, timeout=240):
    req = urllib.request.Request("https://api.openai.com/v1" + path, headers=H,
        data=json.dumps(body).encode() if body is not None else None, method="POST" if body is not None else "GET")
    with urllib.request.urlopen(req, timeout=timeout) as r: return json.loads(r.read())
def gen(name, prompt, size, quality, model, transparent=False, n=1):
    body = {"model": model, "prompt": prompt, "size": size, "quality": quality, "n": n, "output_format": "png"}
    if transparent: body["background"] = "transparent"
    t = time.time()
    # A dropped connection mid-download (http.client.IncompleteRead) is not an HTTPError - retry on
    # anything, three times with backoff, so one flaky read cannot kill a 40-image batch.
    for attempt in range(3):
        try: r = call("/images/generations", body); break
        except Exception as e:
            if attempt == 2: raise
            print(f"  retry {name} ({type(e).__name__}: {str(e)[:80]})"); time.sleep(5 * (attempt + 1))
    outs = []
    for i, d in enumerate(r["data"]):
        p = SP / (f"{name}.png" if n == 1 else f"{name}_{i+1}.png")
        p.write_bytes(base64.b64decode(d["b64_json"])); outs.append(p)
    u = r.get("usage", {})
    print(f"{name}: {len(outs)} file(s), {time.time()-t:.0f}s, usage={u.get('input_tokens')}/{u.get('output_tokens')} tokens")
    return outs
if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "models":
        ms = sorted(m["id"] for m in call("/models")["data"] if "image" in m["id"] or "dall" in m["id"])
        print("image models:", ms)
    elif cmd == "probe":
        model = sys.argv[2]
        gen("probe_cursor_arrow",
            "A single mouse cursor arrow pointer icon in a modern macOS style: white fill, a thin dark navy (#0d1c52) outline, "
            "a very subtle soft drop shadow, slightly rounded corners, flat clean vector look with no gradient. Centered, large, "
            "completely isolated on a transparent background. No text, no other objects.",
            "1024x1024", "medium", model, transparent=True)
        gen("probe_wallpaper_1",
            "Abstract desktop wallpaper in the style of the macOS Sequoia and Sonoma wallpapers: a few large, layered, translucent "
            "glass-like ribbons flowing diagonally with soft volumetric light and gentle depth of field, in deep midnight navy "
            "(#0d1c52) through royal blue (#2b53e6) with one warm coral (#ef4444) ribbon as the only accent. Silky, ultra smooth, "
            "premium, calm; the center third stays darker and quieter than the edges. No text, no logos, no grain.",
            "1536x1024", "high", model)
