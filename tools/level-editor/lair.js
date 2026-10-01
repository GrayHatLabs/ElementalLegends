/* Elemental Legends level editor: format 2 (hand-designed lairs, docs/LEVEL_FORMAT.md
 * "Format 2"). Model, JSON in the style of scripts/lairkit.py, the room grid map, doors,
 * the room panel and Check (a port of src/keepdef.rs parse_keep + check_solvable).
 * Loaded before editor.js; only defines things, and uses editor.js globals (S, A, IMG, el,
 * $, commit, ...) when called. */
'use strict';

const F2 = {
  GRID: 13,             // the "at" grid is 13 x 13 screens (0-12)
  CW: 32, CH: 26,       // map cell in px: 2 px per tile
  TILE_CHARS: '.#~icoDplruht',
  BRUSHES: [
    { ch: 'p', name: 'Pit', key: '8' },
    { ch: 'l', name: 'Lava', key: '9' },
    { ch: 'r', name: 'Orange', key: '0' },
    { ch: 'u', name: 'Blue', key: 'u' },
    { ch: 'h', name: 'Hidden', key: 'h' },
    { ch: 't', name: 'Thorns', key: 't' },
  ],
  TILE_NAMES: { p: 'pit', l: 'lava', r: 'orange barrier (starts raised)', u: 'blue barrier (starts lowered)', h: 'hidden bridge', t: 'thorns' },
  SOLID: '#~cDt',       // world.rs solid_tile: wall, water, crack, decor, thorns
  OBJ_TYPES: ['chest', 'big_chest', 'torch', 'block', 'ice_block', 'lever', 'floor_switch', 'crystal_switch', 'crystal',
    'shrine', 'whip_post', 'boulder', 'pot', 'crate', 'tablet', 'prop'],
  // Extra keys per type, in the order lairkit / the shipped files write them.
  OBJ_KEYS: { torch: ['lit'], block: [], ice_block: [], lever: ['sets'], floor_switch: ['sets'], chest: ['contents', 'hidden'],
    big_chest: ['relic'], shrine: ['element'], crystal_switch: [], crystal: ['element', 'order'], whip_post: [], boulder: [],
    pot: [], crate: [], tablet: ['text'], prop: ['name', 'solid'] },
  OBJ_DEFAULT: { lit: false, sets: null, contents: null, hidden: false, relic: null, element: null, order: null, text: null, name: null, solid: true },
  // Objects that block a doorway (lairkit.lint SOLID_OBJ; shrines are walked into, not around).
  SOLID_OBJ: ['torch', 'block', 'ice_block', 'lever', 'big_chest', 'pot', 'crate', 'boulder', 'whip_post', 'crystal', 'crystal_switch', 'tablet'],
  RELICS: ['vine_whip', 'spirit_lantern', 'titan_gloves', 'ember_boots', 'feather_cloak'],
  RELIC_NAMES: { vine_whip: 'Vine Whip', spirit_lantern: 'Spirit Lantern', titan_gloves: 'Titan Gloves', ember_boots: 'Ember Boots', feather_cloak: 'Feather Cloak' },
  SHUTTERS: ['none', 'combat', 'switch', 'torches', 'crystals', 'plates'],
  SHUTTER_INFO: { none: 'no shutter', combat: 'every door seals until all monsters are gone', switch: 'every floor switch pressed',
    torches: 'every torch lit', crystals: 'every crystal lit (in order if they have one)', plates: 'every plate covered (block, ice block or frozen monster)' },
  LOCKS: ['none', 'small', 'big', 'shutter', 'bomb', 'flag'],
  LOCK_INFO: { none: 'always open', small: 'small key (used up)', big: 'big key', shutter: 'this room\'s shutter', bomb: 'cracked wall (bombs, Quake, bolts)', flag: 'lever / floor switch flag' },
  LOCK_COL: { none: '#50dc78', exit: '#7fd8ff', small: '#ffd84a', big: '#ff5a4a', shutter: '#b8bcc8', bomb: '#d08848', flag: '#c490ff', bad: '#ff2a6a' },
  SIDES: ['n', 's', 'e', 'w'],
  SIDE_NAMES: { n: 'north', s: 'south', e: 'east', w: 'west' },
  BACK: [1, 0, 3, 2],
  CONTENTS: ['small_key', 'big_key', 'map', 'finder', 'elixir', 'heart', 'mana', 'potion', 'page', 'key'],
  LOOT: ['key', 'gold', 'bombs', 'elixir', 'elixirs', 'heart', 'mana', 'potion', 'page', 'small_key', 'map', 'finder', 'big_key'],
  ROOM_KEYS: ['id', 'name', 'at', 'size', 'dark', 'tiles', 'boss_stairs', 'shutter', 'doors', 'flag_tiles', 'enemies', 'random_enemies', 'objects'],
  TOP_KEYS: ['format', 'kind', 'number', 'name', 'theme', 'relic', 'rooms'],
  DOOR_KEYS: ['side', 'to', 'seg', 'lock'],
  FLAG_KEYS: ['flag', 'from', 'to'],
  CONTENT_ICON: { small_key: 'small_key', big_key: 'big_key', map: 'dungeon_map', finder: 'finder' },
};

const f2pair = v => Array.isArray(v) && v.length === 2 && v.every(n => Number.isInteger(n) && n >= 0);
/** Room size [w, h] when valid, else [1, 1]. */
function size2(R) { return f2pair(R.size) && R.size.every(n => n >= 1 && n <= 2) ? R.size : [1, 1]; }
/** [cols, rows] of a format 2 room. */
function dims2(R) { const [w, h] = size2(R); return [16 * w, 13 * h]; }
/** Grid rectangle {x, y, w, h} or null if "at" / "size" are broken. */
function geo2(R) {
  if (!f2pair(R.at) || R.at[0] > 12 || R.at[1] > 12) return null;
  if (!f2pair(R.size) || !R.size.every(n => n >= 1 && n <= 2)) return null;
  return { x: R.at[0], y: R.at[1], w: R.size[0], h: R.size[1] };
}
/** tile2 in keepdef.rs: is c a format 2 tile character? */
const tile2ok = c => typeof c === 'string' && c.length > 0 && F2.TILE_CHARS.includes(c[0]);
/** levels.rs loot(): accepted chest contents. */
function loot2(s) {
  if (typeof s !== 'string') return null;
  const i = s.indexOf(':');
  let k = s, n = 1;
  if (i >= 0) {
    k = s.slice(0, i);
    const t = s.slice(i + 1).trim();
    if (!/^\+?\d+$/.test(t)) return null;
    n = Number(t);
    if (!(n > 0) || n > 2147483647) return null;
  }
  return F2.LOOT.includes(k) ? k : null;
}
/** Parse a door lock string: {lock, flag} or null. */
function lock2(s) {
  if (s === 'none' || s === 'small' || s === 'big' || s === 'shutter' || s === 'bomb') return { lock: s, flag: null };
  if (typeof s === 'string' && s.startsWith('flag:') && s.length > 5) return { lock: 'flag', flag: s.slice(5) };
  return null;
}
function blankTiles2(w, h) {
  const cols = 16 * w, rows = 13 * h, t = [];
  for (let y = 0; y < rows; y++) {
    let s = '';
    for (let x = 0; x < cols; x++) s += (x === 0 || y === 0 || x === cols - 1 || y === rows - 1) ? '#' : '.';
    t.push(s);
  }
  return t;
}
function blankRoom2(id, at, size) {
  return { id, name: String(id).toUpperCase(), at: at.slice(), size: size.slice(), dark: false, tiles: blankTiles2(size[0], size[1]),
    boss_stairs: false, shutter: null, doors: [], flag_tiles: [], enemies: [], random_enemies: null, objects: [], extra: {}, problems: [] };
}
function newLevel2(number) {
  const n = Math.min(6, Math.max(1, number | 0));
  const entry = blankRoom2('entry', [6, 11], [1, 1]);
  entry.name = 'ENTRANCE';
  entry.doors.push({ side: 's', to: 'exit', seg: 0, lock: 'none', extra: {} });
  return { format: 2, kind: 'lair', number: n, name: '', theme: null, relic: n <= 5 ? F2.RELICS[n - 1] : null,
    rooms: [entry], extra: {}, problems: [] };
}
const sizeOk2 = R => { const [c, r] = dims2(R); return R.tiles.length === r && R.tiles.every(s => s.length === c); };
/** Pad / trim the tiles to the room size (missing cells become floor inside a wall ring). */
function normalize2(R) {
  const [w, h] = size2(R), blank = blankTiles2(w, h), cols = 16 * w;
  R.tiles = blank.map((b, y) => { const row = R.tiles[y] || ''; return (row + b.slice(row.length)).slice(0, cols); });
}

