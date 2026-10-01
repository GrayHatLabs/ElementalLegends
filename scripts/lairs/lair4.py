"""Lair 4: DRAGON FORTRESS - the Ember Boots. 13 rooms.

Smelter (torches) -> key 1 -> Anvil Room (blocks onto plates) -> key 2. Steam Vents:
whip over the chasm to the lever that cools the Cinder Hall's lava moat. War Room (dark,
crystals fire-earth-ice-storm, key 2 door) -> big key -> Dragon's Hoard (big chest:
boots) -> Obsidian Walk (lava) -> Magma Chamber (lava, boulders) -> Wyrm Stair.
Optional: Slag Pits (fight: finder, boulder-guarded map), Drake Kennel (fight: elixirs).
"""
from lairkit import Room, lair

P = 'prop_7_'


def build():
    gate = Room('gate', 'Magma Gate', (2, 6))
    gate.door('s', 'exit').door('n', 'hall').door('w', 'smelter').door('e', 'vents')
    gate.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    gate.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    gate.prop(P + 'mosaic', 8, 7, solid=False)
    gate.obj('tablet', 5, 2, text='The wyrm sleeps where the lava runs deepest.')
    gate.mob('imp', 5, 8).mob('imp', 10, 8)

    smelter = Room('smelter', 'Smelter', (1, 6))
    smelter.door('e', 'gate')
    smelter.shutter = 'torches'
    for x, y in [(3, 3), (12, 3), (3, 9), (12, 9)]:
        smelter.obj('torch', x, y)
    smelter.obj('shrine', 8, 10, element='fire')
    smelter.rect(6, 5, 9, 7, 'l')
    smelter.obj('chest', 8, 3, contents='small_key', hidden=True)
    smelter.obj('tablet', 5, 6, text='Rekindle the smelter fires.')
    smelter.random_enemies = 1

    vents = Room('vents', 'Steam Vents', (3, 6))
    vents.door('w', 'gate')
    vents.rect(6, 1, 8, 11, 'p')
    vents.obj('whip_post', 10, 6, _ok=1).obj('whip_post', 4, 4)
    vents.obj('lever', 13, 6, sets='cool', _ok=1)
    vents.obj('tablet', 3, 9, text='The vent lever cools the lava in the Cinder Hall.')
    vents.prop(P + 'hoard', 12, 2).prop(P + 'hoard', 12, 10)
    vents.mob('bat', 3, 3).mob('bat', 12, 9)

    hall = Room('hall', 'Forge Hall', (2, 5))
    hall.door('s', 'gate').door('n', 'cinder').door('w', 'slag').door('e', 'anvil', 'small')
    hall.pillars([(4, 3), (11, 3), (4, 9), (11, 9)])
    hall.frame(11, 5, 13, 7, 'r').set(11, 6, 'r')
    hall.obj('chest', 12, 6, contents='gold:60', _ok=1)
    hall.obj('crystal_switch', 6, 6)
    hall.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    hall.mob('imp', 8, 8).mob('golem', 6, 9)

    slag = Room('slag', 'Slag Pits', (1, 5))
    slag.door('e', 'hall')
    slag.shutter = 'combat'
    slag.rect(4, 4, 5, 8, 'p').rect(10, 4, 11, 8, 'p')
    slag.rect(1, 2, 3, 2, '#').rect(3, 1, 3, 1, '#')
    slag.obj('boulder', 3, 3, _ok=1)
    slag.obj('chest', 1, 1, contents='map', _ok=1)
    slag.obj('chest', 8, 6, contents='finder', hidden=True)
    slag.mob('imp', 7, 3).mob('imp', 8, 9).mob('golem', 13, 6).mob('bat', 7, 6)

    anvil = Room('anvil', 'Anvil Room', (3, 5))
    anvil.door('w', 'hall').door('e', 'kennel')
    anvil.shutter = 'plates'
    anvil.set(4, 3, 'o').set(11, 9, 'o')
    anvil.obj('block', 6, 4).obj('block', 9, 8)
    anvil.obj('chest', 8, 2, contents='small_key', hidden=True)
    anvil.prop(P + 'brazier', 2, 2).prop(P + 'brazier', 13, 2)
    anvil.random_enemies = 1

    kennel = Room('kennel', 'Drake Kennel', (4, 5))
    kennel.door('w', 'anvil')
    kennel.shutter = 'combat'
    kennel.rect(4, 3, 4, 9, 'D').rect(11, 3, 11, 9, 'D')
    kennel.mob('imp', 6, 3).mob('imp', 9, 9).mob('golem', 8, 6).mob('bat', 13, 3).mob('bat', 2, 9)
    kennel.obj('chest', 8, 3, contents='elixirs:2', hidden=True)

    cinder = Room('cinder', 'Cinder Hall', (2, 4))
    cinder.door('s', 'hall').door('n', 'hoard', 'big').door('w', 'warroom', 'small')
    cinder.rect(1, 8, 14, 10, 'l')
    cinder.when('cool', 'l', '.')
    cinder.prop(P + 'statue', 4, 3).prop(P + 'statue', 11, 3)
    cinder.obj('tablet', 8, 11, text='A moat of fire. Somewhere a vent could cool it.')
    cinder.mob('imp', 6, 5).mob('imp', 10, 5)

    war = Room('warroom', 'War Room', (1, 4), dark=True)
    war.door('e', 'cinder')
    war.shutter = 'crystals'
    war.rect(5, 5, 10, 7, 'D')
    war.obj('crystal', 3, 3, element='fire', order=1).obj('crystal', 12, 3, element='earth', order=2)
    war.obj('crystal', 12, 9, element='ice', order=3).obj('crystal', 3, 9, element='storm', order=4)
    for x, el in [(2, 'fire'), (6, 'earth'), (9, 'ice'), (13, 'storm')]:
        war.obj('shrine', x, 11, element=el)
    war.obj('chest', 8, 2, contents='big_key', hidden=True)
    war.obj('tablet', 8, 9, text='Fire, earth, ice, storm: the order of the war council.')
    war.mob('ghost', 4, 6)

    hoard = Room('hoard', "Dragon's Hoard", (2, 3))
    hoard.door('s', 'cinder').door('e', 'obsidian')
    hoard.obj('big_chest', 8, 4)
    for x, y in [(3, 3), (12, 3), (3, 9), (12, 9)]:
        hoard.prop(P + 'hoard', x, y)
    hoard.prop(P + 'mosaic', 8, 8, solid=False)
    hoard.random_enemies = 0

    obs = Room('obsidian', 'Obsidian Walk', (3, 3))
    obs.door('w', 'hoard').door('n', 'magma')
    obs.rect(3, 1, 14, 11, 'l')
    for x, y in [(6, 3), (9, 8), (12, 5)]:
        obs.rect(x, y, x + 1, y + 1, '.')
    obs.obj('tablet', 1, 9, text='Only boots that defy fire may walk the obsidian.')
    obs.mob('imp', 6, 3).mob('imp', 12, 5)

    magma = Room('magma', 'Magma Chamber', (3, 2))
    magma.door('s', 'obsidian').door('w', 'wyrm')
    magma.rect(1, 8, 14, 10, 'l')
    magma.rect(1, 4, 3, 4, '#').rect(1, 8, 3, 8, '#')
    magma.obj('boulder', 2, 5).obj('boulder', 2, 6).obj('boulder', 2, 7)
    magma.prop(P + 'statue', 8, 2).prop(P + 'brazier', 12, 2).prop(P + 'brazier', 5, 2)
    magma.mob('golem', 9, 5).mob('imp', 12, 6)

    wyrm = Room('wyrm', 'Wyrm Stair', (2, 2))
    wyrm.door('e', 'magma')
    wyrm.boss_stairs = True
    wyrm.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    wyrm.prop(P + 'statue', 3, 7).prop(P + 'statue', 12, 7)
    wyrm.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    wyrm.random_enemies = 0

    rooms = [gate, smelter, vents, hall, slag, anvil, kennel, cinder, war, hoard, obs, magma, wyrm]
    return lair(4, 'Dragon Fortress', 7, 'ember_boots', rooms), rooms
