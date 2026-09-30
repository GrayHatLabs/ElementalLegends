# Elemental Legends level format (version 1)

Hand-made rooms for **caves** and **lair dungeons** are stored as JSON, one file per
cave or lair. The overworld stays procedurally generated.

- Built-in levels live in `levels/` in the repo and are embedded in the game by
  `python scripts/import_levels.py`, which writes `src/levels_gen.rs`.
- At start-up the game also looks for a `levels/` folder next to the executable, and
  then in the current directory. Any file found there **overrides** the built-in one with
  the same name, so you can test edits without rebuilding.
- A cave or lair with no level file uses the built-in generated layout.
- The level editor (`tools/level-editor/index.html`) reads and writes these files.

## File names

| File | What it defines |
|---|---|
| `cave_1.json` .. `cave_8.json` | Optional caves, numbered from the nearest (1) to the farthest (8) |
| `lair_1.json` .. `lair_6.json` | Lair dungeons 1-5 and the Dark Tower (6); the boss arenas are not editable |

## Top level

```json
{
  "format": 1,
  "kind": "cave",
  "number": 2,
  "name": "WHISPERING GROTTO",
  "theme": 4,
  "rooms": { "mouth": { ... }, "challenge": { ... }, "treasure": { ... } }
}
```

| Key | Required | Meaning |
|---|---|---|
| `format` | yes | Always `1` for this version |
| `kind` | yes | `"cave"` or `"lair"` |
| `number` | yes | Cave 1-8 or lair 1-6; must match the file name |
| `name` | no | Shown when entering (caves) and on the dungeon map. Upper-case letters, digits and basic punctuation only |
| `theme` | no | Wall/floor tileset index (see *Themes*). Default: the cave's region, or the lair's own set |
| `rooms` | yes | Rooms by slot name (see below). Slots you leave out keep their generated layout |

### Room slots

Rooms are laid out on a small grid and connected through the middle of their shared
walls. The door gaps are carved automatically, so don't draw them.

**Caves** (the challenge must be beaten to open the north door to the treasure):

| Slot | Position | Notes |
|---|---|---|
| `mouth` | bottom | Optional. With it the cave has three rooms (mouth, challenge, treasure); without it, two (the challenge room is the entrance). Exit south to the overworld |
| `challenge` | middle (or bottom) | Needs `"puzzle"`: `"combat"`, `"freeze_plate"` or `"none"` |
| `treasure` | top | Should hold a `chest`; its `contents` is the cave's reward |

**Lairs** (same flow as the built-in lairs):

| Slot | Position | Rules the game adds |
|---|---|---|
| `entry` | bottom centre | Exit south to the overworld. In lairs 2-6 the east wall hides the pantry behind a cracked stretch |
| `hub` | centre | The north door is locked until the key is found. `"combat": true` seals it for a fight on the first visit |
| `west` | left | Its puzzle reveals or unlocks the **key chest** (a `chest` with `"contents": "key"`) |
| `east` | right | Solving its puzzle breaks the seal on the stairs |
| `stairs` | top | The boss staircase always sits at columns 7-8, rows 3-4 |
| `feast` | lower left, lair 1 only | Food room |
| `pantry` | lower right, lairs 2-6 | Hidden food room |

## A room

```json
{
  "tiles": [
    "################",
    "#..............#",
    "#..#........#..#",
    "#..............#",
    "#....o....o....#",
    "#..............#",
    "#..............#",
    "#..............#",
    "#..~~~~~~~~~~..#",
    "#..............#",
    "#..#........#..#",
    "#..............#",
    "################"
  ],
  "puzzle": "freeze_plate",
  "combat": false,
  "enemies": [
    { "kind": "slime", "element": "ice", "x": 5, "y": 9, "hp": 40, "still": false },
    { "kind": "golem", "x": 10, "y": 9 }
  ],
  "random_enemies": 0,
  "objects": [
    { "type": "shrine", "x": 3, "y": 8, "element": "ice" },
    { "type": "chest", "x": 8, "y": 5, "contents": "bombs:5", "hidden": false }
  ]
}
```

