"""Lair 6: THE DARK TOWER - every relic. 16 rooms.

Thorn Garden (whip) -> key 1 -> Lava Well (key 1 door, boots) -> key 2. Hall of Chains:
lift the boulders (gloves) -> Storm Engine (crystal switch frees the lever) -> unseals the
Mirror Hall -> crystals storm-ice-earth-fire -> big key. Abyssal Stair (key 2 door;
whip posts or the cloak over the pits) -> Throne Antechamber (big key door, fight) ->
Dark Throne Stair. Dark Archive (lantern, hidden bridge) -> map; Void Ledge (cloak or
whip) -> finder; optional Cursed Library, Ice Gallery, Root Cellar and Shadow Armory.
"""
from lairkit import Room, lair

P = 'prop_9_'


def build():
    gate = Room('gate', 'Tower Gate', (2, 6))
    gate.door('s', 'exit').door('n', 'foyer')
    gate.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    gate.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    gate.prop(P + 'circle', 8, 7, solid=False)
    gate.obj('tablet', 5, 2, text='All five relics, all four elements. Only then the throne.')
    gate.mob('skeleton', 5, 8).mob('ghost', 10, 8)

    foyer = Room('foyer', 'Shadow Foyer', (2, 5))
    foyer.door('s', 'gate').door('w', 'thorns').door('e', 'archive').door('n', 'chains', seg=0)
    foyer.pillars([(4, 3), (11, 3), (4, 9), (11, 9)])
    foyer.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    foyer.prop(P + 'cage', 2, 10).prop(P + 'cage', 13, 10)
    foyer.mob('imp', 7, 6).mob('ghost', 9, 8)

    thorns = Room('thorns', 'Thorn Garden', (1, 5))
    thorns.door('e', 'foyer').door('w', 'cellar', 'bomb')
    thorns.rect(3, 3, 12, 3, 't').rect(3, 9, 12, 9, 't').rect(3, 4, 3, 8, 't').rect(12, 4, 12, 8, 't')
    thorns.frame(10, 1, 14, 3, 't').set(14, 1, '#').set(14, 2, '#')
    thorns.obj('chest', 12, 2, contents='small_key', _ok=1)
    thorns.obj('chest', 7, 6, contents='gold:80', _ok=1)
    thorns.obj('tablet', 14, 10, text='Thorns everywhere. Cut, then cut again.')
    thorns.mob('slime', 2, 6).mob('slime', 13, 6)

    cellar = Room('cellar', 'Root Cellar', (0, 5))
    cellar.door('e', 'thorns')
    for x, y in [(2, 2), (3, 2), (12, 2), (13, 10), (2, 10)]:
        cellar.obj('pot', x, y)
    for x, y in [(13, 2), (12, 10), (3, 10)]:
        cellar.obj('crate', x, y)
    cellar.obj('chest', 8, 6, contents='heart')
    cellar.random_enemies = 1

    archive = Room('archive', 'Dark Archive', (3, 5), dark=True)
    archive.door('w', 'foyer').door('e', 'library')
    archive.rect(2, 2, 13, 4, 'p')
    for x, y in [(4, 4), (4, 3), (5, 3), (6, 3), (6, 2)]:
        archive.set(x, y, 'h')
    archive.rect(5, 1, 7, 1, '.')
    archive.obj('chest', 6, 1, contents='map', _ok=1)
    archive.prop(P + 'banner', 12, 10).prop(P + 'cage', 3, 9)
    archive.obj('tablet', 10, 9, text='The map lies past a bridge only the lantern shows.')
    archive.mob('ghost', 9, 7)

    library = Room('library', 'Cursed Library', (4, 5))
    library.door('w', 'archive')
    library.shutter = 'combat'
    for y in (3, 9):
        library.rect(2, y, 5, y, 'D').rect(10, y, 13, y, 'D')
    library.mob('ghost', 4, 6).mob('ghost', 11, 6).mob('imp', 8, 2).mob('skeleton', 8, 10).mob('golem', 13, 6)
    library.obj('chest', 8, 6, contents='elixirs:2', hidden=True)

    chains = Room('chains', 'Hall of Chains', (2, 4), size=(2, 1))
    chains.door('s', 'foyer', seg=0).door('w', 'lava', 'small').door('e', 'void')
    chains.door('n', 'mirror', 'flag:seal', seg=0).door('n', 'storm', seg=1)
    chains.rect(21, 1, 21, 2, '#').rect(26, 1, 26, 2, '#').set(22, 2, '#').set(25, 2, '#')
    chains.obj('boulder', 23, 2, _ok=1).obj('boulder', 24, 2, _ok=1)
    for x, y in [(4, 3), (12, 9), (15, 3), (19, 9), (28, 3)]:
        chains.prop(P + 'cage', x, y)
    chains.pillars([(10, 4), (10, 8), (17, 4), (17, 8)])
    chains.mob('golem', 8, 6).mob('golem', 24, 7).mob('skeleton', 14, 5).mob('skeleton', 20, 8)

    lava = Room('lava', 'Lava Well', (1, 4))
    lava.door('e', 'chains')
    lava.rect(2, 2, 13, 10, 'l')
    lava.rect(7, 5, 9, 7, '.')
    lava.obj('chest', 8, 6, contents='small_key', _ok=1)
    lava.obj('tablet', 14, 9, text='Walk where fire flows.')
    lava.random_enemies = 1

    void = Room('void', 'Void Ledge', (4, 4))
    void.door('w', 'chains')
    void.rect(4, 1, 7, 11, 'p')
    void.obj('whip_post', 9, 5, _ok=1).obj('whip_post', 2, 8)
    void.obj('chest', 12, 6, contents='finder', _ok=1)
    void.prop(P + 'statue', 13, 2, solid=True)
    void.random_enemies = 0

    storm = Room('storm', 'Storm Engine', (3, 3))
    storm.door('s', 'chains')
    storm.frame(6, 1, 10, 3, 'r')
    storm.obj('lever', 8, 2, sets='seal', _ok=1)
    storm.obj('crystal_switch', 8, 9)
    storm.obj('tablet', 3, 9, text='The engine seals the Mirror Hall until its lever is thrown.')
    storm.pillars([(3, 5), (12, 5)])
    storm.mob('ghost', 5, 7).mob('imp', 11, 7)

    mirror = Room('mirror', 'Mirror Hall', (2, 3))
    mirror.door('s', 'chains').door('w', 'ice').door('n', 'seals', 'small')
    mirror.shutter = 'crystals'
    mirror.obj('crystal', 4, 3, element='storm', order=1).obj('crystal', 11, 3, element='ice', order=2)
    mirror.obj('crystal', 11, 9, element='earth', order=3).obj('crystal', 4, 9, element='fire', order=4)
    for x, el in [(6, 'storm'), (7, 'ice'), (8, 'earth'), (9, 'fire')]:
        mirror.obj('shrine', x, 11, element=el)
    mirror.obj('chest', 8, 6, contents='big_key', hidden=True)
    mirror.obj('tablet', 2, 6, text='Storm, ice, earth, fire: the mirror shows the order reversed.')

    ice = Room('ice', 'Ice Gallery', (1, 3))
    ice.door('e', 'mirror')
    ice.shutter = 'plates'
    ice.set(3, 2, 'o').set(12, 2, 'o')
    ice.obj('ice_block', 3, 9).obj('ice_block', 12, 9)
    ice.obj('chest', 8, 5, contents='bombs:5', hidden=True)
    ice.pillars([(7, 7), (8, 7)])
    ice.random_enemies = 1

    seals = Room('seals', 'Abyssal Stair', (2, 2))
    seals.door('s', 'mirror').door('n', 'throne', 'big')
    seals.rect(1, 3, 14, 9, 'p')
    seals.rect(6, 5, 9, 7, '.')
    seals.obj('whip_post', 8, 5, _ok=1).obj('whip_post', 8, 1, _ok=1)
    seals.obj('whip_post', 6, 7, _ok=1).obj('whip_post', 9, 11)
    seals.obj('tablet', 3, 11, text='Swing, float, or fall.')
    seals.mob('bat', 4, 1).mob('bat', 11, 11)

    throne = Room('throne', 'Throne Antechamber', (2, 1))
    throne.door('s', 'seals').door('w', 'armory').door('n', 'stairs', 'shutter')
    throne.shutter = 'combat'
    throne.pillars([(4, 4), (11, 4), (4, 8), (11, 8)])
    throne.prop(P + 'circle', 8, 6, solid=False)
    throne.mob('skeleton', 5, 6).mob('skeleton', 10, 6).mob('golem', 8, 3).mob('ghost', 3, 10).mob('ghost', 12, 10)

    armory = Room('armory', 'Shadow Armory', (1, 1))
    armory.door('e', 'throne')
    armory.obj('chest', 6, 5, contents='bombs:5').obj('chest', 10, 5, contents='elixir')
    armory.prop(P + 'cage', 3, 3).prop(P + 'banner', 12, 2)
    armory.random_enemies = 1

    stairs = Room('stairs', 'Dark Throne Stair', (2, 0))
    stairs.door('s', 'throne')
    stairs.boss_stairs = True
    stairs.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    stairs.prop(P + 'statue', 3, 7).prop(P + 'statue', 12, 7)
    stairs.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    stairs.random_enemies = 0

    rooms = [gate, foyer, thorns, cellar, archive, library, chains, lava, void, storm, mirror, ice, seals, throne, armory, stairs]
    data = lair(6, 'The Dark Tower', 9, None, rooms)
    del data['relic']
    return data, rooms