// ------------------------------------------------------------------ JSON in / out
function fromJson2(j) {
  const L = { format: 2, kind: j.kind, number: j.number, name: j.name === undefined ? '' : j.name,
    theme: j.theme === undefined ? null : j.theme, relic: j.relic === undefined ? null : j.relic,
    rooms: [], extra: {}, problems: [] };
  for (const k of Object.keys(j)) if (!F2.TOP_KEYS.includes(k)) L.extra[k] = j[k];
  if (j.rooms === undefined) L.problems.push('"rooms" is missing');
  else if (!Array.isArray(j.rooms)) L.problems.push('"rooms" must be a list (format 2)');
  else L.rooms = j.rooms.map(roomFromJson2);
  return L;
}
function roomFromJson2(r, i) {
  const R = blankRoom2('', [0, 0], [1, 1]);
  R.tiles = []; R.name = '';
  if (!isObj(r)) { R.id = 'room' + i; R.problems.push('room ' + i + ' is not an object'); return R; }
  R.id = r.id;
  R.name = r.name === undefined ? '' : r.name;
  R.at = Array.isArray(r.at) ? r.at.slice() : r.at;
  R.size = r.size === undefined ? [1, 1] : (Array.isArray(r.size) ? r.size.slice() : r.size);
  R.dark = r.dark === undefined ? false : r.dark;
  R.boss_stairs = r.boss_stairs === undefined ? false : r.boss_stairs;
  R.shutter = r.shutter === undefined ? null : r.shutter;
  R.random_enemies = r.random_enemies === undefined ? null : r.random_enemies;
  if (!Array.isArray(r.tiles)) R.problems.push('"tiles" must be a list of strings');
  else r.tiles.forEach((row, y) => {
    if (typeof row === 'string') R.tiles.push(row);
    else { R.problems.push('tiles row ' + y + ' is not a string'); R.tiles.push(''); }
  });
  const list = (key, conv) => {
    if (r[key] === undefined) return [];
    if (!Array.isArray(r[key])) { R.problems.push('"' + key + '" must be a list'); return []; }
    return r[key].map(conv);
  };
  const rest = (o, known, out) => { for (const k of Object.keys(o)) if (!known.includes(k)) out[k] = o[k]; };
  R.doors = list('doors', d => {
    const o = isObj(d) ? d : {};
    const D = { side: o.side, to: o.to, seg: o.seg === undefined ? 0 : o.seg, lock: o.lock === undefined ? 'none' : o.lock, extra: {} };
    rest(o, F2.DOOR_KEYS, D.extra);
    if (!isObj(d)) D.extra._bad = true;
    return D;
  });
  R.flag_tiles = list('flag_tiles', f => {
    const o = isObj(f) ? f : {};
    const F = { flag: o.flag, from: o.from, to: o.to, extra: {} };
    rest(o, F2.FLAG_KEYS, F.extra);
    if (!isObj(f)) F.extra._bad = true;
    return F;
  });
  R.enemies = list('enemies', e => {
    const o = isObj(e) ? e : {};
    const E = { kind: o.kind, element: o.element === undefined ? null : o.element, x: o.x, y: o.y,
      hp: o.hp === undefined ? null : o.hp, still: o.still === undefined ? false : o.still, extra: {} };
    rest(o, ENEMY_KEYS, E.extra);
    if (!isObj(e)) E.extra._bad = true;
    return E;
  });
  R.objects = list('objects', b => {
    const o = isObj(b) ? b : {};
    const keys = F2.OBJ_KEYS[o.type] || [];
    const O = { type: o.type, x: o.x, y: o.y, extra: {} };
    for (const k of keys) O[k] = o[k] === undefined ? F2.OBJ_DEFAULT[k] : o[k];
    rest(o, ['type', 'x', 'y', ...keys], O.extra);
    if (!isObj(b)) O.extra._bad = true;
    return O;
  });
  rest(r, F2.ROOM_KEYS, R.extra);
  return R;
}
const noBad = extra => Object.entries(extra).filter(([k]) => k !== '_bad');
/** Editor model -> plain object with lairkit's key order (defaults left out). */
function toJsonObj2(L) {
  const o = { format: L.format, kind: L.kind, number: L.number };
  if (L.name !== '' && L.name !== null && L.name !== undefined) o.name = L.name;
  if (L.theme !== null && L.theme !== undefined) o.theme = L.theme;
  if (L.relic !== null && L.relic !== undefined) o.relic = L.relic;
  o.rooms = L.rooms.map(roomToJsonObj2);
  for (const [k, v] of Object.entries(L.extra)) o[k] = v;
  return o;
}
function roomToJsonObj2(R) {
  const o = { id: R.id };
  if (R.name !== '' && R.name !== null && R.name !== undefined) o.name = R.name;
  o.at = R.at;
  if (!(f2pair(R.size) && R.size[0] === 1 && R.size[1] === 1)) o.size = R.size;
  if (R.dark !== false) o.dark = R.dark;
  o.tiles = R.tiles.slice();
  if (R.boss_stairs !== false) o.boss_stairs = R.boss_stairs;
  if (R.shutter !== null && R.shutter !== 'none') o.shutter = R.shutter;
  o.doors = R.doors.map(d => {
    const x = { side: d.side, to: d.to };
    if (d.seg !== 0) x.seg = d.seg;
    if (d.lock !== 'none') x.lock = d.lock;
    for (const [k, v] of noBad(d.extra)) x[k] = v;
    return x;
  });
  if (R.flag_tiles.length) o.flag_tiles = R.flag_tiles.map(f => {
    const x = { flag: f.flag, from: f.from, to: f.to };
    for (const [k, v] of noBad(f.extra)) x[k] = v;
    return x;
  });
  if (R.enemies.length) o.enemies = R.enemies.map(e => {
    const x = { kind: e.kind, x: e.x, y: e.y };
    if (e.element !== null && e.element !== undefined) x.element = e.element;
    if (e.hp !== null && e.hp !== undefined) x.hp = e.hp;
    if (e.still !== false) x.still = e.still;
    for (const [k, v] of noBad(e.extra)) x[k] = v;
    return x;
  });
  if (R.random_enemies !== null && R.random_enemies !== undefined) o.random_enemies = R.random_enemies;
  if (R.objects.length) o.objects = R.objects.map(b => {
    const x = { type: b.type, x: b.x, y: b.y };
    for (const k of F2.OBJ_KEYS[b.type] || []) {
      const v = b[k];
      if (v === undefined || v === F2.OBJ_DEFAULT[k] || (k === 'order' && v === 0)) continue;
      x[k] = v;
    }
    for (const [k, v] of noBad(b.extra)) x[k] = v;
    return x;
  });
  for (const [k, v] of Object.entries(R.extra)) o[k] = v;
  return o;
}
/** Python's json.dumps(indent=2) (ensure_ascii) plus lairkit.compact(): [int, int] pairs on
 * one line, and list items that are objects with 1-8 flat values on one line. */
function pyStr(s) { return JSON.stringify(s).replace(/[\u0080-￿]/g, c => '\\u' + c.charCodeAt(0).toString(16).padStart(4, '0')); }
function pyDump(v, ind, inList) {
  const pad = '  '.repeat(ind), pad1 = '  '.repeat(ind + 1);
  const pair = x => Array.isArray(x) && x.length === 2 && x.every(Number.isInteger);
  const flat = x => (!isObj(x) && !Array.isArray(x)) || pair(x) || (Array.isArray(x) && !x.length) || (isObj(x) && !Object.keys(x).length);
  if (Array.isArray(v)) {
    if (!v.length) return '[]';
    if (pair(v)) return '[' + v[0] + ', ' + v[1] + ']';
    return '[\n' + v.map(x => pad1 + pyDump(x, ind + 1, true)).join(',\n') + '\n' + pad + ']';
  }
  if (isObj(v)) {
    const e = Object.entries(v);
    if (!e.length) return '{}';
    if (inList && e.length <= 8 && e.every(([, x]) => flat(x))) return '{ ' + e.map(([k, x]) => pyStr(k) + ': ' + pyDump(x, 0, false)).join(', ') + ' }';
    return '{\n' + e.map(([k, x]) => pad1 + pyStr(k) + ': ' + pyDump(x, ind + 1, false)).join(',\n') + '\n' + pad + '}';
  }
  if (typeof v === 'string') return pyStr(v);
  if (v === undefined) return 'null';
  return JSON.stringify(v);
}
function stringify2(L) { return pyDump(toJsonObj2(L), 0, false) + '\n'; }

// ------------------------------------------------------------------ the door graph (parse_keep)
/** Cell next to door d (side index, seg) of the room rectangle a. */
function doorCell(a, side, seg) {
  const c = [[a.x + seg, a.y], [a.x + seg, a.y + a.h - 1], [a.x + a.w - 1, a.y + seg], [a.x, a.y + seg]][side];
  const n = [[c[0], c[1] - 1], [c[0], c[1] + 1], [c[0] + 1, c[1]], [c[0] - 1, c[1]]][side];
  return { cell: c, next: n };
}
const inRect = (o, x, y) => x >= o.x && y >= o.y && x < o.x + o.w && y < o.y + o.h;
function roomAtCell(L, x, y, skip) {
  for (let i = 0; i < L.rooms.length; i++) {
    if (i === skip) continue;
    const g = geo2(L.rooms[i]);
    if (g && inRect(g, x, y)) return i;
  }
  return -1;
}
function idIndex(L) {
  const m = new Map();
  L.rooms.forEach((R, i) => { if (typeof R.id === 'string' && R.id && !m.has(R.id)) m.set(R.id, i); });
  return m;
}
/** Declared door -> {side (0-3), seg, to (index | null), lock, flag} or {err}. */
function parseDoor2(d, R, ids) {
  if (d.extra && d.extra._bad) return { err: 'a door is not an object' };
  const side = F2.SIDES.indexOf(d.side);
  if (side < 0) return { err: 'a door needs "side": n, s, e or w (got ' + JSON.stringify(d.side) + ')' };
  const seg = d.seg;
  const [w, h] = size2(R);
  if (!Number.isInteger(seg) || seg < 0) return { err: 'door "seg" must be a whole number 0 or more' };
  if (seg >= (side < 2 ? w : h)) return { err: 'the ' + F2.SIDE_NAMES[d.side] + ' door "seg" ' + seg + ' is past the room\'s edge' };
  if (typeof d.to !== 'string') return { err: 'a door needs "to" (a room id or "exit")' };
  let to = null;
  if (d.to !== 'exit') { if (!ids.has(d.to)) return { err: 'door to unknown room "' + d.to + '"' }; to = ids.get(d.to); }
  const lk = lock2(d.lock);
  if (!lk) return { err: 'unknown door lock ' + JSON.stringify(d.lock) };
  return { side, seg, to, lock: lk.lock, flag: lk.flag };
}
/** The doors as the game builds them: declared doors that are valid, plus the mirrored far
 * sides, with locks copied across. Also the entry room and per-door problems. */
function keepGraph(L) {
  const ids = idIndex(L);
  const geo = L.rooms.map(geo2);
  const G = { rooms: L.rooms.map(() => ({ doors: [] })), entry: -1, issues: [], bad: [] };
  L.rooms.forEach((R, i) => R.doors.forEach((d, di) => {
    const p = parseDoor2(d, R, ids);
    if (p.err) { G.issues.push({ room: i, door: di, msg: p.err }); G.bad.push({ room: i, door: di, side: F2.SIDES.indexOf(d.side), seg: d.seg }); return; }
    p.decl = { room: i, door: di }; p.mirrored = false;
    G.rooms[i].doors.push(p);
  }));
  // Doors must lead into the room they name; then mirror them (keepdef.rs).
  const extra = [];
  G.rooms.forEach((gr, i) => {
    gr.doors = gr.doors.filter(d => {
      if (d.to === null) return true;
      const a = geo[i], o = geo[d.to];
      if (!a || !o) return true; // "at" / "size" broken: reported elsewhere
      const { next } = doorCell(a, d.side, d.seg);
      if (!inRect(o, next[0], next[1])) {
        G.issues.push({ room: i, door: d.decl.door, msg: 'the ' + F2.SIDE_NAMES[F2.SIDES[d.side]] + ' door doesn\'t lead into "' + L.rooms[d.to].id + '" (they don\'t touch there)' });
        G.bad.push({ room: i, door: d.decl.door, side: d.side, seg: d.seg });
        return false;
      }
      return true;
    });
  });
  G.rooms.forEach((gr, i) => gr.doors.forEach(d => {
    if (d.to === null) return;
    const a = geo[i], o = geo[d.to];
    if (!a || !o) return;
    const { next } = doorCell(a, d.side, d.seg);
    const back = F2.BACK[d.side];
    const bseg = back < 2 ? next[0] - o.x : next[1] - o.y;
    if (!G.rooms[d.to].doors.some(od => od.side === back && od.seg === bseg) && !extra.some(([k, od]) => k === d.to && od.side === back && od.seg === bseg)) {
      extra.push([d.to, { side: back, seg: bseg, to: i, lock: d.lock === 'shutter' ? 'none' : d.lock, flag: d.lock === 'shutter' ? null : d.flag, decl: d.decl, mirrored: true }]);
    }
  }));
  for (const [j, d] of extra) G.rooms[j].doors.push(d);
  G.rooms.forEach((gr, i) => gr.doors.forEach(d => {
    if (d.to === null || !['small', 'big', 'bomb', 'flag'].includes(d.lock)) return;
    const back = F2.BACK[d.side];
    const bd = G.rooms[d.to].doors.find(x => x.side === back && x.to === i);
    if (bd && bd.lock === 'none') { bd.lock = d.lock; bd.flag = d.flag; }
  }));
  G.entry = G.rooms.findIndex(gr => gr.doors.some(d => d.to === null));
  return G;
}
/** Port of keepdef.rs check_solvable. Returns [{msg, room}]. */
function solvable2(L, G) {
  const n = L.rooms.length, out = [];
  if (G.entry < 0) return out;
  const reach = new Array(n).fill(false);
  reach[G.entry] = true;
  let found = 0, used = 0, big = false;
  const flags = [];
  // Things the parser keeps (inside the room, valid contents) that the walk collects.
  const items = L.rooms.map(R => {
    const [cols, rows] = dims2(R);
    return R.objects.filter(o => Number.isInteger(o.x) && Number.isInteger(o.y) && o.x >= 1 && o.y >= 1 && o.x < cols - 1 && o.y < rows - 1).map(o => {
      if (o.type === 'chest') {
        const k = loot2(o.contents === null ? 'gold:20' : o.contents);
        return k === 'small_key' ? { k: 'small' } : k === 'big_key' ? { k: 'big' } : null;
      }
      if ((o.type === 'lever' || o.type === 'floor_switch') && typeof o.sets === 'string') return { k: 'flag', f: o.sets };
      return null;
    }).filter(Boolean);
  });
  const taken = items.map(l => l.map(() => false));
  const opened = G.rooms.map(r => r.doors.map(() => false));
  for (;;) {
    let progress = false;
    for (let i = 0; i < n; i++) {
      if (!reach[i]) continue;
      items[i].forEach((it, oi) => {
        if (taken[i][oi]) return;
        if (it.k === 'small') found++; else if (it.k === 'big') big = true; else flags.push(it.f);
        taken[i][oi] = true; progress = true;
      });
      G.rooms[i].doors.forEach((d, di) => {
        if (d.to === null || opened[i][di]) return;
        let ok = true;
        if (d.lock === 'small') { if (found > used) used++; else ok = false; }
        else if (d.lock === 'big') ok = big;
        else if (d.lock === 'flag') ok = flags.includes(d.flag);
        if (!ok) return;
        opened[i][di] = true;
        const back = F2.BACK[d.side];
        const bi = G.rooms[d.to].doors.findIndex(bd => bd.side === back && bd.to === i);
        if (bi >= 0) opened[d.to][bi] = true;
        reach[d.to] = true;
        progress = true;
      });
    }
    if (!progress) break;
  }
  L.rooms.forEach((R, i) => { if (!reach[i]) out.push({ room: i, msg: 'room "' + R.id + '" can\'t be reached' }); });
  const s = L.rooms.findIndex(R => R.boss_stairs === true);
  if (s >= 0 && !reach[s]) out.push({ room: s, msg: 'the boss stairs can\'t be reached' });
  const bc = L.rooms.findIndex(R => R.objects.some(o => o.type === 'big_chest'));
  if (bc >= 0 && !big) out.push({ room: bc, msg: 'the big chest needs the big key, which is never found' });
  return out;
}

