# New project prompt: Elemental Legends II (a Gauntlet Legends-style sequel)

Paste everything below the line into a new Claude Code session, started in `D:\projects`.

---

## What I want

A second Elemental Legends game that plays like **Gauntlet Legends** (N64 / arcade, 1998-99), not like
A Link to the Past. The first game (`D:\projects\ElementalLegends`) is a Zelda-style adventure with one
mage, an open overworld, puzzles and relics. This one is an **arcade co-op dungeon crawler**: pick a
hero, drop into a level, carve through endless hordes, smash monster generators, grab keys, food and
treasure, find the exit, beat the realm bosses and collect their rune stones.

Working title: **Elemental Legends II** (suggest better names; I'll choose).
New project folder: `D:\projects\ElementalLegends2`. It must not touch or overwrite
`D:\projects\ElementalLegends` or `D:\projects\ElementalLegends-art`. You may *copy* code and tools from
them.

## How to work with me

- **Questions:** ask one at a time, using a question pop-up, with your recommended option first.
  Use Gauntlet Legends as the reference whenever a design choice is unclear.
- **Tuning:** keep it forgiving.
- **Reporting:** be honest about what was verified. Headless tests and screenshots are not playtesting
  or hardware testing.
- **Commits:** commit to `main` after each verified milestone, and keep an `IMPLEMENTATION_STATUS.md`.
- **First:** write a `CLAUDE.md` with the project rules (adapt the first game's), a `docs/DESIGN.md`,
  and a milestone plan. Show me the plan before building.

## Core gameplay (Gauntlet Legends features to capture)

- **Heroes:** four, each tied to one of the first game's elements. Each has a melee/ranged basic attack,
  a charged "turbo" attack (hold the button, as in Gauntlet Legends) and an element spell. A first
  proposal for you to refine:
  - **Pyre Knight** (fire, warrior): strong melee, slow.
  - **Frost Valkyrie** (ice, balanced): shield charge; freezes foes.
  - **Storm Archer** (storm, fast): long-range piercing arrows, weak armour.
  - **Earth Sage** (earth, wizard): area magic and the strongest spells, fragile.
- **Co-op:** 1-4 players on one screen (local), with a shared camera that keeps everyone in view, and
  drop-in/drop-out at any time. Single player must be fully enjoyable.
- **Health drains over time,** as in Gauntlet. Food restores it. Classic announcer lines show as big
  on-screen callouts ("Wizard needs food badly!", "Valkyrie is about to die!"). Spoken voice is optional
  later.
- **Monster generators:** bone piles, huts and portals spawn monsters endlessly until destroyed. Different
  generators spawn different monster tiers.
- **Hordes:** many weak enemies at once (dozens on screen), plus elites, ranged casters, ghosts that
  drain, and "Death", which steals health and can only be killed with a special item.
- **Keys and doors:** keys are a carried count, as in Gauntlet. Locked doors and treasure rooms.
  Destructible walls. Secret areas with bonus treasure.
- **Magic potions:** a screen-clearing element blast. Also amulets and power-ups: speed, invisibility,
  reflect shots, three-way shot, super shot, levitation.
- **RPG growth:** heroes level up and you spend points on strength, speed, magic and armour. Progress
  is saved per hero, so the same hero can be taken into any game.
- **Gold and shop:** a merchant between levels sells food, potions, keys, armour and turbo upgrades.
- **Structure:** a hub with four realms (matching the first game's regions: Greenwood, Old Crypt,
  Mirefen, Emberpeak). Each realm has 4-6 levels and ends in a boss, which drops a rune stone. With all
  four, the final realm opens (the Dark Tower, a callback to game 1).
- **Levels:** hand-designed, with secret areas and occasional set-piece traps (rolling boulders, spike
  floors, collapsing bridges). Each level is a timed run with a score screen (gold, kills, secrets).
- **Bosses:** big, multi-phase bosses in the Gauntlet Legends style, built to be fought by 1-4 players.
- **Optional extras:** unlockable secret heroes, and a survival/endless mode.

## Look and feel

- **Camera and art:** top-down / 3/4 view pixel art at the same resolution as game 1 (320x240 logical,
  24 px tiles) and an SNES-era look. Readable when the screen is full of enemies.
- **Shared assets:** the four elements, the rune stones and the Dark Tower tie the two games together.
  Reuse game 1's art where it fits (tilesets, monsters, items, UI frames), and make new art for the
  heroes and new monsters.
- **Music:** punchy, arcade-like, one track per realm (procedural synth like game 1's `audio.rs` is
  fine).

## Technology (same as game 1)

- **Language and libraries:** Rust and SDL2, with a software framebuffer and integer scaling. Don't
  switch engines without asking.
- **Build environment:** Windows plus Ubuntu WSL (`wsl -d Ubuntu`). Rust, SDL2, the aarch64 cross
  toolchain and the Android SDK/NDK are already installed there.
- **Platforms:** desktop (Windows and Linux), the ANBERNIC RG35XX Pro/H on Knulli with PortMaster (the
  main handheld; it must run at 60 fps with a full horde), and Android with a gamepad.
- **Reuse from game 1** (copy into the new repo; don't link to the old one):
  - **Engine:** the core pieces: `gfx.rs` (software renderer), `audio.rs` (synth + songs), `art.rs` + the
    `scripts/import_art.py` pipeline, input mapping, the save system, and the headless self-test
    harness (`snapshot.rs`: scripted play through the real game loop, plus PNG screenshots).
  - **Builds:** `scripts/build-handheld.sh`, `scripts/build-android.sh` + `android/`, the `port/` launcher
    (it runs with or without PortMaster and logs to `log.txt`), and `port.json` / `gameinfo.xml` /
    cover / screenshot.
  - **CI/CD:** `.github/workflows/ci.yml` and `release.yml` (Windows, Linux, handheld and Android
    builds; a version tag publishes a GitHub Release).
- **Art:** PixelLab via the existing tools in `D:\projects\ElementalLegends-art\tools` (copy them into a
  new art repo).
  - The API key is the Windows user env var `PIXELLAB_API_KEY`. **Never print, log or commit it.**
  - The PixelLab account is shared with other games: check the balance first and ask before any batch
    over about 150 generations.
  - Generate art with background agents, each with its own manifest file and budget.
  - Read my saved PixelLab tips (memory: `pixellab-api-tips`).
- **Co-op input:** up to 4 gamepads on desktop; the keyboard counts as player 1. On the handheld,
  player 1 uses the built-in controls and a second player can join with a USB/Bluetooth pad.
- **Performance:** design for hundreds of entities and particles on the RG35XX's H700 (4x Cortex-A53).
  Use pooled entities and spatial hashing for collisions, and cap particles.

## Repos

- **GitHub:** new repos `GrayHatLabs/ElementalLegends2` and `GrayHatLabs/ElementalLegends2-art`.
  Ask me whether they should be public or private. Game 1 is public with MIT code and CC BY 4.0 art.
- **Before every push:** scan for the PixelLab key.

## Suggested first milestones (refine them in the plan)

1. **Skeleton:** a new repo with the reused engine pieces; one hero walking and attacking in a test
   arena; the self-test harness running; desktop and handheld builds.
2. **Horde combat:** generators, 3 monster types, a health drain, food, keys and doors, potions, and a
   level exit with a score screen.
3. **Heroes:** all four heroes with basic, turbo and element attacks; levelling and stat points;
   per-hero saves.
4. **Co-op:** 2-4 local players, a shared camera, drop-in/out, and announcer callouts.
5. **Greenwood:** the realm's levels, its boss and the shop between levels.
6. **The remaining realms and the Dark Tower,** then polish, music, Android and CI/CD, and a first
   release.

## Open questions to ask me (one at a time)

- The final name.
- The hero lineup and whether there are secret heroes.
- Public or private repos.
- Whether the levels are fully hand-made (with a level editor like game 1's) or hand-made plus some
  procedural rooms.
- The number of levels per realm.
- Whether game-1 saves unlock anything (a bonus for owning both).
