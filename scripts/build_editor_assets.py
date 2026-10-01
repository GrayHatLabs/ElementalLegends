#!/usr/bin/env python3
"""Bundle the art the level editor needs into tools/level-editor/assets.js.

Reads the art folder (default: ../ElementalLegends-art/sheets next to the repo, or the
path given as the first argument) and writes `window.EL_ASSETS = {...}` with base64 PNG
data URLs, so the editor works when opened straight from disk (file://) with no server.

Contents:
  themes[<index>]   {wall, water?}   96x96 Wang corner atlases (4x4 tiles of 24 px)
  enemies[kind][element]  {src, w, h}  one idle frame (idle_down for 4-direction sheets)
  objects[name]     {src, w, h}      torch, block, lever_off, lever_on, chest, shrine, and the
                                     format 2 objects (big_chest, whip_post, boulder, pot, crate,
                                     floor_switch, crystal_switch, ice_block, tablet,
                                     crystal_<element>, crystal_<element>_lit)
  tiles[name]       {src, w, h}      format 2 tiles: pit, lava (frame 0), thorns, and the
                                     barrier pegs orange_up/_down, blue_up/_down
  relics[name]      {src, w, h}      16 px icons from relic_items (relics, small/big key, map...)
  props[name]       {src, w, h, theme}  every prop_<theme>_<name> sheet (first frame)
  mage              {src, w, h}      mage_fire idle_down, for scale
  items[name]       {src, w, h}      icons from the items sheet (chest contents)

Anything missing is skipped; the editor draws coloured shapes instead.

Usage:  python scripts/build_editor_assets.py [path/to/sheets]
"""
import base64
import io
import json
import os
import re
import sys

from PIL import Image

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_ART = os.path.join(os.path.dirname(REPO), "ElementalLegends-art", "sheets")
OUT = os.path.join(REPO, "tools", "level-editor", "assets.js")

KINDS = ["slime", "bat", "skeleton", "imp", "ghost", "golem"]
ELEMENTS = ["fire", "ice", "storm", "earth", "neutral"]
SINGLE = {"zombie": "enemy_zombie", "generator": "enemy_generator"}
OBJECTS = {
    "torch": "obj_brazier",
    "block": "obj_push_block",
    "lever_off": "obj_lever_off",
    "lever_on": "obj_lever_on",
    "chest": "obj_chest_closed",
    "shrine": "obj_shrine_pedestal",
    # Format 2 lairs.
    "big_chest": "obj_big_chest_closed",
    "whip_post": "obj_whip_post",
    "boulder": "obj_boulder",
    "pot": "obj_pot",
    "crate": "obj_crate",
    "floor_switch": "obj_floor_switch_up",
    "crystal_switch": "obj_crystal_switch_orange",
    "ice_block": "obj_ice_block",
    "tablet": "obj_lore_tablet",
}
CRYSTALS = ["fire", "ice", "storm", "earth"]
TILES = {
    "pit": "tile_pit",
    "lava": "tile_lava",
    "thorns": "tile_thorns",
    "orange_up": "obj_block_orange_up",
    "orange_down": "obj_block_orange_down",
    "blue_up": "obj_block_blue_up",
    "blue_down": "obj_block_blue_down",
}


def data_url(img):
    buf = io.BytesIO()
    img.save(buf, "PNG", optimize=True)
    return "data:image/png;base64," + base64.b64encode(buf.getvalue()).decode("ascii")


def warn(msg):
    print("  skip: " + msg)


def load_manifests(art):
    sprites = {}
    for name in ("manifest.json", "manifest_objects.json", "manifest_bosses.json",
                 "manifest_relics.json", "manifest_props.json"):
        p = os.path.join(art, name)
        if not os.path.exists(p):
            warn(name + " not found")
            continue
        with open(p, encoding="utf-8") as f:
            sprites.update(json.load(f).get("sprites", {}))
    return sprites


def frame(art, sprites, sheet, anims=("idle_down", "idle"), col=0):
    """First frame of the first anim in `anims` the sheet has, or None."""
    d = sprites.get(sheet)
    if d is None:
        warn(sheet + " not in manifests")
        return None
    path = os.path.join(art, d.get("file", sheet + ".png"))
    if not os.path.exists(path):
        warn(path + " missing")
        return None
    anim = next((d["anims"][a] for a in anims if a in d.get("anims", {})), None)
    if anim is None:
        warn(sheet + " has none of the rows " + "/".join(anims))
        return None
    cw, ch = d["cell"]
    img = Image.open(path).convert("RGBA")
    box = (col * cw, anim["row"] * ch, col * cw + cw, anim["row"] * ch + ch)
    if box[2] > img.width or box[3] > img.height:
        warn(sheet + " frame outside the sheet")
        return None
    fr = img.crop(box)
    return {"src": data_url(fr), "w": cw, "h": ch}