// ------------------------------------------------------------------ Check
function runCheck2(L, add) {
  for (const p of L.problems || []) add('error', null, p);
  if (L.kind !== 'lair') add('error', null, 'format 2 is for lairs ("kind": "lair"), got ' + JSON.stringify(L.kind));
  if (!isInt(L.number) || L.number < 1 || L.number > 6) add('error', null, '"number" must be 1-6 (got ' + JSON.stringify(L.number) + ')');
  if (S.savedName && S.savedName !== fileName(L)) add('warn', null, 'opened as ' + S.savedName + ' but the number makes it ' + fileName(L));
  if (typeof L.name !== 'string') add('error', null, '"name" must be a string');
  else if (!NAME_RE.test(L.name)) add('error', null, 'name "' + L.name + '": upper-case letters, digits and basic punctuation only');
  if (L.theme !== null && (!isInt(L.theme) || L.theme < 0 || L.theme >= THEMES.length)) add('error', null, 'theme must be 0-' + (THEMES.length - 1) + ' (got ' + JSON.stringify(L.theme) + ')');
  if (L.relic !== null && !F2.RELICS.includes(L.relic)) add('error', null, 'unknown relic ' + JSON.stringify(L.relic) + ' (' + F2.RELICS.join(', ') + ')');
  if (L.number === 6 && L.relic !== null) add('warn', null, 'the Dark Tower (lair 6) normally has no relic');
  for (const k of Object.keys(L.extra)) add('warn', null, 'unknown top-level key "' + k + '" (ignored by the game)');
  if (!L.rooms.length) { if (!(L.problems || []).length) add('error', null, '"rooms" is empty'); return; }
  // Ids.
  const seen = new Map();
  L.rooms.forEach((R, i) => {
    if (typeof R.id !== 'string' || !R.id) add('error', i, 'room ' + i + ' has no "id"');
    else if (seen.has(R.id)) add('error', i, 'two rooms are called "' + R.id + '"');
    else seen.set(R.id, i);
    if (R.id === 'exit') add('error', i, 'a room can\'t be called "exit" (doors use it for the way out)');
  });
  L.rooms.forEach((R, i) => checkRoom2(L, i, R, add));
  // Overlaps.
  const geo = L.rooms.map(geo2);
  for (let i = 0; i < L.rooms.length; i++) for (let j = 0; j < i; j++) {
    const a = geo[i], b = geo[j];
    if (a && b && a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h) add('error', i, 'rooms "' + L.rooms[i].id + '" and "' + L.rooms[j].id + '" overlap');
  }
  // Doors as the game links them.
  const G = keepGraph(L);
  for (const it of G.issues) add('error', it.room, it.msg, ...doorBox(L.rooms[it.room], L.rooms[it.room].doors[it.door]));
  const exits = L.rooms.map((R, i) => G.rooms[i].doors.some(d => d.to === null) ? i : -1).filter(i => i >= 0);
  if (!exits.length) add('error', null, 'no room has a door out of the lair ("to": "exit")');
  else if (exits.length > 1) add('warn', exits[1], 'more than one room has an exit; the entrance is "' + L.rooms[exits[0]].id + '" (the first)');
  L.rooms.forEach((R, i) => G.rooms[i].doors.forEach((d, di) => {
    if (G.rooms[i].doors.findIndex(x => x.side === d.side && x.seg === d.seg) !== di && !d.mirrored) add('warn', i, 'two doors on the ' + F2.SIDE_NAMES[F2.SIDES[d.side]] + ' side, segment ' + d.seg + ' of "' + R.id + '"');
  }));
  const stairs = L.rooms.map((R, i) => R.boss_stairs === true ? i : -1).filter(i => i >= 0);
  if (!stairs.length) add('warn', null, 'no room has "boss_stairs": the boss can\'t be reached');
  else if (stairs.length > 1) add('warn', stairs[1], 'more than one room has the boss stairs');
  const bigChests = L.rooms.reduce((n, R) => n + R.objects.filter(o => o.type === 'big_chest').length, 0);
  if (!bigChests && isInt(L.number) && L.number <= 5) add('warn', null, 'no big chest: this lair gives no relic');
  if (bigChests > 1) add('warn', null, bigChests + ' big chests (the lair has one relic)');
  const bigKeys = L.rooms.reduce((n, R) => n + R.objects.filter(o => o.type === 'chest' && o.contents === 'big_key').length, 0);
  if (bigKeys > 1) add('warn', null, bigKeys + ' big keys');
  if (bigChests && L.relic === null && !L.rooms.some(R => R.objects.some(o => o.type === 'big_chest' && o.relic !== null))) add('warn', null, 'there is a big chest but no "relic" for it to hold');
  // Flags: who sets them, who uses them.
  const setters = new Set(), users = new Set();
  L.rooms.forEach(R => R.objects.forEach(o => { if ((o.type === 'lever' || o.type === 'floor_switch') && typeof o.sets === 'string' && o.sets) setters.add(o.sets); }));
  L.rooms.forEach((R, i) => {
    R.doors.forEach(d => { const lk = lock2(d.lock); if (lk && lk.lock === 'flag') { users.add(lk.flag); if (!setters.has(lk.flag)) add('warn', i, 'door lock "flag:' + lk.flag + '": no lever or floor switch sets "' + lk.flag + '"', ...doorBox(R, d)); } });
    R.flag_tiles.forEach(f => { if (typeof f.flag === 'string' && f.flag) { users.add(f.flag); if (!setters.has(f.flag)) add('warn', i, 'flag_tiles "' + f.flag + '": no lever or floor switch sets it'); } });
  });
  L.rooms.forEach((R, i) => R.objects.forEach(o => {
    if ((o.type === 'lever' || o.type === 'floor_switch') && typeof o.sets === 'string' && o.sets && !users.has(o.sets)) add('warn', i, o.type + ' sets "' + o.sets + '", which no door or flag_tiles uses', o.x, o.y);
  }));
  // Can it be finished?
  if (G.entry >= 0) for (const p of solvable2(L, G)) add('error', p.room, p.msg);
}
/** Box [x, y, w, h] around a door's gap, for highlighting. */
function doorBox(R, d) {
  const side = F2.SIDES.indexOf(d && d.side);
  if (side < 0 || !Number.isInteger(d.seg)) return [];
  const [cols, rows] = dims2(R);
  const t = gapTiles2(side, d.seg, cols, rows);
  if (!t.length) return [];
  const xs = t.map(a => a[0]), ys = t.map(a => a[1]);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs) - Math.min(...xs) + 1, Math.max(...ys) - Math.min(...ys) + 1];
}
/** Gap tiles of a door (world.rs set_gap_seg). */
function gapTiles2(side, seg, cols, rows) {
  const t = [], ox = seg * 16, oy = seg * 13;
  if (side === 0) for (let x = 6; x <= 9; x++) t.push([ox + x, 0]);
  if (side === 1) for (let x = 6; x <= 9; x++) t.push([ox + x, rows - 1]);
  if (side === 2) for (let y = 5; y <= 7; y++) t.push([cols - 1, oy + y]);
  if (side === 3) for (let y = 5; y <= 7; y++) t.push([0, oy + y]);
  return t.filter(([x, y]) => x >= 0 && y >= 0 && x < cols && y < rows);
}
function checkRoom2(L, i, R, add) {
  const who = typeof R.id === 'string' && R.id ? R.id : 'room ' + i;
  for (const p of R.problems || []) add('error', i, p);
  if (typeof R.name !== 'string') add('error', i, '"name" must be a string');
  else if (!NAME_RE.test(R.name)) add('warn', i, 'name "' + R.name + '": the game shows it in upper case; use letters, digits and basic punctuation');
  if (!f2pair(R.at) || R.at[0] > 12 || R.at[1] > 12) add('error', i, '"at" must be [x, y] with each 0-12 (got ' + JSON.stringify(R.at) + ')');
  if (!f2pair(R.size) || !R.size.every(n => n >= 1 && n <= 2)) add('error', i, '"size" must be 1 or 2 screens each way (got ' + JSON.stringify(R.size) + ')');
  const g = geo2(R);
  if (g && (g.x + g.w > F2.GRID || g.y + g.h > F2.GRID)) add('warn', i, who + ' reaches past the 13 x 13 grid');
  if (typeof R.dark !== 'boolean') add('error', i, '"dark" must be true or false');
  if (typeof R.boss_stairs !== 'boolean') add('error', i, '"boss_stairs" must be true or false');
  if (R.shutter !== null && !F2.SHUTTERS.includes(R.shutter)) add('error', i, 'unknown shutter ' + JSON.stringify(R.shutter));
  if (R.random_enemies !== null && (!isInt(R.random_enemies) || R.random_enemies < 0)) add('error', i, '"random_enemies" must be a whole number 0 or more');
  else if (R.random_enemies > 12) add('warn', i, '"random_enemies" above 12 is treated as 12');
  for (const k of Object.keys(R.extra)) add('warn', i, 'unknown room key "' + k + '" (ignored)');
  // Tiles.
  const [cols, rows] = dims2(R);
  if (R.tiles.length !== rows) add('error', i, '"tiles" has ' + R.tiles.length + ' rows, needs ' + rows + ' for a ' + size2(R).join('x') + ' room', 0, 0, cols, rows);
  R.tiles.forEach((row, y) => { if (y < rows && row.length !== cols) add('error', i, 'tile row ' + y + ' has ' + row.length + ' characters, needs ' + cols, 0, y, cols, 1); });
  let bad = 0, ring = 0, firstRing = null;
  for (let y = 0; y < Math.min(R.tiles.length, rows); y++) for (let x = 0; x < R.tiles[y].length; x++) {
    const c = R.tiles[y][x];
    if (!F2.TILE_CHARS.includes(c)) { if (bad++ < 5) add('error', i, 'unknown tile "' + c + '" at ' + x + ',' + y, x, y); }
    else if ((x === 0 || y === 0 || x === cols - 1 || y === rows - 1) && c !== '#') { ring++; if (!firstRing) firstRing = [x, y]; }
  }
  if (bad > 5) add('error', i, (bad - 5) + ' more unknown tile characters');
  if (ring) add('warn', i, 'outer ring should be "#" (' + ring + ' tile' + (ring > 1 ? 's' : '') + ' not, first at ' + firstRing + ')', firstRing[0], firstRing[1]);
  const T = (x, y) => (R.tiles[y] || '')[x];
  const inside = (x, y) => isInt(x) && isInt(y) && x >= 1 && y >= 1 && x < cols - 1 && y < rows - 1;
  // Doors (side / seg / lock are checked with the graph); doorways blocked from inside.
  const solidAt = new Set(R.objects.filter(o => F2.SOLID_OBJ.includes(o.type) || (o.type === 'prop' && o.solid !== false)).map(o => o.x + ',' + o.y));
  R.doors.forEach(d => {
    for (const k of Object.keys(d.extra)) if (k !== '_bad') add('warn', i, 'door: unknown key "' + k + '"');
    const side = F2.SIDES.indexOf(d.side);
    if (side < 0 || !isInt(d.seg)) return;
    const inner = gapTiles2(side, d.seg, cols, rows).map(([x, y]) => [x + (side === 3) - (side === 2), y + (side === 0) - (side === 1)]);
    if (inner.length && inner.every(([x, y]) => '#~cDtpl'.includes(T(x, y) || '#') || solidAt.has(x + ',' + y))) {
      const hard = inner.some(([x, y]) => '#D'.includes(T(x, y) || '#') || solidAt.has(x + ',' + y));
      const what = [...new Set(inner.map(([x, y]) => T(x, y)).filter(c => '~ctpl'.includes(c)).map(tileName))].join(' / ');
      add('warn', i, 'the ' + F2.SIDE_NAMES[d.side] + ' doorway' + (d.seg ? ' (segment ' + d.seg + ')' : '') + ' to ' + d.to + ' is blocked from inside' +
        (hard ? '' : ' by ' + what + ' (fine if a relic or magic is meant to cross it)'), ...doorBox(R, d));
    }
  });
  // Flag tiles.
  R.flag_tiles.forEach((f, k) => {
    if (f.extra._bad) { add('error', i, 'flag_tiles entry ' + (k + 1) + ' is not an object'); return; }
    if (typeof f.flag !== 'string') add('error', i, 'flag_tiles needs "flag"');
    else if (!f.flag) add('warn', i, 'flag_tiles with an empty flag name');
    if (!tile2ok(f.from) || !tile2ok(f.to)) add('error', i, 'flag_tiles needs "from" and "to" tile characters (got ' + JSON.stringify(f.from) + ', ' + JSON.stringify(f.to) + ')');
    else if (!R.tiles.some(r => r.includes(f.from[0]))) add('warn', i, 'flag_tiles "' + f.flag + '": the room has no "' + f.from[0] + '" tiles to change');
    for (const x of Object.keys(f.extra)) if (x !== '_bad') add('warn', i, 'flag_tiles: unknown key "' + x + '"');
  });
  // Shutter conditions.
  const count = t => R.objects.filter(o => o.type === t).length;
  const plates = [];
  for (let y = 0; y < rows; y++) for (let x = 0; x < cols; x++) if (T(x, y) === 'o') plates.push([x, y]);
  if (R.shutter === 'combat' && !R.enemies.length && R.random_enemies === 0) add('warn', i, 'combat shutter with no monsters: the doors open at once');
  if (R.shutter === 'switch' && !count('floor_switch')) add('warn', i, 'switch shutter without a floor_switch');
  if (R.shutter === 'torches' && !count('torch')) add('warn', i, 'torches shutter without a torch');
  if (R.shutter === 'crystals' && !count('crystal')) add('warn', i, 'crystals shutter without a crystal');
  if (R.shutter === 'plates') {
    if (!plates.length) add('warn', i, 'plates shutter without any "o" plates');
    else if (count('block') + count('ice_block') < plates.length) add('warn', i, plates.length + ' plates but only ' + (count('block') + count('ice_block')) + ' blocks / ice blocks (frozen monsters can cover the rest)', plates[0][0], plates[0][1]);
  }
  const hasShutterDoor = R.doors.some(d => d.lock === 'shutter');
  if (hasShutterDoor && (R.shutter === null || R.shutter === 'none')) add('warn', i, 'a door has lock "shutter" but the room has no shutter condition');
  const orders = R.objects.filter(o => o.type === 'crystal' && isInt(o.order) && o.order > 0).map(o => o.order).sort((a, b) => a - b);
  if (orders.length && orders.some((o, k) => o !== k + 1)) add('warn', i, 'crystal orders should run 1, 2, 3... (got ' + orders.join(', ') + ')');
  if (orders.length && orders.length !== count('crystal')) add('warn', i, 'some crystals have an order and some don\'t');
  // Boss stairs.
  if (R.boss_stairs === true) {
    for (const [x, y] of STAIRS) { const c = T(x, y); if (c !== '.' && c !== undefined) add('warn', i, 'tile ' + x + ',' + y + ' is under the boss stairs (the game overwrites it)', x, y); }
    R.objects.forEach(o => { if (STAIRS.some(([x, y]) => x === o.x && y === o.y)) add('warn', i, o.type + ' on the boss stairs', o.x, o.y); });
  }
  // Enemies.
  R.enemies.forEach((e, k) => {
    const w = 'enemy #' + (k + 1) + ' (' + e.kind + ')';
    if (e.extra._bad) { add('error', i, 'enemy #' + (k + 1) + ' is not an object'); return; }
    if (!KINDS.includes(e.kind)) add('error', i, 'unknown enemy kind ' + JSON.stringify(e.kind) + ' (skipped by the game)', e.x, e.y);
    if (e.element !== null && !ELEMENTS.includes(e.element)) add('error', i, w + ': unknown element ' + JSON.stringify(e.element), e.x, e.y);
    if (!inside(e.x, e.y)) { add('error', i, w + ': outside the room or on its edge (got ' + JSON.stringify(e.x) + ', ' + JSON.stringify(e.y) + '); skipped by the game', isInt(e.x) ? e.x : undefined, isInt(e.y) ? e.y : undefined); return; }
    if (e.hp !== null && (typeof e.hp !== 'number' || !isFinite(e.hp) || e.hp <= 0)) add('error', i, w + ': hp must be a positive number', e.x, e.y);
    if (typeof e.still !== 'boolean') add('error', i, w + ': "still" must be true or false', e.x, e.y);
    const t = T(e.x, e.y);
    if (F2.SOLID.includes(t)) add('error', i, w + ' is in ' + tileName(t) + ' at ' + e.x + ',' + e.y + ' (skipped by the game)', e.x, e.y);
    else if (t === 'p' || t === 'l' || t === 'h') add('warn', i, w + ' stands on ' + tileName(t) + ' at ' + e.x + ',' + e.y, e.x, e.y);
    for (const x of Object.keys(e.extra)) add('warn', i, w + ': unknown key "' + x + '"', e.x, e.y);
  });
  // Objects.
  const taken = {};
  const props = A.props || {};
  R.objects.forEach((o, k) => {
    const w = 'object #' + (k + 1) + ' (' + o.type + ')';
    if (o.extra._bad) { add('error', i, 'object #' + (k + 1) + ' is not an object'); return; }
    if (!F2.OBJ_TYPES.includes(o.type)) add('error', i, 'unknown object type ' + JSON.stringify(o.type) + ' (skipped by the game)', o.x, o.y);
    if (!inside(o.x, o.y)) { add('error', i, w + ': outside the room or on its edge (got ' + JSON.stringify(o.x) + ', ' + JSON.stringify(o.y) + '); skipped by the game'); return; }
    if (o.type === 'chest') {
      if (o.contents === null) add('warn', i, 'chest without "contents" (the game puts gold:20 in it)', o.x, o.y);
      else if (!loot2(o.contents)) add('error', i, 'unknown chest contents ' + JSON.stringify(o.contents) + ' (skipped by the game)', o.x, o.y);
      else if (loot2(o.contents) === 'key') add('warn', i, 'chest "key" is the format 1 lair key; format 2 lairs use small_key / big_key', o.x, o.y);
      if (typeof o.hidden !== 'boolean') add('error', i, w + ': "hidden" must be true or false', o.x, o.y);
      else if (o.hidden && (R.shutter === null || R.shutter === 'none')) add('warn', i, 'hidden chest in a room without a shutter: it never appears', o.x, o.y);
    }
    if (o.type === 'big_chest' && o.relic !== null && !F2.RELICS.includes(o.relic)) add('warn', i, 'big chest: unknown relic ' + JSON.stringify(o.relic) + ' (the game uses the lair\'s relic)', o.x, o.y);
    if ((o.type === 'shrine' || o.type === 'crystal') && !SHRINE_ELEMENTS.includes(o.element)) add('error', i, o.type + ' needs an element: fire, ice, storm or earth (got ' + JSON.stringify(o.element) + '); skipped by the game', o.x, o.y);
    if (o.type === 'crystal' && o.order !== null && (!isInt(o.order) || o.order < 0 || o.order > 9)) add('warn', i, 'crystal order must be 0-9 (got ' + JSON.stringify(o.order) + ')', o.x, o.y);
    if (o.type === 'lever' && (typeof o.sets !== 'string' || !o.sets)) add('warn', i, 'lever without "sets": pulling it does nothing', o.x, o.y);
    if (o.type === 'floor_switch' && (typeof o.sets !== 'string' || !o.sets) && R.shutter !== 'switch') add('warn', i, 'floor switch without "sets" in a room without a switch shutter: it does nothing', o.x, o.y);
    if ((o.type === 'lever' || o.type === 'floor_switch') && o.sets !== null && typeof o.sets !== 'string') add('error', i, w + ': "sets" must be a flag name', o.x, o.y);
    if (o.type === 'torch' && typeof o.lit !== 'boolean') add('error', i, w + ': "lit" must be true or false', o.x, o.y);
    if (o.type === 'tablet' && (typeof o.text !== 'string' || !o.text.trim())) add('warn', i, 'tablet without "text"', o.x, o.y);
    if (o.type === 'prop') {
      if (typeof o.name !== 'string' || !o.name) add('warn', i, 'prop without a "name" (draws nothing)', o.x, o.y);
      else if (Object.keys(props).length && !props[o.name]) add('warn', i, 'prop "' + o.name + '" is not in the art sheets', o.x, o.y);
      if (typeof o.solid !== 'boolean') add('error', i, w + ': "solid" must be true or false', o.x, o.y);
    }
    const t = T(o.x, o.y);
    if (o.type !== 'prop' && (F2.SOLID.includes(t) || t === 'p' || t === 'l' || t === 'h')) add('warn', i, w + ' stands on ' + tileName(t) + ' at ' + o.x + ',' + o.y, o.x, o.y);
    const key = o.x + ',' + o.y;
    if (taken[key]) add('warn', i, w + ' shares tile ' + key + ' with another object', o.x, o.y);
    taken[key] = true;
    for (const x of Object.keys(o.extra)) add('warn', i, w + ': unknown key "' + x + '"', o.x, o.y);
  });
}

