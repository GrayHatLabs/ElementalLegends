"""Lair 3: RUINED CASTLE - the Titan Gloves. 12 rooms around a 2x2 courtyard.

Barracks (fight) -> key -> Armory (blocks onto plates) -> map -> Winch Room (lever) ->
drawbridge drains the moat -> Keep Hall -> dark Library (crystals: earth, storm, fire)
-> big key -> Treasury (big chest: gloves) -> lift the courtyard boulders -> Tower Stair
(whip over the chasm) -> Rampart Walk (crystal switch) -> High Tower.
"""
from lairkit import Room, lair

P = 'prop_6_'


def build():
    gate = Room('gate', 'Gatehouse', (2, 6))
    gate.door('s', 'exit').door('n', 'court')
    gate.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    gate.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    gate.prop(P + 'banner', 5, 1).prop(P + 'banner', 10, 1)
    gate.obj('tablet', 5, 9, text='The castle fell, but its guardian never left the high tower.')
    gate.mob('skeleton', 5, 7).mob('skeleton', 10, 7)

    court = Room('court', 'Ruined Courtyard', (1, 4), size=(2, 2))
    court.door('s', 'gate', seg=1).door('w', 'barracks', seg=0).door('w', 'cells', seg=1)
    court.door('e', 'armory', seg=1).door('n', 'keephall', seg=0).door('n', 'towerstair', seg=1)
    court.rect(1, 3, 30, 5, '~')
    court.when('drawbridge', '~', '.')
    # Boulders guard the tower door (Titan Gloves).
    court.rect(21, 1, 21, 2, '#').rect(26, 1, 26, 2, '#').set(22, 2, '#').set(25, 2, '#')
    court.obj('boulder', 23, 2, _ok=1).obj('boulder', 24, 2, _ok=1)
    # Broken walls and rubble across the yard.
    court.rect(5, 10, 9, 10, '#').rect(22, 10, 26, 10, '#').rect(12, 15, 19, 15, '#')
    court.rect(4, 20, 4, 23, '#').rect(27, 13, 27, 16, '#')
    for x, y in [(3, 8), (14, 9), (29, 9), (8, 17), (18, 21), (25, 20)]:
        court.prop(P + 'rubble', x, y)
    court.prop(P + 'statue', 15, 12).prop(P + 'statue', 16, 12)
    court.prop(P + 'pillar', 10, 19).prop(P + 'pillar', 21, 19)
    court.obj('tablet', 13, 22, text='The drawbridge winch lies past the armory.')
    court.mob('skeleton', 8, 13).mob('skeleton', 24, 13).mob('imp', 16, 18).mob('imp', 6, 22).mob('golem', 25, 23)

    barracks = Room('barracks', 'Barracks', (0, 4))
    barracks.door('e', 'court')
    barracks.shutter = 'combat'
    for x in (3, 6, 9, 12):
        barracks.rect(x, 2, x, 3, 'D').rect(x, 9, x, 10, 'D')
    barracks.mob('skeleton', 4, 6).mob('skeleton', 11, 6).mob('imp', 8, 4).mob('imp', 8, 9).mob('golem', 13, 6)
    barracks.obj('chest', 8, 6, contents='small_key', hidden=True)

    cells = Room('cells', 'Prison Cells', (0, 5))
    cells.door('e', 'court')
    for x in (4, 8):
        cells.rect(x, 1, x, 4, '#')
    cells.rect(1, 5, 12, 5, '#').rect(2, 5, 3, 5, 't').rect(6, 5, 7, 5, '.').rect(10, 5, 11, 5, 'c')
    cells.obj('chest', 2, 2, contents='finder', _ok=1)
    cells.obj('chest', 10, 2, contents='bombs:4', _ok=1)
    cells.prop(P + 'rubble', 6, 2)
    cells.obj('pot', 3, 10).obj('crate', 12, 10).obj('crate', 13, 9).obj('pot', 7, 10)
    cells.obj('tablet', 9, 8, text='The first cell is choked with thorns; the last wall is brittle.')
    cells.mob('zombie', 5, 8).mob('bat', 11, 8)

    armory = Room('armory', 'Armory', (3, 5))
    armory.door('w', 'court').door('n', 'winch', 'small')
    armory.shutter = 'plates'
    armory.set(4, 3, 'o').set(11, 3, 'o')
    armory.obj('block', 5, 7).obj('block', 10, 7)
    armory.obj('chest', 8, 5, contents='map', hidden=True)
    armory.prop(P + 'rubble', 2, 10).prop(P + 'rubble', 13, 10)
    armory.prop(P + 'banner', 2, 1).prop(P + 'banner', 13, 1)
    armory.random_enemies = 1

    winch = Room('winch', 'Winch Room', (3, 4))
    winch.door('s', 'armory')
    winch.obj('lever', 8, 3, sets='drawbridge')
    winch.obj('tablet', 5, 6, text='Pull the lever to lower the drawbridge over the moat.')
    winch.prop(P + 'pillar', 4, 3).prop(P + 'pillar', 11, 3)
    winch.obj('crate', 2, 9).obj('crate', 13, 9).obj('pot', 12, 2)
    winch.mob('imp', 10, 8)

    keep = Room('keephall', 'Keep Hall', (1, 3))
    keep.door('s', 'court').door('w', 'library').door('n', 'treasury', 'big')
    keep.pillars([(4, 3), (11, 3), (4, 9), (11, 9)])
    keep.prop(P + 'rug', 8, 7, solid=False)
    keep.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    keep.obj('tablet', 13, 10, text="The library's crystals answer earth, then storm, then fire.")
    keep.mob('skeleton', 6, 6).mob('golem', 10, 6)

    lib = Room('library', 'Library', (0, 3), dark=True)
    lib.door('e', 'keephall')
    lib.shutter = 'crystals'
    for y in (3, 9):
        lib.rect(2, y, 5, y, 'D').rect(10, y, 13, y, 'D')
    lib.obj('crystal', 4, 6, element='earth', order=1).obj('crystal', 8, 2, element='storm', order=2)
    lib.obj('crystal', 12, 6, element='fire', order=3)
    lib.obj('shrine', 2, 11, element='earth').obj('shrine', 8, 11, element='storm').obj('shrine', 13, 11, element='fire')
    lib.obj('chest', 8, 6, contents='big_key', hidden=True)
    lib.mob('ghost', 6, 8)

    treasury = Room('treasury', 'Treasury', (1, 2))
    treasury.door('s', 'keephall')
    treasury.obj('big_chest', 8, 4)
    treasury.prop(P + 'statue', 5, 4).prop(P + 'statue', 11, 4)
    treasury.prop(P + 'rubble', 3, 9).prop(P + 'rubble', 12, 9)
    treasury.prop(P + 'rug', 8, 8, solid=False)
    treasury.random_enemies = 0

    tower = Room('towerstair', 'Tower Stair', (2, 3))
    tower.door('s', 'court').door('n', 'rampart')
    tower.rect(1, 5, 14, 7, 'p')
    tower.obj('whip_post', 8, 3, _ok=1).obj('whip_post', 11, 9)
    tower.prop(P + 'pillar', 3, 2).prop(P + 'pillar', 12, 2)
    tower.obj('tablet', 5, 10, text='Lash the vine to the post to cross.')
    tower.mob('bat', 6, 9).mob('bat', 10, 2)

    rampart = Room('rampart', 'Rampart Walk', (2, 2))
    rampart.door('s', 'towerstair').door('n', 'hightower')
    rampart.rect(5, 2, 10, 2, 'r')
    rampart.rect(3, 4, 3, 8, 'u')
    rampart.obj('crystal_switch', 8, 8)
    rampart.pillars([(5, 5), (10, 5)])
    rampart.mob('skeleton', 4, 9).mob('skeleton', 12, 4).mob('imp', 12, 9)

    high = Room('hightower', 'High Tower', (2, 1))
    high.door('s', 'rampart')
    high.boss_stairs = True
    high.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    high.prop(P + 'statue', 3, 7).prop(P + 'statue', 12, 7)
    high.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    high.random_enemies = 0

    rooms = [gate, court, barracks, cells, armory, winch, keep, lib, treasury, tower, rampart, high]
    return lair(3, 'Ruined Castle', 6, 'titan_gloves', rooms), rooms
