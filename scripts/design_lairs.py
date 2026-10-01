"""Hand-designed lairs (format 2). Writes levels/lair_<n>.json, then lints every room.

    python scripts/design_lairs.py        # write all lairs
    python scripts/design_lairs.py 1 3    # just lairs 1 and 3

The JSON files are the source of truth afterwards (editable in tools/level-editor); this
script is how the first versions were drawn. Re-running it overwrites the files.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
sys.path.insert(0, str(pathlib.Path(__file__).parent / 'lairs'))
from lairkit import Room, lair, lint, write  # noqa: E402

LEVELS = pathlib.Path(__file__).resolve().parent.parent / 'levels'


# ====================================================================== lair 1
def lair1():
    """OVERGROWN SHRINE - the Vine Whip. 9 rooms.
    Gallery (crystal switch) -> small key -> Thorn Nest (fight) -> finder -> Seed Vault
    (torches) -> big key -> Hall of the Great Seed (big chest: whip) -> Root Hall chasm
    (whip posts) -> lever -> Guardian's Stair."""
    P = 'prop_4_'
    entry = Room('entry', 'Shrine Gate', (1, 4))
    entry.door('s', 'exit').door('n', 'atrium').door('e', 'cellar')
    entry.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    entry.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    entry.prop(P + 'roots', 8, 7, solid=False)
    entry.obj('tablet', 5, 2, text='Shrine of the Vine. Its guardian stirs above.')
    entry.mob('slime', 5, 8).mob('slime', 10, 8)

    cellar = Room('cellar', 'Fungus Cellar', (2, 4))
    cellar.door('w', 'entry')
    cellar.rect(9, 1, 9, 4, '#').rect(9, 8, 9, 11, '#')
    for x, y in [(3, 3), (4, 3), (12, 9), (3, 9), (13, 2)]:
        cellar.obj('pot', x, y)
    for x, y in [(12, 3), (13, 4), (2, 9)]:
        cellar.obj('crate', x, y)
    cellar.obj('chest', 12, 6, contents='bombs:3')
    cellar.obj('tablet', 6, 9, text='Fire wakes the cold braziers of the Seed Vault.')
    cellar.prop(P + 'clump', 5, 2).prop(P + 'clump', 6, 10)
    cellar.mob('slime', 6, 6)

    atrium = Room('atrium', 'Vine Atrium', (1, 3))
    atrium.door('s', 'entry').door('w', 'gallery').door('e', 'nest', 'small').door('n', 'roothall')
    atrium.pillars([(4, 4), (11, 4), (4, 8), (11, 8)])
    atrium.prop(P + 'banner', 3, 1).prop(P + 'banner', 12, 1)
    atrium.prop(P + 'pillar', 2, 3).prop(P + 'pillar', 13, 3)
    atrium.obj('tablet', 5, 2, text='The Great Seed sleeps behind a golden lock.')
    atrium.obj('pot', 2, 10).obj('pot', 13, 10)
    atrium.mob('bat', 7, 6).mob('imp', 9, 9)

    gallery = Room('gallery', 'Moss Gallery', (0, 3))
    gallery.door('e', 'atrium').door('n', 'seedhall', 'big')
    gallery.frame(7, 3, 9, 5, 'r')
    gallery.obj('chest', 8, 4, contents='small_key', _ok=1)
    gallery.obj('crystal_switch', 4, 9)
    gallery.obj('chest', 12, 10, contents='map')
    gallery.obj('tablet', 11, 2, text='Strike the crystal and the bars will fall.')
    gallery.prop(P + 'pillar', 2, 2).prop(P + 'pillar', 2, 10)
    gallery.mob('skeleton', 6, 8).mob('slime', 11, 7)

    seedhall = Room('seedhall', 'Hall of the Great Seed', (0, 2))
    seedhall.door('s', 'gallery')
    seedhall.obj('big_chest', 8, 4)
    seedhall.prop(P + 'statue', 5, 4).prop(P + 'statue', 11, 4)
    seedhall.prop(P + 'brazier', 3, 2).prop(P + 'brazier', 12, 2)
    seedhall.prop(P + 'roots', 8, 8, solid=False)
    seedhall.obj('tablet', 4, 9, text='With the vine, cross the chasm of the Root Hall.')
    seedhall.random_enemies = 0

    nest = Room('nest', 'Thorn Nest', (2, 3))
    nest.door('w', 'atrium').door('n', 'vault')
    nest.shutter = 'combat'
    nest.prop(P + 'clump', 2, 2).prop(P + 'clump', 13, 2).prop(P + 'clump', 2, 10).prop(P + 'clump', 13, 10)
    nest.pillars([(5, 6), (10, 6)])
    nest.mob('slime', 5, 4).mob('slime', 10, 4).mob('bat', 8, 5).mob('imp', 4, 9).mob('skeleton', 11, 9)
    nest.obj('chest', 8, 8, contents='finder', hidden=True)

    vault = Room('vault', 'Seed Vault', (2, 2))
    vault.door('s', 'nest')
    vault.shutter = 'torches'
    for x, y in [(3, 3), (12, 3), (3, 9), (12, 9)]:
        vault.obj('torch', x, y)
    vault.obj('shrine', 8, 9, element='fire')
    vault.obj('chest', 8, 4, contents='big_key', hidden=True)
    vault.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    vault.random_enemies = 1

    root = Room('roothall', 'Root Hall', (1, 2))
    root.door('s', 'atrium').door('n', 'stairs', 'flag:roots')
    root.rect(1, 4, 14, 6, 'p')
    root.obj('whip_post', 4, 2, _ok=1).obj('whip_post', 11, 9)
    root.obj('lever', 12, 2, sets='roots', _ok=1)
    # A thorny alcove with gold, cut open with the whip.
    root.rect(3, 1, 3, 3, '#').rect(1, 3, 2, 3, 't')
    root.obj('chest', 1, 1, contents='gold:50', _ok=1)
    root.obj('tablet', 6, 9, text='A lever across the chasm opens the guardian\'s stair.')
    root.prop(P + 'roots', 8, 9, solid=False)
    root.mob('bat', 8, 8).mob('bat', 13, 2)

    stairs = Room('stairs', "Guardian's Stair", (1, 1))
    stairs.door('s', 'roothall')
    stairs.boss_stairs = True
    stairs.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    stairs.prop(P + 'pillar', 3, 7).prop(P + 'pillar', 12, 7)
    stairs.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    stairs.random_enemies = 0

    rooms = [entry, cellar, atrium, gallery, seedhall, nest, vault, root, stairs]
    return lair(1, 'Overgrown Shrine', 4, 'vine_whip', rooms), rooms