// ------------------------------------------------------------------ room canvas features
/** Doors of room i as the game builds them, with labels. */
function roomDoors2(L, i) {
  const G = keepGraph(L);
  const R = L.rooms[i];
  const out = G.rooms[i].doors.map(d => {
    const target = d.to === null ? 'exit' : (d.mirrored ? 'from ' : 'to ') + L.rooms[d.to].id;
    const lk = d.lock === 'flag' ? 'flag:' + d.flag : d.lock;
    return { side: d.side, seg: d.seg, lock: d.to === null ? 'exit' : d.lock, label: target + (lk !== 'none' ? ' [' + lk + ']' : ''), mirrored: d.mirrored, decl: d.decl };
  });
  for (const b of G.bad) if (b.room === i && b.side >= 0 && isInt(b.seg)) {
    const d = R.doors[b.door];
    out.push({ side: b.side, seg: b.seg, lock: 'bad', label: 'BAD: to ' + d.to, mirrored: false, decl: { room: i, door: b.door } });
  }
  return out;
}
/** Tile chars the game puts in door gaps (keep.rs door_tile) and on the boss stairs. */
function applyFeatures2(t, R, i) {
  const [cols, rows] = dims2(R);
  for (const d of roomDoors2(S.level, i)) {
    const ch = { none: '.', exit: '.', shutter: '.', small: 'L', big: 'G', bomb: 'c', flag: 'S', bad: '.' }[d.lock];
    for (const [x, y] of gapTiles2(d.side, d.seg, cols, rows)) if (t[y]) t[y][x] = ch;
  }
  if (R.boss_stairs === true) for (const [x, y] of STAIRS) if (t[y]) t[y][x] = 'B';
}
function drawFeatureMarkers2(R, i) {
  const z = S.zoom;
  const [cols, rows] = dims2(R);
  for (const d of roomDoors2(S.level, i)) {
    const ts = gapTiles2(d.side, d.seg, cols, rows);
    if (!ts.length) continue;
    const xs = ts.map(t => t[0]), ys = ts.map(t => t[1]);
    const x0 = Math.min(...xs), y0 = Math.min(...ys), x1 = Math.max(...xs), y1 = Math.max(...ys);
    const line = F2.LOCK_COL[d.lock] || '#50dc78';
    ctx.fillStyle = line + '38'; ctx.fillRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
    ctx.strokeStyle = line; ctx.lineWidth = 1.5 / z; ctx.setLineDash(d.mirrored ? [1.5 / z * 2, 2 / z * 2] : [3 / z * 2, 2 / z * 2]);
    ctx.strokeRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
    ctx.setLineDash([]);
    const cx = (x0 + x1 + 1) * TS / 2, cy = (y0 + y1 + 1) * TS / 2;
    const off = [[0, TS * 0.95], [0, -TS * 0.95], [-TS * 2.2, 0], [TS * 2.2, 0]][d.side];
    if (z >= 2) label(d.label, cx + off[0], cy + off[1], line, 7);
  }
  if (R.boss_stairs === true) {
    ctx.strokeStyle = '#c490ff'; ctx.lineWidth = 1.5 / z; ctx.setLineDash([4 / z, 3 / z]);
    ctx.strokeRect(7 * TS, 3 * TS, 2 * TS, 2 * TS); ctx.setLineDash([]);
    if (z >= 2) label('boss stairs', 8 * TS, 5 * TS + 6, '#c490ff', 7);
  }
  if (R.dark === true) {
    ctx.fillStyle = 'rgba(0,0,10,0.18)'; ctx.fillRect(0, 0, cols * TS, rows * TS);
    if (z >= 2) label('dark room', cols * TS - 30, 8, '#9fb0ff', 7);
  }
}
// Tile painters for the format 2 characters (also used by the brush swatches).
function tileImg(k) { const im = IMG['tile_' + k]; return ready(im) ? im : null; }
function drawPit(px, py) {
  const im = tileImg('pit');
  if (im) ctx.drawImage(im, px, py, TS, TS);
  else { ctx.fillStyle = '#05060a'; ctx.fillRect(px, py, TS, TS); ctx.strokeStyle = '#30343e'; ctx.lineWidth = 1; ctx.strokeRect(px + 0.5, py + 0.5, TS - 1, TS - 1); }
}
function drawLava(px, py) {
  const im = tileImg('lava');
  if (im) ctx.drawImage(im, px, py, TS, TS);
  else {
    ctx.fillStyle = '#c83a10'; ctx.fillRect(px, py, TS, TS);
    ctx.fillStyle = '#ff9a30'; ctx.fillRect(px + 3, py + 6, 8, 2); ctx.fillRect(px + 12, py + 15, 9, 2);
    ctx.fillStyle = '#ffe060'; ctx.fillRect(px + 6, py + 6, 2, 2);
  }
}
function drawPeg(px, py, col, up) {
  const im = tileImg(col + '_' + (up ? 'up' : 'down'));
  if (im) { ctx.drawImage(im, px, py, TS, TS); return; }
  const [hi, lo] = col === 'orange' ? ['#fcbc3c', '#a86000'] : ['#78a0fc', '#2c3cbc'];
  if (up) { ctx.fillStyle = lo; ctx.fillRect(px + 2, py + 1, TS - 4, TS - 3); ctx.fillStyle = hi; ctx.fillRect(px + 2, py + 1, TS - 4, 6); }
  else { ctx.strokeStyle = lo; ctx.lineWidth = 2; ctx.strokeRect(px + 5, py + 5, TS - 10, TS - 10); }
}
function drawHidden(px, py) {
  drawPit(px, py);
  ctx.strokeStyle = 'rgba(160,230,255,0.9)'; ctx.lineWidth = 1; ctx.setLineDash([2, 2]);
  ctx.strokeRect(px + 2.5, py + 2.5, TS - 5, TS - 5); ctx.setLineDash([]);
  ctx.fillStyle = 'rgba(160,230,255,0.18)'; ctx.fillRect(px + 3, py + 3, TS - 6, TS - 6);
}
function drawThorns(px, py) {
  const im = tileImg('thorns');
  if (im) { ctx.drawImage(im, px, py, TS, TS); return; }
  ctx.fillStyle = '#2c5c18';
  for (let k = 0; k < 4; k++) { const ax = px + (k * 7) % 18, ay = py + (k * 11) % 18; ctx.fillRect(ax, ay + 3, 10, 2); ctx.fillRect(ax + 4, ay, 2, 8); }
}
function drawBigLock(px, py) {
  ctx.fillStyle = '#5a1810'; ctx.fillRect(px + 1, py + 1, TS - 2, TS - 2);
  ctx.fillStyle = '#a82818'; ctx.fillRect(px + 3, py + 3, TS - 6, TS - 6);
  ctx.fillStyle = '#ffd84a'; ctx.fillRect(px + 8, py + 6, 8, 8);
  ctx.fillStyle = '#1a1008'; ctx.fillRect(px + 11, py + 8, 2, 5);
}
/** Overlay painter for a format 2 tile char; returns false if c is not one. */
function drawSpecial2(c, px, py) {
  if (c === 'p') drawPit(px, py);
  else if (c === 'l') drawLava(px, py);
  else if (c === 'r') drawPeg(px, py, 'orange', true);
  else if (c === 'u') drawPeg(px, py, 'blue', false);
  else if (c === 'h') drawHidden(px, py);
  else if (c === 't') drawThorns(px, py);
  else if (c === 'G') drawBigLock(px, py);
  else return false;
  return true;
}
/** Sprite for the format 2 objects (null: use the format 1 lookup). */
function objSprite2(o) {
  let k = null;
  if (o.type === 'crystal') k = 'crystal_' + (SHRINE_ELEMENTS.includes(o.element) ? o.element : 'fire');
  else if (o.type === 'lever') k = 'lever_off';
  else if (o.type === 'prop') {
    const f = (A.props || {})[o.name];
    return f ? { im: IMG['pr_' + o.name], w: f.w, h: f.h, anchor: true } : null;
  } else if (['big_chest', 'whip_post', 'boulder', 'pot', 'crate', 'floor_switch', 'crystal_switch', 'ice_block', 'tablet'].includes(o.type)) k = o.type;
  if (!k) return null;
  const f = (A.objects || {})[k];
  return f ? { im: IMG['ob_' + k], w: f.w, h: f.h, anchor: o.type !== 'lever' } : null;
}
const F2_FALLBACK = { big_chest: '#a81000', whip_post: '#6c4420', boulder: '#6c6c78', pot: '#a85c2c', crate: '#8c5c2c', floor_switch: '#9c9ca8',
  crystal_switch: '#fc9838', crystal: '#c040c0', ice_block: '#78c8ec', tablet: '#6c6c78', prop: '#5c5c64' };
