# ELEMENTAL LEGENDS

An NES-style fantasy action adventure: Zelda-style screen-by-screen exploration
meets Gauntlet-style monster hordes. You are a mage who wields one of four elements.

Written in Rust with SDL2. Renders a 256x240 NES-sized framebuffer in software,
integer-scaled (2x on the Anbernic RG35XX H's 640x480 screen). All art, music and
sound are generated in code, so there are no asset files.

## How to play

- **You awaken at an ancient monolith.** Touch it to restore your strength. It lights a rune for every lair you conquer. The village shop is one screen away.
- **Choose an element** at the start: FIRE, ICE, STORM or EARTH.
  - FIRE: firebolts that set foes burning (damage over time), spell FLAME RING
  - ICE: the first hit chills (slows), a second hit freezes solid; the ice shatters when it breaks. Spell FROST NOVA
  - STORM: fast piercing bolts, spell CHAIN BOLT
  - EARTH: heavy knockback boulders, spell QUAKE
- **Elemental weaknesses:** fire and ice are opposites, and so are storm and earth.
  Hitting an enemy with its opposite deals **2x damage** (WEAK!). Hitting it with its
  own element deals **half damage** (RESIST). The enemy's colour shows its element.
- **Change element** by touching an orb shrine (a floating orb on a pedestal) or by
  picking up an orb dropped by an elemental monster.
- **You need to eat.** The food bar drains over time. When it's empty you starve
  and lose life. Eat apples, bread and roasts, or buy food in the starting village shop.
- **Treasure:** chests, gold coins, gems, dragon hoards and heart containers are
  hidden around the realm. Spend gold at the shop (stand on an item and press A).
- **Monster generators** (skull stones) keep spawning enemies until you destroy them.
- **Five lairs and the Dark Tower** are dungeons behind fantasy buildings. Each has a hall with a locked door, two puzzle chambers, and a sealed staircase. The west chamber hides the key and the east chamber breaks the staircase seal. Puzzles include combat seals, braziers to light with fire, blocks to push onto pressure plates, rivers to freeze into ice bridges, and cracked walls hiding levers. Puzzles that need an element come with its orb shrine.
- **Bosses** (Ancient Treant, Undead Guardian, Stone Golem, Crimson Dragon, Arcane Sorcerer, Dark Sorcerer) wait at the bottom of the stairs. Watch for the red **!** and ground markers before big attacks. Beating one gives a rune, a power-up and more life, and unseals the next lair.

## Controls

| Action | Handheld | Keyboard |
|---|---|---|
| Move | D-pad / left stick | Arrows / WASD |
| Cast bolt (hold to strafe) | A or X, R2 | Z / J / Space |
| Element spell (costs MP) | B, L2 | X / K |
| Map / pause | START | Enter / Esc |
| Mute | SELECT | M |
| Quit | SELECT + START | close window |

Progress saves automatically to `elementallegends.sav` next to the executable.

## Building

### Desktop (Linux / WSL)

```bash
sudo apt install libsdl2-dev build-essential
curl https://sh.rustup.rs -sSf | sh
scripts/dev-build.sh run
```

On Windows 11 you can run it inside WSL (WSLg shows the window and plays sound).

### Windows (native)

With CMake installed: `cargo run --release --features bundled`

### Anbernic RG35XX H (and other aarch64 Linux handhelds)

From Ubuntu 22.04 / WSL:

```bash
scripts/setup-cross.sh      # one-time: arm64 SDL2 + cross linker + rust target
scripts/build-handheld.sh   # builds dist/ElementalLegends-aarch64.zip
```

Unzip `dist/ElementalLegends-aarch64.zip` onto the SD card so that
`ElementalLegends.sh` and the `elementallegends/` folder land in your **ports**
folder. It uses the standard PortMaster launcher (install PortMaster from your
firmware's app menu first). This works on muOS, Knulli and ROCKNIX, and on the stock
OS with PortMaster installed. It then appears under **Ports**.

The binary links to the system `libSDL2-2.0.so.0` and needs glibc 2.35 or newer
(it's built on Ubuntu 22.04).

### Tests

`scripts/test.sh [screenshot-dir]` runs the unit tests and the headless self-test. The self-test plays through every dungeon and boss and checks the acceptance criteria. Current status is in `IMPLEMENTATION_STATUS.md`.

### Headless snapshot test

`elementallegends --snapshot <dir>` plays a scripted run with no window and writes
PNG screenshots. Useful for checking rendering without a display.
