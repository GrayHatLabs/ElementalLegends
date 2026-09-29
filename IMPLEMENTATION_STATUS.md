# Implementation Status — Elemental Legends

Last updated: 2026-09-29

## Verification levels

| Level | Meaning |
|---|---|
| **H** | Headless-verified: `scripts/test.sh` (8 unit tests + 303 scripted self-test checks through the real game loop, real input and collisions) passes |
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
| Save games | H | Round-trip unit test. Old saves without dungeon progress still load |

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

## Requested backlog (in agreed order; report back after each)

| Item | Status |
|---|---|
| Shorter bolt range (playtest feedback) | Done (`ddfa99b`) |
| Single bolt until the end stages; twin bolts only from the 4th lair power-up, no triple shot (playtest feedback) | Done |
| Overworld encounters: Hoard Dragon, Deceiving Dryad (+ poison status, antidote), Food Trees + Angry Treant, Graveyard + Grave Lord — one-time, save-flagged, map markers, old saves load | H, V (snapshots 50–61 reviewed); not yet playtested for feel or on hardware |
| Better-looking dungeon entrances (playtest feedback) | Queued |
| Less childish boss art (playtest feedback) | Queued |
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