function drawObjExtras2(o, px, py) {
  if (o.type === 'crystal' && isInt(o.order) && o.order > 0) label(String(o.order), px + TS / 2, py + TS - 3, '#fff', 9);
  if ((o.type === 'lever' || o.type === 'floor_switch') && typeof o.sets === 'string' && o.sets && S.zoom >= 2) label(o.sets, px + TS / 2, py - 3, '#c490ff', 7);
  if (o.type === 'prop' && o.solid === false) label('flat', px + TS / 2, py + TS - 3, '#9fe0ff', 6);
  if (o.type === 'big_chest') {
    const r = o.relic || S.level.relic;
    const ic = IMG['re_' + r];
    if (ready(ic)) ctx.drawImage(ic, px + TS - 8, py - 8, 14, 14);
  }
  if (o.type === 'tablet' && S.zoom >= 2) label('T', px + 4, py + 4, '#ffe066', 7);
}

// ------------------------------------------------------------------ room grid map
const mapCv = () => $('mapCanvas');
/** The part of the 13 x 13 grid the map shows: the rooms plus a one-cell margin (or the
 * whole grid with "Full grid"), at 2-4 px per tile. */
let MV = { x0: 0, y0: 0, cols: F2.GRID, rows: F2.GRID, cw: F2.CW, ch: F2.CH, k: 2 };
function mapView(L) {
  let x0 = 0, y0 = 0, x1 = F2.GRID - 1, y1 = F2.GRID - 1;
  const gs = L.rooms.map(geo2).filter(Boolean);
  if (!S.mapFull && gs.length) {
    x0 = Math.max(0, Math.min(...gs.map(g => g.x)) - 1); y0 = Math.max(0, Math.min(...gs.map(g => g.y)) - 1);
    x1 = Math.min(F2.GRID - 1, Math.max(...gs.map(g => g.x + g.w - 1)) + 1); y1 = Math.min(F2.GRID - 1, Math.max(...gs.map(g => g.y + g.h - 1)) + 1);
    while (x1 - x0 + 1 < 6) { if (x1 < F2.GRID - 1) x1++; else x0--; }
    while (y1 - y0 + 1 < 6) { if (y1 < F2.GRID - 1) y1++; else y0--; }
  }
  const cols = x1 - x0 + 1, rows = y1 - y0 + 1;
  const k = Math.max(2, Math.min(4, Math.floor(Math.min(500 / (cols * 16), 380 / (rows * 13)))));
  return { x0, y0, cols, rows, cw: 16 * k, ch: 13 * k, k };
}
const mpx = x => (x - MV.x0) * MV.cw, mpy = y => (y - MV.y0) * MV.ch;
function mapCellFromEvent(ev) {
  const r = mapCv().getBoundingClientRect();
  const cx = Math.floor((ev.clientX - r.left) / MV.cw), cy = Math.floor((ev.clientY - r.top) / MV.ch);
  if (cx < 0 || cy < 0 || cx >= MV.cols || cy >= MV.rows) return null;
  return { x: cx + MV.x0, y: cy + MV.y0 };
}
const MINI = { '#': '#565b68', '.': '#2e3b30', '~': '#2f6fbf', i: '#bfe8ff', c: '#8a5a30', o: '#9ca0aa', D: '#7a5a34', p: '#030305', l: '#e05010',
  r: '#fcbc3c', u: '#3c5cdc', h: '#1a4050', t: '#3c7c20' };
