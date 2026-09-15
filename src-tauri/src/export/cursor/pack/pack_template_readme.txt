TCursor cursor pack: folder contract
=====================================

This folder is a starting point. Edit the PNGs, edit pack.json, then use
"Import pack..." in the editor's Cursor panel to add it as a real pack.

Sprite files (one PNG per cursor state)
----------------------------------------
  arrow.png        the plain pointer
  ibeam.png        text cursor
  hand.png         link / clickable
  resize_ns.png    vertical resize
  resize_ew.png    horizontal resize
  resize_nwse.png  diagonal resize, top-left to bottom-right
  resize_nesw.png  diagonal resize, top-right to bottom-left
  move.png         move / drag
  busy.png         loading / wait (see "Busy animation" below)

Any state you leave out simply falls back to TCursor's built-in sprite for
that state, so a pack does not need to cover all nine to be valid - it is
only invalid if it has none of them. An empty (0-byte) file counts as left
out too. Sprites should be square RGBA PNGs with straight (not premultiplied)
alpha; the bundled packs use 128x128, which is a good default size.

hotspots.json (optional)
-------------------------
A JSON object mapping each state name above to [hx, hy]: the click point as a
fraction (0.0 to 1.0) of that sprite's own width and height, e.g. an arrow
whose tip sits near the top-left corner of its canvas might use [0.22, 0.06].
Any state missing from this file, or the whole file being absent, defaults to
a centered hotspot (0.5, 0.5) for that state.

pack.json fields
-----------------
  id        string, a short unique id for this pack (internal use)
  name      string, shown in the Cursor panel's pack picker
  category  optional string; which section the picker groups this pack
            under (Classic, Glass and glow, Playful, Drawn, Retro). Leave
            it out, or blank, and the pack lists under "Imported".
  version   optional, informational only; not read by the loader
  busy      optional object, see "Busy animation" below
  material  optional string. Leave it out for a plain sprite (the normal
            case). Set it to "glass" to make every sprite a refracting lens
            over the recording instead of a flat picture - artwork for a
            glass pack should be a clear lens shape with highlights and a
            rim, not an opaque drawing, since whatever it paints opaquely is
            frame the lens cannot bend through.

Busy animation
----------------
A single busy.png does not have to sit still. Declare it in pack.json:

  "busy": { "anim": "spin", "fps": 24 }

  anim: "spin"   one full clockwise turn per cycle (cycle length = 24/fps
                 seconds - fps 24 is one turn per second)
        "flip"   holds still, then turns 180 degrees over the last part of
                 a one-second cycle, always turning the same way
        "pulse"  breathes: scales up slightly and back over a one-second
                 cycle, in place

If you would rather hand-draw the frames yourself, ship busy_00.png,
busy_01.png, busy_02.png, ... (two-digit, zero-padded, up to 64 frames,
stopping at the first missing number) instead of relying on "anim". Explicit
frames like these take precedence over a declared "anim" and are played back
at "fps" frames per second.

Where an imported pack lives
------------------------------
This template folder can live anywhere. Once you use "Import pack..." in the
Cursor panel to pick it, TCursor copies its sprites into its own app-data
cursors folder, and that copy - not this folder - is what the app reads from
then on. Keep editing this folder and import it again to update the copy.