# ====================================================================== lair 2
def lair2():
    """UNDERGROUND CRYPT - the Spirit Lantern. 11 rooms.
    Ossuary (whip cuts thorns) -> key 1 -> Catacombs (fight) -> finder -> Embalming Room
    (ice blocks onto plates) -> key 2. Nave crystal switch lowers the bars to the dark
    Side Chapels (whip over the chasm) -> bell-rope switch opens the Choir -> crystals in
    order (fire, ice, storm) -> big key -> Reliquary (big chest: lantern) -> Bridge of Souls
    (hidden bridge, key 2 door) -> Bone Throne Stair."""
    P = 'prop_5_'
    entry = Room('entry', 'Crypt Stairs', (2, 5))
    entry.door('s', 'exit').door('n', 'nave').door('w', 'ossuary').door('e', 'catacomb', 'small')
    entry.prop(P + 'statue', 3, 3).prop(P + 'statue', 12, 3)
    entry.prop(P + 'brazier', 3, 9).prop(P + 'brazier', 12, 9)
    entry.prop(P + 'runes', 8, 7, solid=False)
    entry.obj('tablet', 5, 2, text='The dead keep a light that shows the hidden way.')
    entry.mob('skeleton', 5, 8).mob('bat', 10, 6)

    oss = Room('ossuary', 'Ossuary', (1, 5))
    oss.door('e', 'entry')
    oss.rect(11, 1, 11, 4, '#').rect(12, 4, 14, 4, 't')
    oss.obj('chest', 13, 2, contents='small_key', _ok=1)
    for x, y in [(3, 3), (6, 9), (8, 3), (4, 11)]:
        oss.prop(P + 'bones', x, y)
    oss.obj('pot', 2, 10).obj('pot', 12, 10).obj('pot', 13, 9)
    oss.obj('tablet', 9, 9, text='Bones and thorns guard the key. Cut your way through.')
    oss.mob('zombie', 6, 6).mob('skeleton', 9, 7)

    nave = Room('nave', 'Great Nave', (2, 4))
    nave.door('s', 'entry').door('w', 'chapels').door('e', 'reliquary', 'big').door('n', 'choir', 'flag:bellrope')
    nave.pillars([(4, 3), (11, 3), (4, 9), (11, 9)])
    nave.rect(2, 4, 2, 8, 'r')
    nave.rect(5, 2, 10, 2, 'u')
    nave.obj('crystal_switch', 8, 7)
    nave.obj('tablet', 13, 10, text='Ring the bell rope in the chapels to open the choir.')
    nave.prop(P + 'banner', 6, 1).prop(P + 'banner', 9, 1)
    nave.mob('skeleton', 6, 9).mob('zombie', 10, 8)

    chap = Room('chapels', 'Side Chapels', (1, 4), dark=True)
    chap.door('e', 'nave').door('w', 'charnel', 'bomb')
    chap.rect(6, 1, 8, 11, 'p')
    chap.obj('whip_post', 4, 6).obj('whip_post', 11, 4)
    chap.obj('floor_switch', 3, 9, sets='bellrope', _ok=1)
    chap.obj('chest', 2, 2, contents='map', _ok=1)
    chap.obj('pot', 13, 10).obj('pot', 12, 2)
    chap.prop(P + 'statue', 2, 5).prop(P + 'statue', 13, 7)
    chap.mob('bat', 12, 9).mob('ghost', 3, 4)

    charnel = Room('charnel', 'Charnel Pit', (0, 4))
    charnel.door('e', 'chapels')
    for x, y in [(3, 3), (12, 3), (3, 9), (12, 9), (6, 2)]:
        charnel.prop(P + 'bones', x, y)
    charnel.obj('chest', 8, 6, contents='elixir')
    charnel.obj('tablet', 8, 10, text='The forgotten dead rest here. Take what they cannot.')
    charnel.mob('zombie', 5, 7).mob('zombie', 11, 7)

    cat = Room('catacomb', 'Catacombs', (3, 5), size=(2, 1))
    cat.door('w', 'entry').door('n', 'embalmer', seg=1)
    cat.shutter = 'combat'
    for x in (5, 10, 21, 26):
        cat.rect(x, 3, x + 1, 3, '#').rect(x, 9, x + 1, 9, '#')
    for x, y in [(3, 2), (14, 10), (17, 2), (29, 10)]:
        cat.prop(P + 'bones', x, y)
    cat.mob('skeleton', 8, 6).mob('skeleton', 16, 4).mob('skeleton', 24, 8)
    cat.mob('zombie', 12, 8).mob('zombie', 20, 5).mob('ghost', 28, 6)
    cat.obj('chest', 16, 6, contents='finder', hidden=True)

    emb = Room('embalmer', 'Embalming Room', (4, 4))
    emb.door('s', 'catacomb')
    emb.shutter = 'plates'
    emb.set(4, 3, 'o').set(11, 3, 'o')
    emb.obj('ice_block', 4, 8).obj('ice_block', 11, 8)
    emb.obj('chest', 8, 5, contents='small_key', hidden=True)
    emb.obj('tablet', 8, 10, text='The cold slabs slide where they are pushed.')
    emb.prop(P + 'brazier', 2, 1).prop(P + 'brazier', 13, 1)
    emb.random_enemies = 1

    choir = Room('choir', 'Choir Loft', (2, 3))
    choir.door('s', 'nave').door('n', 'bridge', 'small')
    choir.shutter = 'crystals'
    choir.obj('crystal', 4, 3, element='fire', order=1).obj('crystal', 11, 3, element='ice', order=2)
    choir.obj('crystal', 8, 2, element='storm', order=3)
    choir.obj('shrine', 3, 9, element='fire').obj('shrine', 12, 9, element='ice').obj('shrine', 8, 10, element='storm')
    choir.obj('chest', 8, 6, contents='big_key', hidden=True)
    choir.obj('tablet', 5, 10, text='Flame, then frost, then storm.')
    choir.mob('ghost', 5, 7)

    rel = Room('reliquary', 'Reliquary', (3, 4))
    rel.door('w', 'nave')
    rel.obj('big_chest', 8, 4)
    rel.prop(P + 'statue', 5, 4).prop(P + 'statue', 11, 4)
    rel.prop(P + 'runes', 8, 8, solid=False)
    rel.random_enemies = 0

    bridge = Room('bridge', 'Bridge of Souls', (2, 2), dark=True)
    bridge.door('s', 'choir').door('n', 'stairs')
    bridge.rect(1, 2, 14, 10, 'p')
    for x, y in [(7, 10), (7, 9), (7, 8), (8, 8), (9, 8), (9, 7), (9, 6), (9, 5), (8, 5), (7, 5), (6, 5), (6, 4), (6, 3), (6, 2)]:
        bridge.set(x, y, 'h')
    bridge.obj('tablet', 12, 11, text='Only the Spirit Lantern reveals the bridge of souls.')
    bridge.mob('bat', 4, 11).mob('bat', 11, 1)

    stairs = Room('stairs', 'Bone Throne Stair', (2, 1))
    stairs.door('s', 'bridge')
    stairs.boss_stairs = True
    stairs.prop(P + 'brazier', 5, 2).prop(P + 'brazier', 10, 2)
    stairs.prop(P + 'pillar', 3, 7).prop(P + 'pillar', 12, 7)
    stairs.prop(P + 'banner', 4, 1).prop(P + 'banner', 11, 1)
    stairs.random_enemies = 0

    rooms = [entry, oss, nave, chap, charnel, cat, emb, choir, rel, bridge, stairs]
    return lair(2, 'Underground Crypt', 5, 'spirit_lantern', rooms), rooms


import lair3  # noqa: E402

LAIRS = {1: lair1, 2: lair2, 3: lair3.build}


def main(args):
    which = [int(a) for a in args] or sorted(LAIRS)
    for n in which:
        data, rooms = LAIRS[n]()
        path = LEVELS / f'lair_{n}.json'
        write(path, data)
        print(f'wrote {path.name}: {len(rooms)} rooms')
        for p in lint(rooms):
            print('   lint:', p)


if __name__ == '__main__':
    main(sys.argv[1:])