/** Is the rectangle free (inside the grid, no other room)? */
function mapFree(L, x, y, w, h, skip) {
  if (x < 0 || y < 0 || x + w > F2.GRID || y + h > F2.GRID) return false;
  return !L.rooms.some((R, i) => {
    if (i === skip) return false;
    const g = geo2(R);
    return g && x < g.x + g.w && g.x < x + w && y < g.y + g.h && g.y < y + h;
  });
}
function drawMap() {
  const c = mapCv();
  if (!c || !isF2() || S.mapHidden) return;
  const L = S.level;
  if (!S.mapDrag) MV = mapView(L);
  const W = MV.cols * MV.cw, H = MV.rows * MV.ch, k = MV.k;
  if (c.width !== W + 1 || c.height !== H + 1) { c.width = W + 1; c.height = H + 1; }
  const g = c.getContext('2d');
  g.setTransform(1, 0, 0, 1, 0.5, 0.5);
  g.fillStyle = '#101216'; g.fillRect(-1, -1, W + 2, H + 2);
  g.strokeStyle = '#22262f'; g.lineWidth = 1; g.beginPath();
  for (let i = 0; i <= MV.cols; i++) { g.moveTo(i * MV.cw, 0); g.lineTo(i * MV.cw, H); }
  for (let i = 0; i <= MV.rows; i++) { g.moveTo(0, i * MV.ch); g.lineTo(W, i * MV.ch); }
  g.stroke();
  g.fillStyle = '#3a404c'; g.font = '9px Consolas, monospace'; g.textAlign = 'left'; g.textBaseline = 'top';
  for (let i = 0; i < MV.cols; i++) g.fillText(String(i + MV.x0), i * MV.cw + 2, H - 10);
  for (let i = 0; i < MV.rows; i++) g.fillText(String(i + MV.y0), W - 13, i * MV.ch + 2);
  const G = keepGraph(L);
  const bad = new Set((S.issues || []).filter(it => it.sev === 'error' && isInt(it.slot)).map(it => it.slot));
  const drag = S.mapDrag && S.mapDrag.moved ? S.mapDrag : null;
  L.rooms.forEach((R, i) => {
    let gg = geo2(R);
    if (!gg) return;
    if (drag && drag.room === i) gg = Object.assign({}, gg, { x: drag.x, y: drag.y });
    const x0 = mpx(gg.x), y0 = mpy(gg.y), w = gg.w * MV.cw, h = gg.h * MV.ch;
    // Mini tiles: k px per tile.
    for (let y = 0; y < Math.min(R.tiles.length, 13 * gg.h); y++) {
      const row = R.tiles[y];
      for (let x = 0; x < Math.min(row.length, 16 * gg.w); x++) { g.fillStyle = MINI[row[x]] || '#d02050'; g.fillRect(x0 + x * k - 0.5, y0 + y * k - 0.5, k, k); }
    }
    g.fillStyle = R.dark === true ? 'rgba(0,0,12,0.55)' : 'rgba(0,0,0,0.25)'; g.fillRect(x0, y0, w, h);
    const sel = i === S.slot;
    g.strokeStyle = bad.has(i) ? '#ff6b6b' : sel ? '#ffe066' : '#8d94a3'; g.lineWidth = sel ? 2 : 1;
    g.strokeRect(x0 + (sel ? 0.5 : 0), y0 + (sel ? 0.5 : 0), w - (sel ? 1 : 0), h - (sel ? 1 : 0));
    if (drag && drag.room === i) { g.fillStyle = drag.ok ? 'rgba(80,220,120,0.3)' : 'rgba(255,60,60,0.35)'; g.fillRect(x0, y0, w, h); }
    if (S.flash && S.flash.slot === i && Date.now() < S.flash.until && Math.floor(Date.now() / 250) % 2 === 0) {
      g.strokeStyle = '#ff6b6b'; g.lineWidth = 3; g.strokeRect(x0 + 1, y0 + 1, w - 2, h - 2);
    }
    // Label.
    g.font = 'bold 9px Consolas, monospace'; g.textAlign = 'left'; g.textBaseline = 'top';
    let s = String(R.id);
    while (s.length > 1 && g.measureText(s).width > w - 5) s = s.slice(0, -1);
    if (s !== String(R.id)) s = s.slice(0, -1) + '…';
    g.fillStyle = 'rgba(0,0,0,0.85)'; g.fillText(s, x0 + 3.5, y0 + 3.5);
    g.fillStyle = sel ? '#ffe066' : '#fff'; g.fillText(s, x0 + 3, y0 + 3);
    // Badges.
    const badges = [];
    if (G.entry === i) badges.push(['IN', '#50dc78']);
    if (R.boss_stairs === true) badges.push(['B', '#c490ff']);
    if (R.dark === true) badges.push(['D', '#9fb0ff']);
    if (R.objects.some(o => o.type === 'big_chest')) badges.push(['$', '#ff5a4a']);
    let bx = x0 + 3;
    g.font = 'bold 8px Consolas, monospace';
    for (const [t, col] of badges) {
      const bw = g.measureText(t).width + 4;
      g.fillStyle = col; g.fillRect(bx, y0 + h - 11, bw, 9);
      g.fillStyle = '#000'; g.fillText(t, bx + 2, y0 + h - 10);
      bx += bw + 2;
    }
  });
  // Doors.
  L.rooms.forEach((R, i) => {
    const gg = geo2(R);
    if (!gg || (drag && drag.room === i)) return;
    G.rooms[i].doors.forEach(d => {
      if (d.mirrored) return;
      drawMapDoor(g, gg, d.side, d.seg, d.to === null ? 'exit' : d.lock);
    });
  });
  for (const b of G.bad) {
    const gg = geo2(L.rooms[b.room]);
    if (gg && b.side >= 0 && isInt(b.seg)) drawMapDoor(g, gg, b.side, b.seg, 'bad');
  }
  // Placing a new room.
  if (S.mapAdd && S.mapAdd.cell) {
    const { w, h } = S.mapAdd, { x, y } = S.mapAdd.cell;
    const ok = mapFree(L, x, y, w, h, -1);
    g.fillStyle = ok ? 'rgba(80,220,120,0.35)' : 'rgba(255,60,60,0.35)'; g.fillRect(mpx(x), mpy(y), w * MV.cw, h * MV.ch);
    g.strokeStyle = ok ? '#50dc78' : '#ff6b6b'; g.lineWidth = 2; g.strokeRect(mpx(x), mpy(y), w * MV.cw, h * MV.ch);
  }
}
function drawMapDoor(g, gg, side, seg, lock) {
  const col = F2.LOCK_COL[lock] || '#50dc78';
  let cx, cy;
  if (side === 0) { cx = mpx(gg.x + seg + 0.5); cy = mpy(gg.y); }
  else if (side === 1) { cx = mpx(gg.x + seg + 0.5); cy = mpy(gg.y + gg.h); }
  else if (side === 2) { cx = mpx(gg.x + gg.w); cy = mpy(gg.y + seg + 0.5); }
  else { cx = mpx(gg.x); cy = mpy(gg.y + seg + 0.5); }
  const horiz = side < 2;
  const bw = horiz ? 10 : 5, bh = horiz ? 5 : 10;
  g.fillStyle = '#000'; g.fillRect(cx - bw / 2 - 1, cy - bh / 2 - 1, bw + 2, bh + 2);
  g.fillStyle = col; g.fillRect(cx - bw / 2, cy - bh / 2, bw, bh);
  if (lock === 'exit') {
    const [dx, dy] = [[0, -1], [0, 1], [1, 0], [-1, 0]][side];
    g.strokeStyle = col; g.lineWidth = 2; g.beginPath();
    g.moveTo(cx, cy); g.lineTo(cx + dx * 9, cy + dy * 9);
    g.moveTo(cx + dx * 9 - dy * 3 - dx * 3, cy + dy * 9 - dx * 3 - dy * 3); g.lineTo(cx + dx * 9, cy + dy * 9); g.lineTo(cx + dx * 9 + dy * 3 - dx * 3, cy + dy * 9 + dx * 3 - dy * 3);
    g.stroke();
  }
  if (lock === 'bad') { g.strokeStyle = '#fff'; g.lineWidth = 1; g.beginPath(); g.moveTo(cx - 3, cy - 3); g.lineTo(cx + 3, cy + 3); g.moveTo(cx + 3, cy - 3); g.lineTo(cx - 3, cy + 3); g.stroke(); }
}
function mapRoomAt(cell) { return cell ? roomAtCell(S.level, cell.x, cell.y, -1) : -1; }
function wireMap() {
  const c = mapCv();
  c.addEventListener('contextmenu', e => e.preventDefault());
  c.addEventListener('mousedown', ev => {
    if (!isF2() || ev.button !== 0) return;
    ev.preventDefault();
    const cell = mapCellFromEvent(ev);
    if (!cell) return;
    if (S.mapAdd) {
      const { w, h } = S.mapAdd;
      if (!mapFree(S.level, cell.x, cell.y, w, h, -1)) { toast('That space is taken or past the grid edge', true); return; }
      S.mapAdd = null;
      commit(() => {
        const id = uniqueId(S.level, 'room');
        S.level.rooms.push(blankRoom2(id, [cell.x, cell.y], [w, h]));
        S.slot = S.level.rooms.length - 1; S.sel = null; S.doorForm = null;
      });
      toast('Added a ' + w + 'x' + h + ' room; rename it and add doors on the left');
      return;
    }
    const i = mapRoomAt(cell);
    if (i < 0) return;
    if (i !== S.slot) { S.slot = i; S.sel = null; S.doorForm = null; }
    const g = geo2(S.level.rooms[i]);
    S.mapDrag = { room: i, dx: cell.x - g.x, dy: cell.y - g.y, x: g.x, y: g.y, ok: true, moved: false };
    renderAll();
  });
  c.addEventListener('mousemove', ev => {
    if (!isF2()) return;
    const cell = mapCellFromEvent(ev);
    if (S.mapAdd) { S.mapAdd.cell = cell; drawMap(); }
    const d = S.mapDrag;
    if (d && cell) {
      const R = S.level.rooms[d.room], g = geo2(R);
      const nx = Math.max(0, Math.min(F2.GRID - g.w, cell.x - d.dx)), ny = Math.max(0, Math.min(F2.GRID - g.h, cell.y - d.dy));
      if (nx !== d.x || ny !== d.y) { d.x = nx; d.y = ny; d.moved = true; d.ok = mapFree(S.level, nx, ny, g.w, g.h, d.room); drawMap(); }
    }
    const i = mapRoomAt(cell);
    const R = i >= 0 ? S.level.rooms[i] : null;
    $('mapHover').textContent = cell ? 'cell ' + cell.x + ', ' + cell.y + (R ? '  ' + R.id + (R.name ? ' - ' + R.name : '') + '  ' + size2(R).join('x') : '') : '';
  });
  c.addEventListener('mouseleave', () => { if (S.mapAdd) { S.mapAdd.cell = null; drawMap(); } $('mapHover').textContent = ''; });
  window.addEventListener('mouseup', () => {
    const d = S.mapDrag;
    if (!d) return;
    S.mapDrag = null;
    const R = S.level.rooms[d.room];
    if (d.moved && d.ok && R && (R.at[0] !== d.x || R.at[1] !== d.y)) commit(() => { R.at = [d.x, d.y]; });
    else { if (d.moved && !d.ok) toast('Rooms must not overlap', true); renderAll(); }
  });
  $('btnAddRoom').onclick = () => {
    const [w, h] = $('mapSize').value.split('x').map(Number);
    S.mapAdd = S.mapAdd ? null : { w, h, cell: null };
    renderLairBar();
    if (S.mapAdd) toast('Click an empty spot on the grid to place the ' + w + 'x' + h + ' room (Esc cancels)');
  };
  $('btnDelRoom').onclick = deleteRoom2;
  $('room2Section').addEventListener('change', () => { room2Busy = true; }, true);
  $('btnMapToggle').onclick = () => { S.mapHidden = !S.mapHidden; renderLairBar(); drawMap(); };
  $('tMapFull').onchange = e => { S.mapFull = e.target.checked; drawMap(); };
}
function uniqueId(L, base) {
  const ids = new Set(L.rooms.map(R => R.id));
  for (let n = 1; ; n++) if (!ids.has(base + n)) return base + n;
}
function deleteRoom2() {
  const L = S.level, R = L.rooms[S.slot];
  if (!R) return;
  const refs = L.rooms.reduce((n, X) => n + X.doors.filter(d => d.to === R.id).length, 0);
  if (L.rooms.length === 1) { toast('A lair needs at least one room', true); return; }
  if (!confirm('Delete room "' + R.id + '"' + (refs ? ' and the ' + refs + ' door' + (refs > 1 ? 's' : '') + ' leading to it' : '') + '? (Undo brings it back.)')) return;
  commit(() => {
    const id = R.id;
    L.rooms.splice(S.slot, 1);
    for (const X of L.rooms) X.doors = X.doors.filter(d => d.to !== id);
    S.slot = Math.max(0, Math.min(S.slot, L.rooms.length - 1)); S.sel = null; S.doorForm = null;
  });
}
function renderLairBar() {
  $('lairBar').hidden = !isF2();
  if (!isF2()) return;
  $('mapWrap').hidden = !!S.mapHidden;
  $('btnMapToggle').textContent = S.mapHidden ? 'Show map' : 'Hide map';
  $('btnAddRoom').classList.toggle('on', !!S.mapAdd);
  $('btnAddRoom').textContent = S.mapAdd ? 'Cancel add' : 'Add room';
  $('btnDelRoom').disabled = S.level.rooms.length < 2;
  // Room list.
  const ul = $('roomList');
  ul.innerHTML = '';
  const G = keepGraph(S.level);
  const bad = new Set((S.issues || []).filter(it => it.sev === 'error' && isInt(it.slot)).map(it => it.slot));
  S.level.rooms.forEach((R, i) => {
    const tags = [];
    if (G.entry === i) tags.push(el('span', { class: 'tag in' }, 'IN'));
    if (R.boss_stairs === true) tags.push(el('span', { class: 'tag boss' }, 'B'));
    if (R.dark === true) tags.push(el('span', { class: 'tag dark' }, 'D'));
    ul.append(el('li', { class: (i === S.slot ? 'current' : '') + (bad.has(i) ? ' bad' : ''), title: (R.name || '') + '  at ' + JSON.stringify(R.at) + ' size ' + size2(R).join('x'),
      onclick: () => { S.slot = i; S.sel = null; S.doorForm = null; renderAll(); } },
    el('span', { class: 'rid' }, String(R.id)), ' ', el('span', { class: 'rname' }, R.name || ''), ...tags));
  });
}

