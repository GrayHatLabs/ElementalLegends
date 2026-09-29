# CLAUDE.md — Elemental Legends

Project rules and reference. Read this before every major change.

## Platform and Technology

- Primary language: Rust.
- Rendering/input: SDL2.
- Genre: Top-down fantasy action RPG inspired by Zelda and Gauntlet Legends.
- Primary development environment: Windows with Ubuntu WSL.
- Desktop testing must remain supported.
- Handheld target: ANBERNIC RG35XX H.
- Distribution: PortMaster-compatible aarch64 package.
- Preserve the existing handheld build script (`scripts/build-handheld.sh`).
- Do not migrate to C++ or another engine without explicit approval.
- Design new effects and animations with handheld performance in mind.

## Existing Gameplay That Must Be Preserved

- Four elements: Fire, Ice, Storm, and Earth.
- Element selection and switching through orb shrines and enemy drops.
- Elemental weaknesses and resistances.
- Flame Ring, Frost Nova, Chain Bolt, and Quake.
- Food depletion, health loss, and consumable food.
- Shop purchasing.
- Treasure chests, gems, gold, and hidden hearts.
- Monster generators.
- Five elemental boss lairs and the Dark Tower.
- Boss rune rewards and power-ups.
- Gauntlet-style strafing: holding cast locks facing while allowing sideways movement.
- Existing save-game functionality.

## New Development Requirements

All previously specified improvements remain required:

1. Animated firebolt projectiles and burning damage.
2. Two-hit ice mechanic with slow, freeze, and shatter effects.
3. Monolith starting environment instead of starting inside the shop.
4. Fantasy dungeon buildings and entrances.
5. Dungeon exploration, keys, locked doors, and elemental puzzles.
6. Animated staircase transitions into boss arenas.
7. Fantasy bosses including treants, dragons, golems, undead guardians, and sorcerers.

## Critical Development Rules

Before every major change:

- Read the existing requirements.
- Inspect the current implementation.
- Preserve existing gameplay mechanics.
- Verify that the desktop build still works.
- Verify that the handheld cross-build still succeeds.
- Do not mark features complete based solely on headless testing when visual or hardware verification is required.
- Update IMPLEMENTATION_STATUS.md.
- Commit completed and verified milestones to Git.

## Current Repository

- Project: `D:\projects\ElementalLegends`
- Previous known baseline commit: `cb63329` — Initial version of Elemental Legends
- The separate NovaGuardian browser project (`D:\projects\NovaGuardian`) must not be overwritten or confused with Elemental Legends.

## Priority

Develop one shared Rust game codebase that supports both desktop and handheld builds. Avoid maintaining two separate gameplay implementations.

## Build & test commands (WSL Ubuntu)

- Desktop build/run: `scripts/dev-build.sh` / `scripts/dev-build.sh run`
- Handheld cross-build + PortMaster zip: `scripts/build-handheld.sh` (one-time setup: `scripts/setup-cross.sh`)
- Unit tests: `cargo test`
- Headless screenshots: `elementallegends --snapshot <dir>`
- Headless acceptance checks: `elementallegends --selftest`
