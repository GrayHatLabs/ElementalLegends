# Elemental Legends level editor

A single-page editor for the hand-made cave and lair rooms described in
[`docs/LEVEL_FORMAT.md`](../../docs/LEVEL_FORMAT.md). Plain HTML/JS: no server, no install,
no network.

## Setup

1. Bundle the art (once, and again whenever the art changes):

   ```
   python scripts/build_editor_assets.py
   ```

   This reads `../ElementalLegends-art/sheets` (or a path given as the first argument) and
   writes `tools/level-editor/assets.js` with the terrain tilesets, enemy / object sprites,
   the mage and item icons. Anything missing is skipped and drawn as a coloured shape.
   Needs Python 3 and Pillow.

2. Open `tools/level-editor/index.html` in **Chrome or Edge** (double-click it; `file://` is
   fine). Firefox works too, but cannot save straight into a folder.

## Using it

- **New** picks the kind (cave / lair) and number. **Open** (or drag a `.json` onto the
  window) loads an existing level. The header sets the name and the theme.
- Each **room slot** has a tab. A slot that is not defined keeps the game's generated
  layout; **Define room** starts an empty walled 16 x 13 room. For caves, defining the
  `mouth` makes it a three-room cave.
- **Tools** (right): tile brushes (floor, wall, water, ice, crack, plate, decor) with paint,
  rectangle, flood fill and eyedropper; **Enemy** and **Object** place things with the
  settings shown below the palette; **Select** moves and deletes. Right click paints floor or
  deletes the thing under the cursor. Press **Help** or `?` for all shortcuts.
- **Game features** shows what the game adds on top of your room: door gaps (green), the
  hub lock, cave challenge seal and pantry crack (amber) and the lair stairs (violet).
- **Check** runs every validation rule from the spec (plus a few warnings, such as a
  blocked doorway). Click a problem to jump to it.
- **JSON** shows the file text (copy it, or paste a level and load it).

## Where files go

- **Save** writes `cave_N.json` / `lair_N.json`. After **Choose levels folder** (Chrome /
  Edge) it saves straight into that folder and lists its files for opening; otherwise the
  file is downloaded and you move it yourself.
- Built-in levels live in the repo's `levels/` folder and are embedded in the game by
  `python scripts/import_levels.py` (rebuild afterwards).
- To test without rebuilding, put the file in a `levels/` folder **next to the game
  executable** or in the directory you run it from: a file there overrides the built-in one
  with the same name. A broken room is logged and ignored (the generated layout is used).

`levels/cave_2.json` is an example made with the editor.