// ------------------------------------------------------------------ room panel (left)
let room2Key = null;
let room2Busy = false; // a field in the room panel just changed (its change event fires as focus leaves)
function renderRoom2(force) {
  const sec = $('room2Section');
  sec.hidden = !isF2();
  $('roomSection').hidden = isF2();
  if (!isF2()) return;
  const R = room();
  const key = JSON.stringify([S.level, S.slot, S.doorForm]);
  const inPanel = sec.contains(document.activeElement);
  if (key === room2Key && inPanel) return;
  // A change made from the panel itself: rebuild once focus has settled (Tab to the next
  // field), then put the focus back on the same field.
  if ((inPanel || room2Busy) && !force) { clearTimeout(renderRoom2.t); renderRoom2.t = setTimeout(() => renderRoom2(true), 0); return; }
  room2Busy = false;
  const focusK = sec.contains(document.activeElement) ? document.activeElement.getAttribute('data-k') : null;
  room2Key = key;
  const body = $('room2Body');
  body.innerHTML = '';
  if (focusK) setTimeout(() => { const e = body.querySelector('[data-k="' + focusK + '"]'); if (e && document.activeElement !== e) { e.focus(); if (e.select && e.type === 'text') e.select(); } }, 0);
  $('room2Id').textContent = R ? String(R.id) : '';
  if (!R) { body.append(el('div', { class: 'note' }, 'No room selected.')); return; }
  const L = S.level, i = S.slot;
  const set = fn => commit(() => fn(L.rooms[i]));
  const text = (val, onch, attrs) => el('input', Object.assign({ type: 'text', value: val, spellcheck: 'false', onchange: e => onch(e.target.value) }, attrs || {}));
  // Id / name.
  body.append(el('div', { class: 'row' }, el('label', {}, 'Id'), text(typeof R.id === 'string' ? R.id : '', v => renameRoom2(i, v.trim()), { title: 'Doors use this; renaming updates them', 'data-k': 'id' })));
  const nm = text(typeof R.name === 'string' ? R.name : '', v => set(X => { X.name = v.trim().toUpperCase(); }), { placeholder: '(no name)', 'data-k': 'name' });
  nm.addEventListener('input', e => { const p = e.target.selectionStart; e.target.value = e.target.value.toUpperCase(); e.target.setSelectionRange(p, p); });
  body.append(el('div', { class: 'row' }, el('label', {}, 'Name'), nm));
  // Position / size.
  const sz = el('select', { 'data-k': 'size', onchange: e => resizeRoom2(i, e.target.value.split('x').map(Number)) });
  for (const s of ['1x1', '2x1', '1x2', '2x2']) sz.append(el('option', { value: s }, s + (s === '1x1' ? ' (16 x 13)' : s === '2x2' ? ' (32 x 26)' : s === '2x1' ? ' (32 x 13)' : ' (16 x 26)')));
  sz.value = size2(R).join('x');
  body.append(el('div', { class: 'row' }, el('label', {}, 'Size'), sz));
  body.append(el('div', { class: 'note' }, 'At cell ' + JSON.stringify(R.at) + ' - drag the room on the map to move it.'));
  // Flags.
  body.append(checkRow('Dark', 'nearly black without the lantern', R.dark === true, v => set(X => { X.dark = v; })));
  body.append(checkRow('Boss stairs', 'staircase at cols 7-8, rows 3-4', R.boss_stairs === true, v => set(X => { X.boss_stairs = v; })));
  const sh = selectRow('Shutter', R.shutter === null ? 'none' : String(R.shutter), F2.SHUTTERS.map(s => [s, s]), v => set(X => { X.shutter = v === 'none' ? null : v; }));
  body.append(sh, el('div', { class: 'note' }, F2.SHUTTER_INFO[R.shutter === null ? 'none' : R.shutter] || ''));
  const rnd = el('input', { type: 'number', min: 0, max: 12, step: 1, class: 'short', 'data-k': 'random', value: R.random_enemies === null ? '' : R.random_enemies, disabled: R.random_enemies === null,
    onchange: e => set(X => { const n = parseInt(e.target.value, 10); X.random_enemies = isNaN(n) ? 0 : Math.max(0, n); }) });
  body.append(el('div', { class: 'row' }, el('label', {}, 'Random enemies'), el('span', { class: 'inline' },
    el('input', { type: 'checkbox', checked: R.random_enemies !== null, title: 'Override the default count', onchange: e => set(X => { X.random_enemies = e.target.checked ? 0 : null; }) }), rnd)));
  body.append(el('div', { class: 'note' }, R.random_enemies === null ? (R.enemies.length ? 'Default: 0 extra (enemies are listed).' : 'Default: the usual number of random monsters.') : 'Extra monsters at random free spots.'));
  if (!sizeOk2(R)) body.append(el('div', { class: 'note' }, el('b', {}, 'The tiles do not match the room size. '), el('button', { class: 'small', onclick: () => set(normalize2) }, 'Fix size')));
  // Doors.
  body.append(el('h3', {}, 'Doors'));
  body.append(doorList2(L, i));
  body.append(doorForm2(L, i));
  // Flag tiles.
  body.append(el('h3', {}, 'Flag tiles'));
  body.append(flagTiles2(L, i));
  body.append(el('div', { class: 'row' }, el('button', { class: 'small danger', onclick: deleteRoom2, disabled: L.rooms.length < 2 }, 'Delete room')));
}
function renameRoom2(i, id) {
  const L = S.level, R = L.rooms[i], old = R.id;
  if (id === old) return;
  if (!id) { toast('The id can\'t be empty', true); renderAll(); return; }
  if (id === 'exit') { toast('"exit" is reserved for the way out', true); renderAll(); return; }
  if (L.rooms.some((X, k) => k !== i && X.id === id)) { toast('Another room is already called "' + id + '"', true); renderAll(); return; }
  commit(() => {
    R.id = id;
    let n = 0;
    for (const X of L.rooms) for (const d of X.doors) if (d.to === old) { d.to = id; n++; }
    if (n) toast('Renamed; ' + n + ' door' + (n > 1 ? 's' : '') + ' updated');
  });
}
function resizeRoom2(i, size) {
  const L = S.level, R = L.rooms[i], g = geo2(R);
  if (g && !mapFree(L, g.x, g.y, size[0], size[1], i)) { toast('No room for a ' + size.join('x') + ' room here: it would overlap or leave the grid. Move it first.', true); renderAll(); return; }
  commit(() => {
    const [ow, oh] = size2(R), oc = 16 * ow, or = 13 * oh;
    const t = blankTiles2(size[0], size[1]).map(r => r.split(''));
    for (let y = 1; y < t.length - 1; y++) for (let x = 1; x < t[0].length - 1; x++) if (y < or - 1 && x < oc - 1 && R.tiles[y] && R.tiles[y][x] !== undefined) t[y][x] = R.tiles[y][x];
    R.size = size.slice();
    R.tiles = t.map(r => r.join(''));
  });
}
function doorList2(L, i) {
  const G = keepGraph(L);
  const ul = el('ul', { class: 'doorlist' });
  const R = L.rooms[i];
  const issueFor = di => G.issues.find(it => it.room === i && it.door === di);
  R.doors.forEach((d, di) => {
    const lk = typeof d.lock === 'string' ? d.lock : JSON.stringify(d.lock);
    const iss = issueFor(di);
    const lockKey = (lock2(d.lock) || { lock: 'bad' }).lock;
    ul.append(el('li', { class: (S.doorForm && S.doorForm.edit === di ? 'current' : '') + (iss ? ' bad' : ''), title: iss ? iss.msg : 'Click to edit',
      onclick: () => { S.doorForm = { edit: di, side: d.side, seg: d.seg, to: d.to, lock: lockKey === 'bad' ? 'none' : lockKey, flag: (lock2(d.lock) || {}).flag || '' }; renderAll(); } },
    el('span', { class: 'sw', style: 'background:' + F2.LOCK_COL[d.to === 'exit' ? 'exit' : iss ? 'bad' : lockKey] }),
    String(d.side).toUpperCase() + (d.seg ? d.seg : '') + ' → ' + d.to + (lk !== 'none' ? '  [' + lk + ']' : ''),
    el('button', { class: 'small danger x', title: 'Delete this door', onclick: e => { e.stopPropagation(); commit(() => { L.rooms[i].doors.splice(di, 1); S.doorForm = null; }); } }, '×')));
  });
  // Doors declared by neighbours (the game adds this side automatically).
  G.rooms[i].doors.filter(d => d.mirrored).forEach(d => {
    const src = L.rooms[d.decl.room];
    ul.append(el('li', { class: 'mirrored', title: 'Declared in "' + src.id + '"; the game adds this side. Click to go there.',
      onclick: () => { S.slot = d.decl.room; S.sel = null; S.doorForm = null; const sd = src.doors[d.decl.door]; S.doorForm = { edit: d.decl.door, side: sd.side, seg: sd.seg, to: sd.to, lock: (lock2(sd.lock) || { lock: 'none' }).lock, flag: (lock2(sd.lock) || {}).flag || '' }; renderAll(); } },
    el('span', { class: 'sw', style: 'background:' + F2.LOCK_COL[d.lock] }),
    F2.SIDES[d.side].toUpperCase() + (d.seg ? d.seg : '') + ' ← ' + src.id + (d.lock !== 'none' ? '  [' + (d.lock === 'flag' ? 'flag:' + d.flag : d.lock) + ']' : '') + '  (from ' + src.id + ')'));
  });
  if (!ul.childElementCount) ul.append(el('li', { class: 'note' }, 'No doors yet.'));
  return ul;
}
/** Rooms that touch room i at (side, seg): the only valid door targets there. */
function neighbours2(L, i, side, seg) {
  const g = geo2(L.rooms[i]);
  if (!g) return [];
  const s = F2.SIDES.indexOf(side);
  const { next } = doorCell(g, s, seg);
  const j = roomAtCell(L, next[0], next[1], i);
  return j >= 0 ? [j] : [];
}
function doorForm2(L, i) {
  const R = L.rooms[i];
  const [w, h] = size2(R);
  if (!S.doorForm) {
    // Start on the first side / segment without a doorway.
    const used = keepGraph(L).rooms[i].doors;
    let side = 'n', seg = 0;
    found: for (const s of F2.SIDES) for (let k = 0; k < (s === 'n' || s === 's' ? w : h); k++) {
      if (!used.some(d => F2.SIDES[d.side] === s && d.seg === k)) { side = s; seg = k; break found; }
    }
    S.doorForm = { edit: null, side, seg, to: '', lock: 'none', flag: '' };
  }
  const F = S.doorForm;
  const box = el('div', { class: 'doorform' });
  const rerender = () => { room2Key = null; renderRoom2(); };
  const segs = F2.SIDES.includes(F.side) ? (F.side === 'n' || F.side === 's' ? w : h) : 1;
  if (!isInt(F.seg) || F.seg >= segs) F.seg = 0;
  const nb = neighbours2(L, i, F.side, F.seg);
  const targets = [...nb.map(j => [L.rooms[j].id, L.rooms[j].id]), ['exit', 'exit (way out of the lair)']];
  if (F.edit === null && !targets.some(([v]) => v === F.to)) F.to = targets[0][0];
  // Is there already a doorway here?
  const G = keepGraph(L);
  const here = G.rooms[i].doors.find(d => F2.SIDES[d.side] === F.side && d.seg === F.seg && !(d.decl.room === i && d.decl.door === F.edit));
  const sideSel = el('select', { 'data-k': 'dside', onchange: e => { F.side = e.target.value; F.seg = 0; F.to = ''; rerender(); } });
  for (const s of F2.SIDES) sideSel.append(el('option', { value: s }, F2.SIDE_NAMES[s]));
  sideSel.value = F.side;
  const segSel = el('select', { class: 'short', 'data-k': 'dseg', onchange: e => { F.seg = +e.target.value; F.to = ''; rerender(); } });
  for (let k = 0; k < segs; k++) segSel.append(el('option', { value: String(k) }, 'seg ' + k));
  segSel.value = String(F.seg);
  box.append(el('div', { class: 'note' }, F.edit === null ? 'New door' : 'Editing door #' + (F.edit + 1)));
  box.append(el('div', { class: 'row' }, el('label', {}, 'Side'), sideSel, segs > 1 ? segSel : null));
  const toSel = el('select', { onchange: e => { F.to = e.target.value; } });
  for (const [v, t] of targets) toSel.append(el('option', { value: v }, t));
  if (!targets.some(([v]) => v === F.to)) toSel.append(el('option', { value: String(F.to) }, String(F.to) + ' (doesn\'t touch here)'));
  toSel.value = String(F.to);
  box.append(el('div', { class: 'row' }, el('label', {}, 'To'), toSel));
  const lockSel = el('select', { 'data-k': 'dlock', onchange: e => { F.lock = e.target.value; rerender(); } });
  for (const k of F2.LOCKS) lockSel.append(el('option', { value: k }, k + ' - ' + F2.LOCK_INFO[k]));
  lockSel.value = F.lock;
  box.append(el('div', { class: 'row' }, el('label', {}, 'Lock'), lockSel));
  if (F.lock === 'flag') {
    const flags = allFlags2(L);
    const dl = el('datalist', { id: 'flagNames' }, ...flags.map(f => el('option', { value: f })));
    box.append(dl, el('div', { class: 'row' }, el('label', {}, 'Flag'), el('input', { type: 'text', value: F.flag, list: 'flagNames', spellcheck: 'false', placeholder: 'flag name', oninput: e => { F.flag = e.target.value.trim(); } })));
  }
  if (here) box.append(el('div', { class: 'note warnnote' }, 'There is already a door here (' + (here.mirrored ? 'declared in ' + L.rooms[here.decl.room].id : 'door #' + (here.decl.door + 1)) + ').'));
  const apply = () => {
    if (F.lock === 'flag' && !F.flag) { toast('Give the flag a name', true); return; }
    if (here && F.edit === null) { toast('There is already a door on that side and segment', true); return; }
    const lock = F.lock === 'flag' ? 'flag:' + F.flag : F.lock;
    commit(() => {
      const X = L.rooms[i];
      if (F.edit === null) { X.doors.push({ side: F.side, to: F.to, seg: F.seg, lock, extra: {} }); F.edit = null; }
      else { const d = X.doors[F.edit]; d.side = F.side; d.seg = F.seg; d.to = F.to; d.lock = lock; }
      S.doorForm = null; // next: the first free side
    });
  };
  box.append(el('div', { class: 'row' },
    el('button', { class: 'small primary', onclick: apply }, F.edit === null ? 'Add door' : 'Update door'),
    F.edit !== null ? el('button', { class: 'small', onclick: () => { S.doorForm = null; rerender(); } }, 'New') : null));
  return box;
}
function allFlags2(L) {
  const s = new Set();
  L.rooms.forEach(R => {
    R.objects.forEach(o => { if (typeof o.sets === 'string' && o.sets) s.add(o.sets); });
    R.doors.forEach(d => { const lk = lock2(d.lock); if (lk && lk.flag) s.add(lk.flag); });
    R.flag_tiles.forEach(f => { if (typeof f.flag === 'string' && f.flag) s.add(f.flag); });
  });
  return [...s].sort();
}
function flagTiles2(L, i) {
  const R = L.rooms[i];
  const box = el('div', { class: 'flagtiles' });
  const flags = allFlags2(L);
  box.append(el('datalist', { id: 'flagNames2' }, ...flags.map(f => el('option', { value: f }))));
  const charSel = (val, onch) => {
    const s = el('select', { class: 'short', onchange: e => onch(e.target.value) });
    for (const c of F2.TILE_CHARS) s.append(el('option', { value: c }, c + ' ' + tileName(c)));
    if (!tile2ok(val)) s.append(el('option', { value: String(val) }, String(val) + ' (invalid)'));
    s.value = tile2ok(val) ? val[0] : String(val);
    return s;
  };
  R.flag_tiles.forEach((f, k) => {
    const upd = fn => commit(() => fn(L.rooms[i].flag_tiles[k]));
    box.append(el('div', { class: 'row ft' },
      el('input', { type: 'text', value: typeof f.flag === 'string' ? f.flag : '', list: 'flagNames2', placeholder: 'flag', spellcheck: 'false', onchange: e => upd(x => { x.flag = e.target.value.trim(); }) }),
      charSel(f.from, v => upd(x => { x.from = v; })), '→', charSel(f.to, v => upd(x => { x.to = v; })),
      el('button', { class: 'small danger', title: 'Remove', onclick: () => commit(() => { L.rooms[i].flag_tiles.splice(k, 1); }) }, '×')));
  });
  if (!R.flag_tiles.length) box.append(el('div', { class: 'note' }, 'When a flag is set anywhere in the lair, tiles of one kind change in this room (e.g. water "~" drains to floor ".").'));
  box.append(el('button', { class: 'small', onclick: () => commit(() => { L.rooms[i].flag_tiles.push({ flag: flags[0] || 'flag', from: '~', to: '.', extra: {} }); }) }, 'Add flag tile rule'));
  return box;
}