### `tiles`

13 strings of 16 characters: row 0 is the top wall, and the outer ring should be `#`.

| Char | Tile | Notes |
|---|---|---|
| `.` | floor | |
| `#` | wall | |
| `~` | water | Blocks walking; Ice magic freezes it into a bridge |
| `i` | ice | Walkable frozen water |
| `c` | cracked wall | Breaks after three bolts (two for Earth), with Quake or with a bomb |
| `o` | pressure plate | Pressed by a push block, or in caves by a frozen monster |
| `D` | decoration | Solid and drawn as floor, for furniture footprints |

Door gaps, locked doors, seals and stairs are placed by the game.

### `puzzle`

| Value | Solved when | Where |
|---|---|---|
| `"none"` | never needed | any room except lair west / east |
| `"combat"` | every monster in the room is defeated (doors seal on entry) | cave challenge, lair west / east (the lair hub uses `"combat": true` instead) |
| `"torches"` | every `torch` is lit with fire | lair west / east |
| `"plates"` | every `o` has a `block` on it | lair west / east |
| `"icebridge"` | the reward is reached across frozen water | lair west / east |
| `"hidden"` | the reward is reached behind a cracked wall | lair west / east |
| `"freeze_plate"` | every `o` has a frozen monster on it | caves |

A lair's east room rewards you by breaking the stairs seal. `icebridge` and `hidden` puzzles
there need a `lever` to pull. A west room's reward is the key chest, which stays hidden
until the puzzle is solved if it has `"hidden": true`.

### `enemies`

| Key | Default | Values |
|---|---|---|
| `kind` | required | `slime`, `bat`, `skeleton`, `imp`, `ghost`, `golem`, `zombie`, `generator` |
| `element` | the kind's usual element | `fire`, `ice`, `storm`, `earth`, `neutral` |
| `x`, `y` | required | Tile column 0-15 and row 0-12, placed at the tile centre |
| `hp` | the kind's usual HP (grows with the region) | Number |
| `still` | `false` | `true` pins it in place |

Listed enemies appear every time the room is entered (unless it is a solved combat
room). `random_enemies` adds that many extra region-appropriate monsters at random free
spots; it defaults to `0` when `enemies` is given, otherwise to the usual count.

### `objects`

| `type` | Extra keys | Notes |
|---|---|---|
| `torch` | `lit` (false) | Brazier; fire bolts light it |
| `block` | | Push block (lair `plates` puzzles) |
| `lever` | `pulled` (false) | Pull by walking into it |
| `chest` | `contents`, `hidden` (false) | See below |
| `shrine` | `element` | An orb that switches the mage's element when touched |

Chest `contents`: `key` (lair west room), `gold:N`, `bombs:N`, `elixir`, `elixirs:N`,
`heart` (heart container), `mana` (+10 max magic), `potion`, `page` (a spellbook page,
if that cave holds one). In caves, the treasure chest also gives the cave's spellbook page
automatically.

## Themes

| Index | Tileset | Index | Tileset |
|---|---|---|---|
| 0 | Greenwood | 6 | Ruined castle |
| 1 | Old crypt | 7 | Dragon fortress |
| 2 | Mirefen | 8 | Forgotten sanctuary |
| 3 | Emberpeak | 9 | Dark tower |
| 4 | Overgrown shrine | 10 | Village shop |
| 5 | Underground crypt | | |

## Validation

The game never crashes on a bad file. It logs a warning, ignores the broken room (or the
whole file if the top level is invalid), and uses the generated layout instead. The
editor's **Check** button reports the same problems before you save:

- rows that aren't 13 × 16
- unknown characters, kinds, elements or contents
- a `plates` puzzle with fewer blocks than plates, or a `freeze_plate` room without plates
- a lair west room with no key chest
- enemies placed on walls
