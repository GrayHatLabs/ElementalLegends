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
- **Mana is precious.** It refills slowly, about 0.5 MP per second. Carry up to 5 mana potions (buy them at the shop, or find them in chests and from monsters) and drink one with the potion button when you need a spell.
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
| Drink mana potion | Y / L1 / R1 | C / L / Shift |
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

### Playtesting

`docs/QA_GUIDE.md` has the spoilers: world map, main route, lair walkthroughs, caves and mini-bosses.

### Android (handhelds with a pad on stock Android, phones with a controller)

From WSL, with the Android SDK/NDK installed (the same one-time setup as Ashen Sanctum's
`scripts/setup-android.sh`):

```bash
scripts/build-android.sh          # builds dist/ElementalLegends.apk (arm64 + x86_64)
scripts/android-emulator-test.sh  # boots an emulator, installs and screenshots it (dist/android_test*.png)
```

Copy the APK to the device and install it (allow installs from unknown sources). It is signed with a
local sideload key (`android/sideload.keystore`, made on first build, kept out of git). The game needs
a gamepad or built-in controls; there are no touch controls. Saves live in the app's own storage.

### Downloads and CI

- **CI** (`.github/workflows/ci.yml`): every push to `main` runs the unit tests and the headless self-test.
- **Release** (`.github/workflows/release.yml`): pushing a version tag (`git tag v0.5.0 && git push origin v0.5.0`)
  builds Windows (.exe, SDL built in), Linux x86_64 (static SDL), the handheld PortMaster zip (aarch64) and
  the Android APK, and publishes them on a GitHub Release. Running it by hand from the Actions tab
  uploads the builds as workflow artifacts only. Optional repo secret `ANDROID_KEYSTORE_B64` (base64 of a
  keystore with alias/passwords `elementallegends`) keeps Android updates installable over older versions.

### Tests

`scripts/test.sh [screenshot-dir]` runs the unit tests and the headless self-test. The self-test plays through every dungeon and boss and checks the acceptance criteria. Current status is in `IMPLEMENTATION_STATUS.md`.

### Headless snapshot test

`elementallegends --snapshot <dir>` plays a scripted run with no window and writes
PNG screenshots. Useful for checking rendering without a display.

## License

Code: MIT. Art and music: CC BY 4.0 (credit "Elemental Legends by GrayHatLabs"). See `LICENSE` and `LICENSE-ART`.
