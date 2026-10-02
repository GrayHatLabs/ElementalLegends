# Elemental Legends: QA Guide (spoilers)

The in-game hints stay as riddles on purpose. This guide gives the plain answers so testing
doesn't stall. It describes the current world (save version 4, 40 areas). If the world
layout changes, regenerate the map section with:

```bash
cargo test --release world_guide -- --ignored --nocapture
```

## Controls

| Action | Handheld | Keyboard |
|---|---|---|
| Move | D-pad | Arrows / WASD |
| Cast bolt (also talk, buy, read) | A (or X) | Z / J / Space |
| Element spell (costs magic) | B | X / K |
| Use the selected bag item | Y (or L1) | C / L / Shift |
| Pick the next bag item | Select | Tab / V / Q |
| Arcane Blink (after the spellbook quest) | R1 | E |
| Pause: item screen, A flips to the map | Start | Enter / Esc / P |
| Quit (handheld) | Select + Start | |
| Aim bolts (RG35XX Pro) | Right stick | |

**Bag items**, in order: mana potion, antidote, bomb, elixir, ration. After that come the Vine
Whip and the Feather Cloak once found. To use one, pick it with Select and press Y.

## Core rules

- **Life:** 5 hearts of four quarters each (4 HP per heart). A heart container adds one heart.
- **Food** drains slowly; starving hurts.
  - **Rations:** buy them at the shop, carry up to 5, and eat one with Y for +40 food.
  - **Pots** in lairs can hold bread or a roast.
  - **Other food:** monsters drop apples, fruit trees can be shaken, and the inn is a full rest.
- **Elements:** fire, ice, storm (lightning) and earth. Each beats one other: fire beats ice
  foes, ice beats fire, storm beats earth and earth beats storm. Orb shrines switch your element.
- **Bolt damage:** fire 2.0, ice 2.0, storm 1.7, earth 5.8. Ice chills, then freezes; earth
  shatters frozen foes; storm pierces.
- **Freeze-plate puzzles:** hit a monster with ice twice to freeze it, then push or knock it
  onto the pressure plate.
