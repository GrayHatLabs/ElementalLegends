# Implementation Status — Elemental Legends

Last updated: 2026-09-30 (branch `snes-upgrade`)

## Verification levels

| Level | Meaning |
|---|---|
| **H** | Headless-verified: `scripts/test.sh` (13 unit tests + 310 scripted self-test checks through the real game loop, real input and collisions) passes |
| **S** | Screenshot-reviewed: rendered frames from the self-test inspected by eye |
| **D** | Desktop-played: run interactively in the SDL window on desktop (WSL) |
| **HW** | Verified on the ANBERNIC RG35XX H hardware |

Headless checks prove logic and render output. They do **not** prove feel, timing, audio, readability at real size, or handheld performance. Nothing is marked HW yet.

## Builds

| Item | Status |
|---|---|
| Desktop build (`scripts/dev-build.sh`) | Builds clean, no warnings |
| Handheld cross-build (`scripts/build-handheld.sh`) | Succeeds: aarch64 ELF + `dist/ElementalLegends-aarch64.zip`, needs glibc ≥ 2.35 |
| Running on RG35XX H | **Not yet verified (needs hardware)** |

## SNES upgrade (branch snes-upgrade, not merged — user plays it first)

Decisions (2026-09-30): 24px tiles, 320x240, A Link to the Past camera, bigger 8x8 world, keep the grey wizard art, ALttP look, SNES-style music, optional twin-stick, fresh saves.

| Milestone | Status | Notes |
|---|---|---|
| M1 Engine: 320x240 framebuffer (fills the 640x480 screen at 2x), camera + 1.5x world zoom (16-unit tiles drawn 24 px), follow camera, SNES slide between areas | H S | Game logic unchanged (same units), so all balance/puzzles/bosses keep working. Text and HUD drawn crisp at screen resolution |
| M1 World: 8x8 cells, 11 wilderness areas of 2x2 screens + single screens (31 areas), doorway links per edge segment, shop always beside the monolith | H S | Specials (buildings, shop, monolith, encounters, hearts, hoards) stay on single screens; big areas get 4 obstacle patterns, bigger hordes, 2 generator rolls and a chest. Unit tests check coverage, reachability, symmetric links and carved gaps |
| M2 Art pipeline (native 24px PNG sheets + manifest, fallback to code-drawn art) | H S | `scripts/import_art.py` embeds art in the binary. Generated wizard in game. Dual-grid corner-tileset terrain renderer ready for the generated tilesets |
| M3 Generated terrain (10 themes) | H S | PixelLab floor/wall and floor/water(lava) corner tilesets for all 10 themes + 12 overworld props (135 generations). Dual-grid edges, procedural floor detail, monolith wildflowers. Dungeon light radii scaled to 75% for the zoomed view. Weaker themes noted in `ElementalLegends-art` report: Greenwood wall repetition, Mirefen banks, Shrine walls look like the crypt, Dark Tower low contrast |
| M4 Generated characters, enemies, bosses, mini-bosses, buildings, props, items | H S | PixelLab art for every enemy kind in 5 element colours (4-direction skeleton/imp/zombie, 1-direction bat/slime/ghost/golem, generator), merchant, dryad, treant, hoard dragon, grave lord, the 6 bosses (idle/attack/hurt), the 6 lair buildings, monolith, standing stones, cottage, shrine pedestal, braziers, push blocks, levers, fruit trees, tombstones, 14 items and chests (~125 generations for this milestone). Camera keeps tall set pieces, bosses and mini-bosses in frame. Hit flash is now an SNES-style blink. Weaker pieces: Guardian (bone not rusted plate), Sorcerer (no staff), Golem attack (core flare, not a fist slam), Dark Tower building (low contrast on its dark ground), cottage (townhouse look), some enemies larger than planned (imp, bat, skeleton) |
| M5 SNES-style music and sound | H | Stereo synth with ADSR instruments (flute, strings, harp, brass, organ, bells, basses, drum kit), SNES-style echo, 8 themes (title, village, Greenwood, Old Crypt, Mirefen, Emberpeak, dungeon, boss) chosen by location. `--render-music <dir>` writes WAVs; unit test checks every song is audible without clipping. **Not yet listened to by a human** |
| M6 Optional twin-stick aiming (RG35XX Pro right stick) | H | Right stick faces and casts that way while the left stick moves; unused without a right stick, so the H plays as before. Needs a hardware check of the Pro's axis mapping |
| Region ambience + full-width dialogue box | H S | Leaves (Greenwood), mist (Old Crypt), fireflies (Mirefen), embers (Emberpeak) with parallax; frame-counter driven, no RNG. SNES dialogue box across the screen |
| HUD redesigned for 320 width | H S | SNES layout: element orb + vertical magic meter, food bar, potions / runes (keys in dungeons) / gold, -LIFE- rows of hearts with half hearts. Menus and messages still use the centred 256-wide layout |

### Judgement calls made while the user was away (please review)

