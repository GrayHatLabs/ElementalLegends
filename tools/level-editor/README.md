# Elemental Legends level editor

A single-page editor for the hand-made cave and lair rooms described in
[`docs/LEVEL_FORMAT.md`](../../docs/LEVEL_FORMAT.md): format 1 (caves, and lairs with fixed
room slots) and format 2 (hand-designed lairs with a free room layout, like the shipped
`levels/lair_1.json` .. `lair_6.json`). Plain HTML/JS: no server, no install, no network.

## Setup

1. Bundle the art (once, and again whenever the art changes):

   ```
   python scripts/build_editor_assets.py
   ```

   This reads `../ElementalLegends-art/sheets` (or a path given as the first argument) and
   writes `tools/level-editor/assets.js` with the terrain tilesets, enemy / object sprites,
   the mage and item icons, plus the format 2 art: pit / lava / thorns tiles, the orange and
   blue barrier pegs, the new objects (great chest, whip post, boulder, pot, crate, switches,
   crystals, ice block, lore tablet), the relic / key / map icons and every `prop_<theme>_*`
   sheet. Anything missing is skipped and drawn as a coloured shape (theme 10 has no water
   atlas, so its water is a plain colour). Needs Python 3 and Pillow.

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

## Format 2 lairs

Opening a file with `"format": 2` (or **New** > *Lair (format 2, free layout)*) switches to the
lair layout. A new format 2 lair starts with one entrance room (exit south) at grid cell 6, 11.

- **Room grid map** (above the room canvas). Every room is drawn on the 13 x 13 `at` grid as
  a rectangle of its `size`, with a miniature of its tiles, its id and badges: **IN**
  entrance (the first room with an exit), **B** boss stairs, **D** dark, **$** great chest.
  Doors sit on the room edges, coloured by lock: green open, yellow small key, red big
  key, grey shutter, brown bomb wall, violet flag, light blue exit (with an arrow), pink
  for a broken door. The map shows the rooms plus a one-cell margin; tick **Full grid**
  for all 13 x 13 cells. Rooms in red have Check errors.
  - Click a room (or a name in the list beside the map) to edit it; `[` / `]` step
    through the rooms.
  - Drag a room to move it. A move onto another room or off the grid is refused.
  - **Add room**: pick 1x1, 2x1, 1x2 or 2x2, press Add room, then click an empty spot
    (green outline = free). `Esc` cancels.
  - **Delete room** removes the selected room and every door that leads to it.
- **Room panel** (left): `id` (renaming it updates every door that points at it), `name`,
  size (resizing keeps the tiles that still fit; it is refused if the bigger room would
  overlap), dark, boss stairs, shutter (none / combat / switch / torches / crystals /
  plates) and random enemies.
- **Doors**: the list shows this room's doors (click one to edit it, x to delete it) and,
  in grey italics, doors declared by a neighbour that the game mirrors onto this room
  (click to jump to the room that declares it). To add one, pick the side and, for
  2-screen rooms, the segment; **To** offers only the room that touches there, plus
  `exit`; pick the lock (`none`, `small`, `big`, `shutter`, `bomb` or `flag` with a flag
  name). Declare a door once: the game adds the other side and copies the lock.
- **Room canvas**: 16 x 13 tiles per screen (a 2x2 room is 32 x 26). Extra brushes: pit
  `8`, lava `9`, orange barrier `0` (starts raised), blue barrier `U` (starts lowered),
  hidden bridge `H`, thorns `T`. With **Game features** on, door gaps are drawn as the game
  carves them (small lock, big lock, cracked bomb wall, flag bars), mirrored doors dashed,
  and the boss stairs at columns 7-8, rows 3-4.
- **Objects**: chest (contents now include `small_key`, `big_key`, `map`, `finder`),
  big_chest (`relic`, blank = the lair's relic), lever / floor_switch (`sets` a flag),
  crystal_switch, crystal (`element` and `order`), ice_block, whip_post, boulder, pot,
  crate, tablet (`text`) and prop (pick a `prop_<theme>_<name>` sheet, this lair's theme
  first, and untick **Solid** for rugs and floor art), plus torch, block and shrine.
- **Flag tiles** (bottom of the room panel): rules like "when `drain` is set, `~` becomes
  `.`" in this room.
- The level header gains **Relic** (what the great chest holds; none for the Dark Tower).

**Check** for format 2 follows the game's loader (`src/keepdef.rs`): duplicate or missing
ids, `at` / `size` limits, overlapping rooms, tile rows and characters for the room size,
unknown locks / shutters / objects / chest contents, doors whose `seg` is past the edge,
doors that don't touch the room they name, a missing exit, enemies or objects outside the
room or in walls, and then the solvability walk (`check_solvable`): every room reachable
with the small keys, big key and flags found on the way, the boss stairs reachable, and
the big key present if there is a great chest. Warnings cover things the game tolerates
(no boss stairs, blocked doorways, flags nobody sets, hidden chests in rooms without a
shutter, crystal orders, missing props...). Click an issue to select the room and flash
the spot (and the room on the map).

Saving writes the same layout as `scripts/lairkit.py` (one tile row per line; doors,
objects, enemies and flag tiles one per line; `at` / `size` pairs inline). Opening a
shipped `levels/lair_N.json` and saving it gives a byte-identical file.

## Where files go

- **Save** writes `cave_N.json` / `lair_N.json`. After **Choose levels folder** (Chrome /
  Edge) it saves straight into that folder and lists its files for opening; otherwise the
  file is downloaded and you move it yourself.
- Built-in levels live in the repo's `levels/` folder and are embedded in the game by
  `python scripts/import_levels.py` (rebuild afterwards).
- To test without rebuilding, put the file in a `levels/` folder **next to the game
  executable** or in the directory you run it from: a file there overrides the built-in one
  with the same name. A broken room is logged and ignored (the generated layout is used).

`levels/cave_2.json` is an example made with the editor. The format 2 lairs in `levels/`
were first drawn with `scripts/design_lairs.py`, but the JSON is the source of truth: edit
them here (re-running that script overwrites them).

The editor is `index.html` + `editor.css` + `editor.js` (format 1 and the shared painter)
+ `lair.js` (format 2: model, JSON writer, room map, doors, Check) + the generated
`assets.js`. It also works when served over `http://` (e.g. `python -m http.server`).
