"""Lair 5: FORGOTTEN SANCTUARY - the Feather Cloak. 14 rooms.

West Cloister (ice blocks onto plates) -> key 1. Baptistry: whip to the island lever,
which drains the Sacred Font -> finder; Hall of Saints (fight) -> map. Star Court
crystal switch picks east or west. Bell Tower (dark, hidden bridge) -> key 2.
Scriptorium (key 1 door; crystals ice-storm-fire-earth) -> big key -> Inner Sanctum
(big chest: cloak). Sky Chasm (key 2 door; float over the pits) -> Gallery of Wings
(fight) -> Angel's Stair. Optional: Relic Vault behind a cracked wall.
"""
from lairkit import Room, lair

P = 'prop_8_'


def build():
    entry = Room('entry', 'Sanctuary Steps', (2, 6))
    entry.door('s', 'exit').door('n', 'nave')
    entry.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    entry.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    entry.prop(P + 'mosaic', 8, 7, solid=False)
    entry.obj('tablet', 5, 2, text='Those who would rise must first learn to fall.')
    entry.mob('ghost', 6, 8).mob('ghost', 10, 8)

    nave = Room('nave', 'Moonlit Nave', (2, 5))
    nave.door('s', 'entry').door('w', 'cloister').door('e', 'baptistry').door('n', 'starcourt')
    nave.pillars([(4, 3), (11, 3), (4, 9), (11, 9)])
    nave.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    nave.prop(P + 'books', 2, 10).prop(P + 'books', 13, 10)
    nave.mob('ghost', 8, 6).mob('bat', 5, 6)

    cloister = Room('cloister', 'West Cloister', (1, 5))
    cloister.door('e', 'nave')
    cloister.shutter = 'plates'
    cloister.set(3, 2, 'o').set(12, 10, 'o')
    cloister.obj('ice_block', 3, 8).obj('ice_block', 6, 10)
    cloister.pillars([(7, 4), (7, 7)])
    cloister.obj('chest', 10, 3, contents='small_key', hidden=True)
    cloister.obj('tablet', 12, 6, text='Ice slides until something stops it.')
    cloister.random_enemies = 1

    bap = Room('baptistry', 'Baptistry', (3, 5))
    bap.door('w', 'nave')
    bap.rect(4, 2, 12, 10, '~')
    bap.rect(7, 5, 9, 7, '.')
    bap.obj('whip_post', 9, 6, _ok=1).obj('whip_post', 1, 8)
    bap.obj('lever', 7, 5, sets='drain', _ok=1)
    bap.obj('tablet', 2, 3, text='The island lever opens the font sluice.')
    bap.prop(P + 'statue', 14, 2).prop(P + 'statue', 14, 10)
    bap.mob('bat', 13, 6)

    star = Room('starcourt', 'Star Court', (2, 4))
    star.door('s', 'nave').door('n', 'scriptorium', 'small').door('w', 'font').door('e', 'belltower')
    star.rect(2, 4, 2, 8, 'u')
    star.rect(13, 4, 13, 8, 'r')
    star.obj('crystal_switch', 8, 6)
    star.prop(P + 'mosaic', 8, 9, solid=False)
    star.pillars([(5, 3), (10, 3)])
    star.mob('ghost', 6, 9).mob('ghost', 10, 9)

    font = Room('font', 'Sacred Font', (1, 4))
    font.door('e', 'starcourt').door('w', 'saints')
    font.rect(1, 2, 12, 10, '~')
    font.when('drain', '~', '.')
    font.set(6, 6, '.')
    font.obj('chest', 6, 6, contents='finder', _ok=1)
    font.prop(P + 'statue', 14, 2).prop(P + 'statue', 14, 10)
    font.obj('tablet', 13, 6, text='When the font is drained, its gift lies bare.')
    font.random_enemies = 0

    saints = Room('saints', 'Hall of Saints', (0, 4))
    saints.door('e', 'font')
    saints.shutter = 'combat'
    for x in (3, 6, 9, 12):
        saints.prop(P + 'statue', x, 2)
    saints.mob('ghost', 4, 6).mob('ghost', 11, 6).mob('skeleton', 8, 9).mob('bat', 6, 4).mob('bat', 10, 4)
    saints.obj('chest', 8, 6, contents='map', hidden=True)

    bell = Room('belltower', 'Bell Tower', (3, 4), dark=True)
    bell.door('w', 'starcourt')
    bell.rect(3, 1, 14, 11, 'p')
    for x, y in [(3, 6), (4, 6), (5, 6), (5, 5), (5, 4), (6, 4), (7, 4), (8, 4), (8, 3), (8, 2), (9, 2), (10, 2), (11, 2)]:
        bell.set(x, y, 'h')
    bell.set(12, 2, '.').set(13, 2, '.')
    bell.obj('chest', 13, 2, contents='small_key', _ok=1)
    bell.obj('tablet', 1, 9, text='The bell ropes hang over nothing. Trust only what the light reveals.')

    scr = Room('scriptorium', 'Scriptorium', (2, 3))
    scr.door('s', 'starcourt').door('n', 'chasm', 'small').door('w', 'sanctum', 'big').door('e', 'vault', 'bomb')
    scr.shutter = 'crystals'
    scr.obj('crystal', 4, 3, element='ice', order=1).obj('crystal', 11, 3, element='storm', order=2)
    scr.obj('crystal', 11, 9, element='fire', order=3).obj('crystal', 4, 9, element='earth', order=4)
    for x, el in [(6, 'ice'), (7, 'storm'), (8, 'fire'), (9, 'earth')]:
        scr.obj('shrine', x, 11, element=el)
    scr.obj('chest', 8, 6, contents='big_key', hidden=True)
    scr.prop(P + 'books', 2, 2).prop(P + 'books', 13, 2)
    scr.obj('tablet', 2, 6, text='Ice, storm, fire, earth: the order of the scribes.')

    sanctum = Room('sanctum', 'Inner Sanctum', (1, 3))
    sanctum.door('e', 'scriptorium')
    sanctum.obj('big_chest', 8, 4)
    sanctum.prop(P + 'statue', 5, 4).prop(P + 'statue', 11, 4)
    sanctum.prop(P + 'mosaic', 8, 8, solid=False)
    sanctum.random_enemies = 0

    vault = Room('vault', 'Relic Vault', (3, 3))
    vault.door('w', 'scriptorium')
    vault.obj('chest', 6, 5, contents='elixir').obj('chest', 10, 5, contents='bombs:5')
    vault.prop(P + 'books', 3, 3).prop(P + 'books', 12, 3)
    vault.random_enemies = 0

    chasm = Room('chasm', 'Sky Chasm', (2, 2))
    chasm.door('s', 'scriptorium').door('n', 'gallery')
    chasm.rect(1, 4, 14, 7, 'p')
    chasm.obj('tablet', 3, 10, text='Spread the feather cloak and float across.')
    chasm.mob('bat', 5, 2).mob('bat', 11, 9)

    gallery = Room('gallery', 'Gallery of Wings', (2, 1))
    gallery.door('s', 'chasm').door('n', 'stairs', 'shutter')
    gallery.shutter = 'combat'
    gallery.rect(3, 4, 5, 5, 'p').rect(10, 7, 12, 8, 'p')
    gallery.mob('ghost', 4, 8).mob('ghost', 11, 3).mob('skeleton', 8, 6).mob('bat', 7, 3).mob('bat', 9, 9)

    stairs = Room('stairs', "Angel's Stair", (2, 0))
    stairs.door('s', 'gallery')
    stairs.boss_stairs = True
    stairs.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    stairs.prop(P + 'statue', 3, 7).prop(P + 'statue', 12, 7)
    stairs.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    stairs.random_enemies = 0

    rooms = [entry, nave, cloister, bap, star, font, saints, bell, scr, sanctum, vault, chasm, gallery, stairs]
    return lair(5, 'Forgotten Sanctuary', 8, 'feather_cloak', rooms), rooms