def main():
    art = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ART
    if not os.path.isdir(art):
        sys.exit("art folder not found: " + art)
    print("art: " + art)
    sprites = load_manifests(art)
    out = {"tile": 24, "themes": {}, "enemies": {}, "objects": {}, "mage": None, "items": {},
           "tiles": {}, "relics": {}, "props": {}}

    # Terrain atlases.
    tdir = os.path.join(art, "terrain")
    names = {}
    tj = os.path.join(tdir, "terrain.json")
    if os.path.exists(tj):
        with open(tj, encoding="utf-8") as f:
            for t in json.load(f).get("themes", []):
                names[t["index"]] = t.get("name", "")
    for i in range(0, 32):
        wall = os.path.join(tdir, "theme%d_wall.png" % i)
        if not os.path.exists(wall):
            continue
        entry = {"name": names.get(i, "")}
        img = Image.open(wall).convert("RGBA")
        if img.size != (96, 96):
            warn("%s is %dx%d, expected 96x96" % (wall, img.width, img.height))
            continue
        entry["wall"] = data_url(img)
        water = os.path.join(tdir, "theme%d_water.png" % i)
        if os.path.exists(water):
            wimg = Image.open(water).convert("RGBA")
            if wimg.size == (96, 96):
                entry["water"] = data_url(wimg)
        else:
            warn("theme %d has no water atlas (the editor draws water as a plain colour)" % i)
        out["themes"][str(i)] = entry

    # Enemies.
    for kind in KINDS:
        for el in ELEMENTS:
            fr = frame(art, sprites, "enemy_%s_%s" % (kind, el))
            if fr:
                out["enemies"].setdefault(kind, {})[el] = fr
    for kind, sheet in SINGLE.items():
        fr = frame(art, sprites, sheet)
        if fr:
            out["enemies"][kind] = {"any": fr}

    # Objects and the mage.
    for key, sheet in OBJECTS.items():
        fr = frame(art, sprites, sheet)
        if fr:
            out["objects"][key] = fr
    for el in CRYSTALS:
        for suffix, anims in (("", ("idle",)), ("_lit", ("lit", "idle"))):
            fr = frame(art, sprites, "obj_crystal_" + el, anims=anims)
            if fr:
                out["objects"]["crystal_" + el + suffix] = fr
    out["mage"] = frame(art, sprites, "mage_fire")

    # Format 2 tiles (frame 0 of animated ones such as lava).
    for key, sheet in TILES.items():
        fr = frame(art, sprites, sheet)
        if fr:
            out["tiles"][key] = fr

    # Relic / dungeon item icons (one per row of relic_items).
    if "relic_items" in sprites:
        for name in sprites["relic_items"].get("anims", {}):
            fr = frame(art, sprites, "relic_items", anims=(name,))
            if fr:
                out["relics"][name] = fr
    else:
        warn("relic_items not in manifests")

    # Props for the format 2 prop palette (grouped by lair theme in the editor).
    for sheet in sorted(sprites):
        m = re.match(r"^prop_(\d+)_[a-z0-9_]+$", sheet)
        if not m:
            continue
        fr = frame(art, sprites, sheet)
        if fr:
            fr["theme"] = int(m.group(1))
            out["props"][sheet] = fr

    # Item icons (column 0 of each row of the items sheet).
    items = sprites.get("items")
    if items:
        for name in items.get("anims", {}):
            fr = frame(art, sprites, "items", anims=(name,))
            if fr:
                out["items"][name] = fr

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("// Generated by scripts/build_editor_assets.py - do not edit.\n")
        f.write("window.EL_ASSETS = ")
        json.dump(out, f, separators=(",", ":"))
        f.write(";\n")
    n_en = sum(len(v) for v in out["enemies"].values())
    print("wrote %s: %d themes, %d enemy frames, %d objects, %d item icons, %d tiles, %d relic icons, "
          "%d props, mage=%s (%d KB)" % (
              os.path.relpath(OUT, REPO), len(out["themes"]), n_en, len(out["objects"]), len(out["items"]),
              len(out["tiles"]), len(out["relics"]), len(out["props"]),
              "yes" if out["mage"] else "no", os.path.getsize(OUT) // 1024))


if __name__ == "__main__":
    main()