1. **Logic units unchanged, rendering zoomed 1.5x.** Instead of rescaling every speed/hitbox/puzzle for 24 px tiles, the game still thinks in 16-unit tiles and the renderer draws them 24 px wide. Same balance, same puzzles; the view shows ~13x9 tiles.
2. **Single screens are 16x13 tiles (bigger than the view), wilderness areas 32x26.** So every area scrolls a little; the monolith, shop, lair buildings, hearts, hoards and mini-boss screens stay single screens so their hand-made layouts keep working. Dungeon rooms and boss arenas also scroll with the camera.
3. **World: 31 areas (11 big) on an 8x8 grid**, shop always the screen west of the monolith.
4. **Terrain: 32 px generation downscaled to 24 px locally** (PixelLab only makes 16/32 px tilesets; its resize endpoint wrecked tiles).
5. **Old code-drawn tiles/sprites stay as fallbacks** for anything without generated art, scaled 1.5x (they look soft/blocky next to native art until replaced).
6. **Dungeon light radius shrunk to 75%** so the zoomed view keeps its dark edges.
7. **Twin-stick aim auto-casts** while the right stick is held (like a twin-stick shooter); left stick/d-pad still moves.
8. **Music:** 8 new themes composed in code; boss music is the loudest. --render-music <dir> renders WAVs for listening.
9. **Screen shake no longer uses the game RNG** (bug: rendering changed gameplay).
10. **Title/menus/map still use the 256-wide layout, centred** (HUD and dialogue box are full width).
11. **Slime is hand-drawn in code, not generated** (three PixelLab slimes came out noisy); recoloured per element like the others.
12. **Camera framing:** rooms with a lair building, the monolith or the shop hold the camera at the top while the mage is on screen; boss and mini-boss fights frame both the foe and the mage.
13. **Dryad's true form** reuses her sprite with a dark green tint (no separate generation); the "no shadow while disguised" tell is kept.
14. **Oversized sprites were not downscaled** (imp, bat, skeleton, zombie, hoard dragon, grave lord are 15-30% bigger than planned) because non-integer NEAREST scaling damaged the art; hitboxes are unchanged.

## Preserved gameplay

| Mechanic | Status | Notes |
|---|---|---|
| Four elements, choose at start | H S | |
| Element switching (orb shrines, enemy orb drops) | H | Dungeon puzzles use shrines |
| Weaknesses / resistances | H | Unit-tested multipliers; boss weaknesses shown in intro banner |
| Flame Ring / Frost Nova / Chain Bolt / Quake | Code preserved | Frost Nova now also freezes water; Quake breaks cracked walls and shatters frozen foes. Not individually scripted in the self-test |
| Food depletion, starvation, eating | H | |
| Shop purchasing | H S | Shop moved to the village screen next to the monolith |
| Chests, gems, gold, hidden hearts, hoards | Code preserved | |
| Monster generators | Code preserved | |
| Five lairs + Dark Tower, sequential unlock | H | All six are now dungeons with a boss |
| Rune rewards and power-ups | H | Reward screen reached for all six bosses |
| Strafing (hold cast locks facing) | H | Checked during every boss fight |
| Save games | H | Round-trip unit test. Saves now carry `version=2`; saves from the old 6x6 world are detected and ignored (user decision: fresh saves are fine) |

## New features

| # | Feature | Status | Notes |
|---|---|---|---|
| 1 | Animated firebolt (travels, flame trail, embers, impact burst) | H S | Distinct halo and pulsing core |
| 1 | Burning: ignite, flames follow enemy, damage over time, fade-out, refresh without stacking | H S | Unit tests cover no-stacking. Bosses burn too |
| 2 | Ice shard projectile + crystal burst on impact | H S | |
| 2 | First hit chills (45% slow, frost tint, ice particles) | H S | |
| 2 | Second hit freezes (ice block encasing animation, no movement/attacks) | H S | Bosses freeze 1 s with a 6 s cooldown |
| 2 | Shatter on thaw (fragments fly out and fall) | H S | Fire/earth hits also break ice early |
| 3 | Monolith start area + opening cinematic (sky pan, runes, light beam) | H S | Monolith shows a rune slot per conquered lair and restores HP/MP on touch |
| 3 | Shop found through exploration | H S | One screen from the monolith |
| 4 | Fantasy dungeon buildings (shrine, crypt, castle, fortress, sanctuary, tower) | H S | Sealed / open / cleared states. Entry walk-in + fade |
| 5 | Dungeons: key, locked door, sealed stairs | H S | 5-room layout per dungeon, progress saved |
| 5 | Puzzles: combat seal, torches (fire), pressure plates + blocks, ice bridge, hidden cracked wall + lever | H S | Each dungeon solved end-to-end by the self-test. Element puzzles include the needed shrine. Unsolved block rooms reset on re-entry (no soft-lock) |
| 6 | Staircase cinematic (activate, auto-walk, sink, camera follow, darkening) → boss intro | H S | Input ignored during the sequence |
| 8 | Feast hall (dungeon 1) and hidden pantries behind cracked walls (dungeons 2-6) | H S | Safe rooms off the entrance hall. Uneaten food stays while you're in the dungeon and restocks on your next visit |
| 9 | Bolt range: about 5 tiles at magic level 1, 7.5 at level 2, 12 at level 3. Shrinks to about half at low health | H | Range measured by the self-test: 87 / 47 (low HP) / 197 px. Shortened again after playtest feedback |
| 10 | Mana as a resource: 0.5 MP/s regen, carry up to 5 potions, drink with the potion button (Y/L1/R1, keyboard C/L/Shift) | H S | Shop potions go to the pack. Pickups go to the pack, or give +30 MP when it's full. No drink at full MP |
| 11 | Graphics pass 1: auto-shaded sprites with coloured outlines, 4-frame mage walk cycle, drop shadows, textured tile variants, raised masonry walls, dungeon/arena lighting (light map with torches, bolts, burning foes, shrines, stairs, boss), polished HUD | H S | Measured update+draw about 0.26 ms/frame on desktop (p99 about 0.4 ms). **Handheld frame time not yet measured on hardware** |
| 7 | Bosses: Treant, Undead Guardian, Stone Golem, Crimson Dragon, Arcane Sorcerer, Dark Sorcerer | H S | Each: telegraphs, ≥3 attacks, 2 phases (Dark Sorcerer 3), hit flash, intro, defeat sequence |