- **Runes:** each lair boss gives a rune. The Dark Tower (lair 6) opens only with all five.
- **Lair items:** each lair has a map and an item finder in chests, small keys for small locks,
  and a big key for the big chest (the lair's relic) and the boss door. The finder chimes in
  rooms with treasure and marks chests on the map.

## World map

Areas are numbered. Each square is one map cell, and the big areas cover 2x2 cells. North is
at the top. You start at the monolith, area 37, at the bottom middle.

```text
        x=0        x=1        x=2         x=3        x=4        x=5         x=6        x=7
y=0   8 Mimic    9 Cave 3   10 Banshee  [ 3 big: Cave 4      ] 11 Hoard   12 LAIR 4   13
y=1  [ 4 big           ]   14 Toad     [ 3                  ] 15 Pool     16 LAIR 3   17 Cave 5
y=2  [ 4               ]   18 Doppel   19 Trees    20 Cave 6   21 Dryad  [ 7 big: Wisp        ]
y=3  22 Treant  23 LAIR 1 [ 6 big: Fairy Ring   ] 24 Bear     25 Heart  [ 7                  ]
y=4  [ 5 big: Cave 1   ] [ 6                    ] 26 LAIR 5  [ 0 big: Bog Witch ] 27 Cave 7
y=5  [ 5               ]   28 Knight   29 VILLAGE  30 LAIR 6 [ 0                ] 31 Trees
y=6  [ 2 big: Cave 2   ]   32 LAIR 2   33 Raccoon  34 Forge    35 Heart  [ 1 big: Salamander  ]
y=7  [ 2               ]   36 Graves   37 START    38 Phoenix  39 Cave 8 [ 1                  ]
```

Regions:
- **Greenwood:** the start, village, lair 1 and the Fairy Ring.
- **Old Crypt:** the west side, with lair 2.
- **Mirefen:** the north and east, with lairs 3, 4 and 5.
- **Emberpeak:** the south-east, with lair 6.

### Relic gates (blocked paths on the overworld)

| Looks like | Needs | From | Where |
|---|---|---|---|
| Thorny vines | Vine Whip (Y, facing them) | Lair 1 | 23 → west to 22 |
| A chasm or pit (it's a hidden walkway) | Spirit Lantern (shows the path) | Lair 2 | **9 → east to 10** (the chasm in the Old Crypt) |
| Huge boulder | Titan Gloves (push into it) | Lair 3 | 16 → 12 and 16 → 17; 18 → 19; 21 → 7 and 21 → 25 |
| Lava river | Ember Boots (walk on lava) | Lair 4 | 12 → 11 and 12 → 13; 17 → 13; 19 → 20; 7 → 0 and 7 → 27 |
| Real chasm | Feather Cloak (Y, then walk over) | Lair 5 | 31 → south to 1 |

Falling into a gate pit drops you back where you entered the screen, with a riddle hint.

## Main route

1. **The start (37):** touch the monolith. Head north to **33** (the Bandit Raccoon's screen),
   then **29, the village**:
   - **Shop:** go in through the cottage door.
   - **Inn:** 10 gold for a full rest.
   - **Notice board and scholar:** the scholar runs the spellbook quest.
2. **Lair 1, the Overgrown Shrine (23):** from the village go north to **6** (the Fairy Ring
   wilderness), then west to **23**. Prize: the **Vine Whip**.
3. **Lair 2, the Underground Crypt (32):**
   1. Cut the thorns on the west side of 23, into **22** (the Treant's grove).
   2. Go south to **5** (Cave 1), east to **28** (the Headless Knight), then south to **32**.
   3. Prize: the **Spirit Lantern**.
4. **Lair 3, the Ruined Castle (16):**
   1. With the lantern, cross the hidden path from **9 east to 10**. To reach 9, take
      22 → 4 → 8 → 9.
   2. From 10 go east into **3** (Cave 4), then east to **15**, then east to **16**.
   3. Prize: the **Titan Gloves**.
5. **Lair 4, the Dragon Fortress (12):** push the boulder north of 16 into **12**. Prize: the
   **Ember Boots**.
6. **Lair 5, the Forgotten Sanctuary (26):**
   1. Use either route. From 21, push the boulder east into **7**, cross the lava south into
      **0**, then go west to **26**. Or from 18, push the boulder into 19, cross the lava into
      20, then go south through 24 to 26.
   2. Prize: the **Feather Cloak**.
7. **Lair 6, the Dark Tower (30), which needs all 5 runes:**
   1. From **7**, cross the lava south to **27**, then go to **31**.
   2. Float over the chasm south into **1** (the Salamander Queen), then go west through 39,
      38 and 34, and north into **30**.

## Lair walkthroughs

### Lair 1: Overgrown Shrine (9 rooms), Vine Whip

1. **Moss Gallery** (west of the Vine Atrium):
   - Hit the **crystal switch** to drop the bars, then open the chest for the **small key**.
   - The map is in the other chest.
2. **Vine Atrium:** the small key opens the east door, to the **Thorn Nest**.
3. **Thorn Nest:** beat every monster. The doors seal until you do. Then the **item finder**
   chest appears.
4. **Seed Vault** (north of the nest):
   - Take **fire** from the orb shrine and light all four **torches**.
   - The **big key** chest appears.
5. **Hall of the Great Seed** (north of the gallery, behind the big lock): the big chest holds the
   **Vine Whip**.
6. **Root Hall** (north of the atrium):
   1. Pick the whip with Select. Stand on the south edge of the chasm below the left post (it
      glows green), face north and press Y. The whip pulls you across.
   2. Pull the **lever** on the north side. That opens the north door to the **Guardian's
      Stair** and the boss.
   3. The post on the south bank brings you back.
7. **Fungus Cellar** (east of the entry): pots and crates, plus 3 bombs in a chest.

### Lair 2: Underground Crypt (11 rooms), Spirit Lantern

1. **Ossuary** (west of the entry): cut the thorns with the whip. **Small key 1.**
2. **Catacombs** (east, through the small lock): a sealed fight, then the **item finder**.
3. **Embalming Room** (north of the catacombs): push both **ice blocks onto the plates**.
   **Small key 2** (keep it for later).
4. **Great Nave** (north of the entry): hit the **crystal switch** to lower the bars to the west.
5. **Side Chapels** (west, dark):
   - Use the **whip posts** over the chasm, then step on the **floor switch** (the bell rope).
     That opens the Choir.
   - The map is here.
   - A bombable wall leads west to the **Charnel Pit**, where an elixir sits in a chest.
6. **Choir Loft** (north of the nave):
   - Hit the crystals in order: **fire, then ice, then storm**. The three shrines in the room
     switch your element.
   - The **big key** chest appears.
7. **Reliquary** (east of the nave, big lock): the big chest holds the **Spirit Lantern**.
8. **Bridge of Souls** (north of the choir, small-key door, dark): the bridge is invisible
   without the lantern. With it the hidden path shows. Then the **Bone Throne Stair** and the
   boss.

### Lair 3: Ruined Castle (12 rooms around a 2x2 courtyard), Titan Gloves

1. **Barracks** (west): a fight, then the **small key**.
2. **Armory** (east):
   - Push the blocks onto the plates. The **map** is here.
   - Go on to the **Winch Room**. Its **lever** lowers the drawbridge, which drains the
     courtyard moat.
3. **Keep Hall**, then the dark **Library**:
   - Hit the crystals in order: **earth, then storm, then fire**.
   - The **big key** chest appears.
4. **Treasury:** the big chest holds the **Titan Gloves**.
5. **Courtyard:** push into the boulders to move them.
6. **Tower Stair** (whip over the chasm), then the **Rampart Walk** (crystal switch), then the
   **High Tower** and the boss.

### Lair 4: Dragon Fortress (13 rooms), Ember Boots

1. **Smelter:** light the torches. **Small key 1.**
2. **Anvil Room:** push the blocks onto the plates. **Small key 2.**
3. **Steam Vents:** whip over the chasm to the **lever**. It cools the lava moat in the
   **Cinder Hall**.
4. **War Room** (dark, small-key door):
   - Hit the crystals in order: **fire, then earth, then ice, then storm**.
   - The **big key** chest appears.
5. **Dragon's Hoard:** the big chest holds the **Ember Boots**.
6. **Obsidian Walk** (lava), then the **Magma Chamber** (lava and boulders), then the **Wyrm
   Stair** and the boss.
7. **Optional:**
   - **Slag Pits:** a fight, then the item finder. The map is guarded by a boulder.
   - **Drake Kennel:** a fight, then elixirs.

### Lair 5: Forgotten Sanctuary (14 rooms), Feather Cloak

1. **West Cloister:** push the ice blocks onto the plates. **Small key 1.**
2. **Baptistry:** whip to the island **lever**. It drains the **Sacred Font**, which holds the
   **item finder**.
3. **Hall of Saints:** a fight, then the **map**.
4. **Star Court:** the crystal switch chooses whether the east or west way is open.
5. **Bell Tower** (dark, hidden bridge, so use the lantern): **small key 2**.
6. **Scriptorium** (small-key door):
   - Hit the crystals in order: **ice, then storm, then fire, then earth**.
   - The **big key** chest appears.
7. **Inner Sanctum:** the big chest holds the **Feather Cloak**.
8. **Sky Chasm** (small-key door): pick the cloak, press Y and float over the pits. Then the
   **Gallery of Wings** (a fight), then the **Angel's Stair** and the boss.
9. **Optional:** the **Relic Vault**, behind a cracked wall (bomb it).

### Lair 6: The Dark Tower (16 rooms), needs every relic

1. **Thorn Garden:** use the whip. **Small key 1.**
2. **Lava Well** (small-key door): walk on the lava with the boots. **Small key 2.**
3. **Hall of Chains:** lift the boulders with the gloves.
4. **Storm Engine:** the crystal switch frees the lever, which unseals the **Mirror Hall**.
5. **Mirror Hall:** hit the crystals in order: **storm, then ice, then earth, then fire**. The
   **big key** chest appears.
6. **Abyssal Stair** (small-key door): cross the pits with the whip posts or the cloak.
7. **Throne Antechamber** (big-key door): a fight. Then the **Dark Throne Stair** and the final
   boss.
8. **Optional:**
   - **Dark Archive:** use the lantern on the hidden bridge to reach the **map**.
   - **Void Ledge:** use the cloak or the whip to reach the **item finder**.
   - **Cursed Library, Ice Gallery, Root Cellar and Shadow Armory.**

## Bosses

| Lair | Boss | Weak to | Resists | Notes |
|---|---|---|---|---|
| 1 | Ancient Treant | fire | earth | Roots under you, a branch sweep across the floor, a shockwave ring, slimes. **Vine Lash:** a red band flashes on both sides at its height, then vines whip across, so don't hide beside it. Half health: phase 2. |
| 2 | Undead Guardian | storm | ice | Sword slashes at you, guards (blocks), dark orbs, raises the dead. |
| 3 | Stone Golem | storm | fire | Armour first (break it), then rock throws, ground slam with falling rocks, shockwave. |
| 4 | Crimson Dragon | ice | fire | Fireballs, flame breath, flies up and lands on you (tracked shadow), tail swipe up close. |
| 5 | Arcane Sorcerer | earth | storm | Bolts, teleports, magic circles under you, shield, summons in phase 2. |
| 6 | Dark Sorcerer | ice | fire | Same tricks as lair 5, with three phases. |

## Caves (optional, 8)

Each cave has either a sealed fight or a freeze-plate puzzle, then a treasure chest. Cleared caves
get a glowing rune over the mouth. The **spellbook pages** are in caves 2, 3, 4, 6 and 8. Bring
all 5 to the scholar in the village to learn **Arcane Blink** (R1).

| Cave | Area | Name | Challenge | Reward |
|---|---|---|---|---|
| 1 | 5 | Mossy Hollow | fight | 5 bombs |
| 2 | 2 | Whispering Grotto | freeze 1 plate | elixir + page |
| 3 | 9 | Bone Cave | freeze 1 plate | heart container + page |
| 4 | 3 | Rooted Den | fight | mana crystal + page |
| 5 | 17 | Frosted Cavern | freeze 2 plates | 5 bombs |
| 6 | 20 | Sunken Burrow | fight | heart container + page |
| 7 | 27 | Echoing Deep | freeze 2 plates | mana crystal |
| 8 | 39 | Ember Vault | fight | 2 elixirs + page |

(Cave 2 has its own level file, `levels/cave_2.json`, and it can change the layout.)

## Mini-bosses (one-time encounters)

| Encounter | Area | How to beat it | Reward |
|---|---|---|---|
| Hoard Dragon | 11 | Can't be hurt; take the gold pile while it sleeps or rages | The gold |
| Deceiving Dryad | 21 (pool in 15) | Follow the "villager"; step in the poison pool or hit her to reveal her; weak to fire | Heart container |
| Angry Treant | 22 | Shake the fruit trees; one is the treant | Golden apple |
| Grave Lord | 36 | Storm bolts raise zombies from graves; after 6 he rises; weak to fire | +20 max magic |
| Mimic Chest | 8 | A chest that breathes; wakes when you get close; weak to fire | Gold, 3 bombs |
| Treasure Goblin | random (5% on plain screens) | Hit it for coins; kill it before it escapes (~15 s) | Its sack |
| Bandit Raccoon | 33 | Steals gold and food, then flees to the next screen; follow it and finish it there | Your loot + stash |
| Merchant Ogre | random (6%) | Fed: A buys bombs and elixirs. Starving or hit: he fights | Elixir, 5 bombs |
| Fairy King | 6 | Step into the mushroom ring; beat the king within 45 s | Speed charm |
| Honey Bear | 24 | Shoot the hive above the centre; the bees sting the bear | Honey, heart container |
| Headless Knight | 28 | The armour can't be hurt; hit the rolling head | Mana crystal |
| Banshee | 10 | Invisible until you stand still or hit her with storm | Gold, elixir |
| Bog Witch | 0 | Her 5-gold stew curses you (reversed controls, poison); then beat her | Shop card: 25% off |
| Giant Toad | 14 | Don't stand in front; if swallowed, tap A to hit its belly | Gold, elixir |
| Will-o'-wisp | 7 | Quicksand drags you back; ice bolts freeze it solid | 2 elixirs |
| Salamander Queen | 1 | Ice first (she turns to stone), then fire or earth for double damage | Heart container |
| Lava Golem Forge | 34 | Freeze all four lava pools with ice, or it reforges | Gold, mana crystal |
| Phoenix | 38 | When it dies it becomes an egg; smash the egg within 8 s | Heart container, elixir |
| Doppelganger | 18 | Immune to your element; switch at a shrine on the screen and hit it before it copies you | Mirror charm (+20% bolts) |

## Known rough edges to watch for

- **Mini-bosses** haven't been played by a person yet, so balance and feel are untested.
- **Handheld:** check frame rate in busy fights, button labels, and that the picture fills the
  screen.
- **Log file:** if the handheld port won't start, check `roms/ports/elementallegends/log.txt`.
- **Reporting bugs:** note the area number (the world map) or the lair room name (shown on
  entry), and what you were doing.