// ------------------------------------------------------------------ object form extras
/** Extra fields for the format 2 object types (appended to the object form). */
function objectFields2(f, o, apply) {
  const L = S.level;
  if (o.type === 'lever' || o.type === 'floor_switch') {
    const flags = allFlags2(L);
    f.append(el('datalist', { id: 'flagNames3' }, ...flags.map(x => el('option', { value: x }))));
    f.append(el('div', { class: 'row' }, el('label', {}, 'Sets'), el('input', { type: 'text', value: typeof o.sets === 'string' ? o.sets : '', list: 'flagNames3', spellcheck: 'false', placeholder: o.type === 'floor_switch' ? '(none: shutter switch)' : 'flag name',
      onchange: e => apply(x => { const v = e.target.value.trim(); x.sets = v === '' ? null : v; }) })));
  }
  if (o.type === 'crystal') {
    f.append(selectRow('Element', o.element === null || o.element === undefined ? '' : String(o.element), [['', '(choose)'], ...SHRINE_ELEMENTS.map(k => [k, k])], v => apply(x => { x.element = v === '' ? null : v; })));
    f.append(el('div', { class: 'row' }, el('label', {}, 'Order'), el('input', { type: 'number', min: 0, max: 9, step: 1, class: 'short', value: o.order === null || o.order === undefined ? '' : o.order,
      onchange: e => apply(x => { const n = parseInt(e.target.value, 10); x.order = isNaN(n) || n === 0 ? null : Math.max(0, Math.min(9, n)); }) }), el('span', { class: 'note' }, 'blank = any order')));
  }
  if (o.type === 'big_chest') {
    f.append(selectRow('Relic', o.relic === null || o.relic === undefined ? '' : String(o.relic), [['', '(the lair\'s: ' + (L.relic || 'none') + ')'], ...F2.RELICS.map(k => [k, F2.RELIC_NAMES[k]])], v => apply(x => { x.relic = v === '' ? null : v; })));
  }
  if (o.type === 'tablet') {
    f.append(el('div', { class: 'row' }, el('label', {}, 'Text'), el('textarea', { rows: 3, spellcheck: 'false', class: 'tabtext', onchange: e => apply(x => { const v = e.target.value.trim(); x.text = v === '' ? null : v; }) }, typeof o.text === 'string' ? o.text : '')));
  }
  if (o.type === 'prop') {
    const props = A.props || {};
    const s = el('select', { onchange: e => apply(x => { x.name = e.target.value === '' ? null : e.target.value; }) });
    s.append(el('option', { value: '' }, '(choose)'));
    const byTheme = {};
    for (const [n, p] of Object.entries(props)) (byTheme[p.theme] = byTheme[p.theme] || []).push(n);
    const lairTheme = L.theme !== null && isInt(L.theme) ? L.theme : defaultTheme(L);
    const order = Object.keys(byTheme).map(Number).sort((a, b) => (a === lairTheme ? -1 : b === lairTheme ? 1 : a - b));
    for (const t of order) {
      const og = el('optgroup', { label: 'Theme ' + t + ' ' + (THEMES[t] || '') + (t === lairTheme ? ' (this lair)' : '') });
      for (const n of byTheme[t].sort()) og.append(el('option', { value: n }, n.replace(/^prop_\d+_/, '') + '  (' + n + ')'));
      s.append(og);
    }
    if (o.name && !props[o.name]) s.append(el('option', { value: String(o.name) }, String(o.name) + ' (no art)'));
    s.value = o.name || '';
    f.append(el('div', { class: 'row' }, el('label', {}, 'Prop'), s));
    f.append(el('div', { class: 'row' }, el('label', {}, ''), el('input', { type: 'text', value: o.name || '', spellcheck: 'false', placeholder: 'prop_<theme>_<name>', onchange: e => apply(x => { const v = e.target.value.trim(); x.name = v === '' ? null : v; }) })));
    f.append(checkRow('Solid', 'blocks walking (off for rugs, floor art)', o.solid !== false, v => apply(x => { x.solid = v; })));
  }
}
/** Default fields when an object becomes / is placed as a format 2 type. */
function objDefaults2(x, tp) {
  const t = x.type;
  if ((t === 'lever' || t === 'floor_switch') && x.sets === undefined) x.sets = tp && tp.sets !== undefined ? tp.sets : null;
  if (t === 'crystal') { if (x.element === undefined || x.element === null) x.element = (tp && tp.element) || 'fire'; if (x.order === undefined) x.order = tp && tp.order !== undefined ? tp.order : null; }
  if (t === 'big_chest' && x.relic === undefined) x.relic = tp && tp.relic !== undefined ? tp.relic : null;
  if (t === 'tablet' && (x.text === undefined)) x.text = tp && tp.text !== undefined ? tp.text : null;
  if (t === 'prop') {
    if (x.name === undefined || x.name === null) {
      const names = Object.keys(A.props || {});
      const th = S.level.theme !== null && isInt(S.level.theme) ? S.level.theme : defaultTheme(S.level);
      x.name = (tp && tp.name) || names.find(n => n.startsWith('prop_' + th + '_')) || names[0] || null;
    }
    if (x.solid === undefined) x.solid = tp && tp.solid !== undefined ? tp.solid : true;
  }
}
/** A new format 2 object from the placing template. */
function newObject2(tp, x, y) {
  const o = { type: tp.type, x, y, extra: {} };
  for (const k of F2.OBJ_KEYS[tp.type] || []) o[k] = tp[k] === undefined ? F2.OBJ_DEFAULT[k] : JSON.parse(JSON.stringify(tp[k]));
  if (tp.type === 'shrine' && !o.element) o.element = 'ice';
  objDefaults2(o, tp);
  return o;
}
