"""Small helpers for designing format 2 lairs in Python (scripts/design_lairs.py).

The JSON files written to levels/ are the real source of truth: they can be opened and
edited in the level editor afterwards. This kit just makes drawing rooms less error-prone.

Tiles: . floor  # wall  ~ water  i ice  c crack  o plate  D decor  p pit  l lava
       r orange barrier  u blue barrier  h hidden bridge  t thorns
"""
import json

RC, RR = 16, 13  # tiles per screen


class Room:
    def __init__(self, rid, name, at, size=(1, 1), dark=False):
        self.id, self.name, self.at, self.size, self.dark = rid, name.upper(), list(at), list(size), dark
        w, h = size[0] * RC, size[1] * RR
        self.g = [['#' if x in (0, w - 1) or y in (0, h - 1) else '.' for x in range(w)] for y in range(h)]
        self.doors, self.objects, self.enemies, self.flag_tiles = [], [], [], []
        self.shutter, self.random_enemies, self.boss_stairs = 'none', None, False

    # ---- tiles
    @property
    def w(self):
        return len(self.g[0])

    @property
    def h(self):
        return len(self.g)

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.g[y][x] = c
        return self

    def rect(self, x0, y0, x1, y1, c):
        """Fill the inclusive rectangle."""
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.set(x, y, c)
        return self

    def frame(self, x0, y0, x1, y1, c):
        for x in range(x0, x1 + 1):
            self.set(x, y0, c).set(x, y1, c)
        for y in range(y0, y1 + 1):
            self.set(x0, y, c).set(x1, y, c)
        return self

    def mirror_x(self, x, y, c):
        """Place a tile and its left-right mirror."""
        return self.set(x, y, c).set(self.w - 1 - x, y, c)

    def pillars(self, pts, c='#'):
        for x, y in pts:
            self.set(x, y, c)
        return self

    def row(self, y, s, x0=1):
        for i, c in enumerate(s):
            if c != ' ':
                self.set(x0 + i, y, c)
        return self

    # ---- contents
    def door(self, side, to, lock='none', seg=0):
        d = {'side': side, 'to': to}
        if seg:
            d['seg'] = seg
        if lock != 'none':
            d['lock'] = lock
        self.doors.append(d)
        return self

    def obj(self, type_, x, y, **kw):
        o = {'type': type_, 'x': x, 'y': y}
        o.update(kw)
        self.objects.append(o)
        return self

    def mob(self, kind, x, y, **kw):
        e = {'kind': kind, 'x': x, 'y': y}
        e.update(kw)
        self.enemies.append(e)
        return self

    def prop(self, name, x, y, solid=True):
        o = {'type': 'prop', 'x': x, 'y': y, 'name': name}
        if not solid:
            o['solid'] = False
        self.objects.append(o)
        self.set(x, y, '.')  # props stand on floor
        return self

    def when(self, flag, frm, to):
        self.flag_tiles.append({'flag': flag, 'from': frm, 'to': to})
        return self

    def to_json(self):
        d = {'id': self.id, 'name': self.name, 'at': self.at}
        if self.size != [1, 1]:
            d['size'] = self.size
        if self.dark:
            d['dark'] = True
        d['tiles'] = [''.join(r) for r in self.g]
        if self.boss_stairs:
            d['boss_stairs'] = True
        if self.shutter != 'none':
            d['shutter'] = self.shutter
        d['doors'] = self.doors
        if self.flag_tiles:
            d['flag_tiles'] = self.flag_tiles
        if self.enemies:
            d['enemies'] = self.enemies
        if self.random_enemies is not None:
            d['random_enemies'] = self.random_enemies
        if self.objects:
            d['objects'] = [{k: v for k, v in o.items() if not k.startswith('_')} for o in self.objects]
        return d


def lair(number, name, theme, relic, rooms):
    return {'format': 2, 'kind': 'lair', 'number': number, 'name': name.upper(), 'theme': theme,
            'relic': relic, 'rooms': [r.to_json() for r in rooms]}


def write(path, data):
    """Pretty JSON with one tile row per line (like the editor writes)."""
    txt = json.dumps(data, indent=2)
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(compact(txt) + '\n')


