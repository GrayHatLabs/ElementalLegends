"""Builds src/selftest_keep.json: a small format 2 lair that exercises every keep mechanic
for the self-test (src/snapshot.rs). Run: python scripts/selftest_keep.py"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from lairkit import Room, lair, write  # noqa: E402

entry = Room('entry', 'Test Entrance', (1, 3))
entry.door('s', 'exit').door('n', 'hall')
entry.obj('tablet', 4, 3, text='Test tablet text')
entry.obj('pot', 3, 9).obj('pot', 12, 9)
entry.obj('chest', 12, 4, contents='small_key')

hall = Room('hall', 'Test Hall', (1, 2))
hall.door('s', 'entry').door('e', 'east', 'small').door('w', 'west').door('n', 'stairs', 'big')
hall.rect(3, 1, 3, 5, '~').rect(1, 5, 3, 5, '~')
hall.when('drain', '~', '.')
hall.obj('chest', 1, 2, contents='finder')

east = Room('east', 'Dark Test Room', (2, 2), dark=True)
east.door('w', 'hall')
east.obj('crystal_switch', 8, 4)
east.frame(11, 5, 13, 7, 'r')
east.obj('chest', 12, 6, contents='big_key')

west = Room('west', 'Crystal Test Room', (0, 2))
west.door('e', 'hall', 'shutter')
west.shutter = 'crystals'
west.obj('crystal', 5, 4, element='fire', order=1).obj('crystal', 10, 4, element='ice', order=2)
west.obj('shrine', 3, 9, element='fire').obj('shrine', 12, 9, element='ice')
west.obj('chest', 8, 6, contents='map', hidden=True)
west.obj('floor_switch', 8, 10, sets='drain')
west.random_enemies = 0

stairs = Room('stairs', 'Test Stairs', (1, 1))
stairs.boss_stairs = True
stairs.door('s', 'hall').door('e', 'trial')
stairs.rect(6, 8, 7, 10, 'p')
stairs.obj('whip_post', 4, 9)
stairs.obj('big_chest', 12, 9)
stairs.random_enemies = 0

trial = Room('trial', 'Test Trials', (2, 1))
trial.door('w', 'stairs', 'shutter')
trial.shutter = 'plates'
trial.set(12, 3, 'o')
trial.obj('ice_block', 4, 3)
trial.obj('boulder', 8, 10)
trial.rect(3, 9, 4, 11, 'l')
trial.random_enemies = 0

data = lair(1, 'Test Keep', 4, 'vine_whip', [entry, hall, east, west, stairs, trial])
out = pathlib.Path(__file__).resolve().parent.parent / 'src' / 'selftest_keep.json'
write(out, data)
print('wrote', out)