## Side content (2026-09-30 plan, on branch snes-upgrade)

The rune/monolith start is unchanged (user decision).

| Step | Status | Notes |
|---|---|---|
| 1 Village: shop moved 2-3 areas (and at least 2 map squares) from the monolith; inn and notice board | H S | Inn: full life, magic and cure for 10 gold. Board: runes, caves cleared, spellbook pages. PixelLab inn and board art. Dialogue box moves to the top when the mage is low on screen (as in A Link to the Past) |
| 2 Relic bag: potion, antidote, bomb, elixir | H S | Y/L1 (keyboard C) uses the selected item; Select (Tab/V/Q) cycles; Left/Right pick it on the pause map; Select while paused mutes. Bombs: 1.5 s fuse, 8 damage in a 30-unit blast, break cracked walls. Shop has 6 stands (bomb 30, elixir 120). Bombs and elixirs are code-drawn icons for now |
| 3 Optional caves (8) | H S | 2-3 rooms each: sealed fight or freeze-plate puzzle (freeze a tough monster and shove it onto each plate; frozen monsters are solid and pushable in caves; lost puzzle monsters are replaced so there is no soft-lock), then a treasure chest (bombs, elixirs, heart container or mana crystal). Map marker C, flag once cleared. PixelLab cave-mouth art per region. Cave walls reuse lair tilesets (a dedicated rock tileset would look more cave-like) |
| 4 Spellbook side quest and Arcane Blink | H S | 5 pages in the chests of caves 2, 3, 4, 6 and 8; the village scholar teaches Blink (R1 / keyboard E): up to 3 tiles, 6 MP, through monsters, bullets and water but not walls. Quest line on the pause map |
| Later puzzle batch: sliding ice blocks, element crystals | Queued | User picked these for the next batch |

## Requested backlog (in agreed order; report back after each)

| Item | Status |
|---|---|
| Shorter bolt range (playtest feedback) | Done (`ddfa99b`) |
| Single bolt until the end stages; twin bolts only from the 4th lair power-up, no triple shot (playtest feedback) | Done |
| Overworld encounters: Hoard Dragon, Deceiving Dryad (+ poison status, antidote), Food Trees + Angry Treant, Graveyard + Grave Lord — one-time, save-flagged, map markers, old saves load | H, V (snapshots 50–61 reviewed); not yet playtested for feel or on hardware |
| Better-looking dungeon entrances (playtest feedback) | Done in the SNES upgrade (generated building art) |
| Less childish boss art (playtest feedback) | Done in the SNES upgrade (generated boss art) |
| Mini-boss batch 1: Mimic Chest, Treasure Goblin, Bandit Raccoon, Wandering Merchant Ogre | Queued |
| Mini-boss batch 2: Mushroom Ring Fairy King, Honey Bear, Headless Knight, Banshee | Queued |
| Mini-boss batch 3: Bog Witch, Giant Toad, Will-o'-wisp, Salamander Queen, Lava Golem Forge, Phoenix | Queued |
| Mini-boss batch 4: Doppelganger Mage | Queued |

Playtest notes: the player likes the mana potions, and the lighting is "just about right".

## Known issues / next steps

- **Hardware test needed:** performance with heavy particle scenes on the H700 (particle cap is 700), button mapping, and fullscreen scaling.
- **Needs playtesting:** boss difficulty and balance, puzzle clarity, and how the cinematics pace and read at full size.
- **Visual polish:** the Dragon's wing membranes render as fanned lines, and the building art is fully procedural.
- **Fixed after playtest (2026-09-29):** the mage could get trapped inside a combat room's portcullis while standing in the doorway. The mage is now pushed into the room, and a regression check covers it.
- **Fixed:** a dungeon lever behind a hidden wall couldn't be pulled because its touch range was too small.