def compact(txt):
    import re
    # Number pairs like "at": [1, 3] on one line.
    txt = re.sub(r'\[\s*(-?\d+),\s*(-?\d+)\s*\]', r'[\1, \2]', txt)
    # Put small objects/doors/enemies on one line each, as the editor does.
    out, buf, depth = [], [], 0
    lines = txt.split('\n')
    i = 0
    while i < len(lines):
        ln = lines[i]
        s = ln.strip()
        if s == '{' and i + 1 < len(lines) and lines[i + 1].strip().startswith('"') and _small_obj(lines, i):
            j = i
            parts = []
            while True:
                parts.append(lines[j].strip())
                if lines[j].strip().startswith('}'):
                    break
                j += 1
            indent = ln[:len(ln) - len(ln.lstrip())]
            body = ' '.join(parts[1:-1])
            out.append(f"{indent}{{ {body} {parts[-1]}")
            i = j + 1
            continue
        out.append(ln)
        i += 1
    return '\n'.join(out)


def _small_obj(lines, i):
    j = i + 1
    n = 0
    while j < len(lines) and not lines[j].strip().startswith('}'):
        s = lines[j].strip()
        if s.endswith('[') or s.endswith('{'):
            return False
        n += 1
        j += 1
    return 0 < n <= 8


# ---------------------------------------------------------------- design checks
WALK = set('.oiDh')  # D is solid but drawn as floor; treated separately below
SOLID_OBJ = {'torch', 'block', 'ice_block', 'lever', 'big_chest', 'pot', 'crate', 'boulder', 'whip_post',
             'crystal', 'crystal_switch', 'tablet'}


def door_cells(room, d):
    """Floor cells just inside a door gap."""
    side, seg = d['side'], d.get('seg', 0)
    ox, oy = seg * RC, seg * RR
    if side == 'n':
        return [(ox + x, 1) for x in range(6, 10)]
    if side == 's':
        return [(ox + x, room.h - 2) for x in range(6, 10)]
    if side == 'e':
        return [(room.w - 2, oy + y) for y in range(5, 8)]
    return [(1, oy + y) for y in range(5, 8)]


def lint(rooms, extra_walk=''):
    """Print problems: blocked doorways, objects in walls, unreachable things per room.
    `extra_walk` adds tile kinds the player can cross (e.g. 'l' with the Ember Boots)."""
    problems = []
    for r in rooms:
        walk = set('.oih') | set(extra_walk)
        solid = {(o['x'], o['y']) for o in r.objects if o['type'] in SOLID_OBJ or (o['type'] == 'prop' and o.get('solid', True))}
        for o in r.objects:
            if r.g[o['y']][o['x']] not in walk and o['type'] != 'prop':
                problems.append(f"{r.id}: {o['type']} at {o['x']},{o['y']} stands on '{r.g[o['y']][o['x']]}'")
        for e in r.enemies:
            if r.g[e['y']][e['x']] not in walk:
                problems.append(f"{r.id}: {e['kind']} at {e['x']},{e['y']} is in '{r.g[e['y']][e['x']]}'")
        starts = []
        for d in r.doors:
            cells = door_cells(r, d)
            open_cells = [c for c in cells if r.g[c[1]][c[0]] in walk and c not in solid]
            if not open_cells:
                problems.append(f"{r.id}: the {d['side']} door is blocked inside")
            starts += open_cells
        seen = set(starts)
        stack = list(starts)
        while stack:
            x, y = stack.pop()
            for nx, ny in ((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)):
                if 0 < nx < r.w - 1 and 0 < ny < r.h - 1 and (nx, ny) not in seen and r.g[ny][nx] in walk and (nx, ny) not in solid:
                    seen.add((nx, ny))
                    stack.append((nx, ny))
        for d in r.doors:
            if not any(c in seen for c in door_cells(r, d)):
                problems.append(f"{r.id}: the {d['side']} door can't be walked to")
        for o in r.objects:
            x, y = o['x'], o['y']
            near = [(x, y), (x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
            if not any(c in seen for c in near) and not o.get('_ok'):
                problems.append(f"{r.id}: {o['type']} at {x},{y} can't be reached on foot")
    return problems
