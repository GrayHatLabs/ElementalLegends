/* Elemental Legends level editor. Reads and writes docs/LEVEL_FORMAT.md (format 1 caves /
 * lairs here; format 2 hand-designed lairs in lair.js, which is loaded first).
 * Plain JS, no dependencies; works from file://. Art comes from assets.js
 * (scripts/build_editor_assets.py); anything missing is drawn as a coloured shape. */
'use strict';

// ------------------------------------------------------------------ constants (from the spec)
const COLS = 16, ROWS = 13, TS = 24;
const TILE_CHARS = '.#~icoD';
const BRUSHES = [
  { ch: '.', name: 'Floor', key: '1' },
  { ch: '#', name: 'Wall', key: '2' },
  { ch: '~', name: 'Water', key: '3' },
  { ch: 'i', name: 'Ice', key: '4' },
  { ch: 'c', name: 'Crack', key: '5' },
  { ch: 'o', name: 'Plate', key: '6' },
  { ch: 'D', name: 'Decor', key: '7' },
];
const TILE_NAMES = { '.': 'floor', '#': 'wall', '~': 'water', 'i': 'ice', 'c': 'cracked wall', 'o': 'pressure plate', 'D': 'decoration' };
const THEMES = ['Greenwood', 'Old crypt', 'Mirefen', 'Emberpeak', 'Overgrown shrine', 'Underground crypt',
  'Ruined castle', 'Dragon fortress', 'Forgotten sanctuary', 'Dark tower', 'Village shop', 'Cave'];
const KINDS = ['slime', 'bat', 'skeleton', 'imp', 'ghost', 'golem', 'zombie', 'generator'];
const ELEMENTS = ['fire', 'ice', 'storm', 'earth', 'neutral'];
const SHRINE_ELEMENTS = ['fire', 'ice', 'storm', 'earth'];
// The kind's usual element and base HP (game.rs make_enemy; HP grows with the region).
const USUAL = {
  slime: ['region', 3], bat: ['storm', 2], skeleton: ['neutral', 4], imp: ['fire', 3],
  ghost: ['ice', 3], golem: ['earth', 9], zombie: ['neutral', 10], generator: ['neutral', 8],
};
const OBJ_TYPES = ['torch', 'block', 'lever', 'chest', 'shrine'];
const CONTENT_SIMPLE = ['key', 'elixir', 'heart', 'mana', 'potion', 'page'];
const CONTENT_COUNTED = ['gold', 'bombs', 'elixirs'];
const CONTENT_RE = /^(key|elixir|heart|mana|potion|page|(gold|bombs|elixirs):[1-9][0-9]*)$/;
const NAME_RE = /^[A-Z0-9 .,'!?:;&()\-]*$/;
const PUZZLES = ['none', 'combat', 'torches', 'plates', 'icebridge', 'hidden', 'freeze_plate'];
const SLOTS = {
  cave: ['mouth', 'challenge', 'treasure'],
  lair: ['entry', 'hub', 'west', 'east', 'stairs', 'feast', 'pantry'],
};
const ROOM_KEYS = ['tiles', 'puzzle', 'combat', 'enemies', 'random_enemies', 'objects'];
const TOP_KEYS = ['format', 'kind', 'number', 'name', 'theme', 'rooms'];
const ENEMY_KEYS = ['kind', 'element', 'x', 'y', 'hp', 'still'];
const OBJ_KEYS = { torch: ['lit'], block: [], lever: ['pulled'], chest: ['contents', 'hidden'], shrine: ['element'] };
const STAIRS = [[7, 3], [8, 3], [7, 4], [8, 4]];

// Format 2 (lair.js) switches: the level being edited decides.
const isF2 = () => !!(S.level && S.level.format === 2);
const tileChars = () => isF2() ? F2.TILE_CHARS : TILE_CHARS;
const brushes = () => isF2() ? BRUSHES.concat(F2.BRUSHES) : BRUSHES;
const tileName = c => TILE_NAMES[c] || F2.TILE_NAMES[c] || JSON.stringify(c);
/** [cols, rows] of the current room: 16 x 13 for format 1, 16w x 13h for format 2. */
function dims() {
  if (isF2()) { const R = room(); return R ? dims2(R) : [COLS, ROWS]; }
  return [COLS, ROWS];
}

/** Slots that exist for this level (the tabs). */
function slotsFor(level) {
  if (level.kind === 'lair') {
    const n = level.number;
    return SLOTS.lair.filter(s => (s !== 'feast' || n === 1) && (s !== 'pantry' || n !== 1));
  }
  return SLOTS.cave.slice();
}
/** Allowed puzzle values for a slot (spec: puzzle table + cave challenge row). */
function puzzlesFor(kind, slot) {
  // Matches the game (src/levels.rs): only these rooms run a puzzle; the lair hub's
  // fight is the separate "combat" flag.
  if (kind === 'cave') return slot === 'challenge' ? ['combat', 'freeze_plate', 'none'] : ['none'];
  if (slot === 'west' || slot === 'east') return ['combat', 'torches', 'plates', 'icebridge', 'hidden'];
  return ['none'];
}
/** Door gaps the game carves, from the slot's neighbours: [{side, kind, label}]. */
function gapsFor(level, slot) {
  const g = [];
  const add = (side, kind, label) => g.push({ side, kind, label });
  if (level.kind === 'lair') {
    const n = level.number;
    switch (slot) {
      case 'entry':
        add('n', 'door', 'to hub'); add('s', 'door', 'exit');
        if (n === 1) add('w', 'door', 'to feast');
        else add('e', 'crack', 'cracked wall to pantry');
        break;
      case 'hub': add('s', 'door', 'to entry'); add('n', 'lock', 'locked door to stairs'); add('w', 'door', 'to west'); add('e', 'door', 'to east'); break;
      case 'west': add('e', 'door', 'to hub'); break;
      case 'east': add('w', 'door', 'to hub'); break;
      case 'stairs': add('s', 'door', 'to hub'); break;
      case 'feast': add('e', 'door', 'to entry'); break;
      case 'pantry': add('w', 'door', 'to entry'); break;
    }
  } else {
    const three = !!level.rooms.mouth;
    switch (slot) {
      case 'mouth': add('n', 'door', 'to challenge'); add('s', 'door', 'exit'); break;
      case 'challenge': add('s', 'door', three ? 'to mouth' : 'exit'); add('n', 'seal', 'sealed until solved'); break;
      case 'treasure': add('s', 'door', 'to challenge'); break;
    }
  }
  return g;
}
/** Tiles covered by a gap on a side (world.rs set_gap_seg). */
function gapTiles(side) {
  const t = [];
  if (side === 'n') for (let x = 6; x <= 9; x++) t.push([x, 0]);
  if (side === 's') for (let x = 6; x <= 9; x++) t.push([x, ROWS - 1]);
  if (side === 'e') for (let y = 5; y <= 7; y++) t.push([COLS - 1, y]);
  if (side === 'w') for (let y = 5; y <= 7; y++) t.push([0, y]);
  return t;
}
function defaultTheme(level) {
  if (level.kind === 'lair') return Math.min(Math.max(level.number | 0, 1), 6) + 3;
  return 11; // caves: the rocky cave tileset
}

// ------------------------------------------------------------------ model
function blankRoom() {
  const tiles = [];
  for (let y = 0; y < ROWS; y++) {
    let s = '';
    for (let x = 0; x < COLS; x++) s += (x === 0 || y === 0 || x === COLS - 1 || y === ROWS - 1) ? '#' : '.';
    tiles.push(s);
  }
  return { tiles, puzzle: null, combat: false, enemies: [], random_enemies: null, objects: [], extra: {}, problems: [] };
}
function newLevel(kind, number) {
  return { format: 1, kind, number, name: '', theme: null, rooms: {}, extra: {}, problems: [] };
}
const isObj = v => v !== null && typeof v === 'object' && !Array.isArray(v);
const isInt = v => typeof v === 'number' && Number.isInteger(v);

/** Parsed JSON -> editor model. Keeps odd values so Check can report them. */
function fromJson(j) {
  if (isObj(j) && j.format === 2) return fromJson2(j);
  const L = newLevel('cave', 1);
  L.problems = [];
  if (!isObj(j)) { L.problems.push('The file is not a JSON object'); return L; }
  L.format = j.format;
  L.kind = j.kind;
  L.number = j.number;
  L.name = j.name === undefined ? '' : j.name;
  L.theme = j.theme === undefined ? null : j.theme;
  for (const k of Object.keys(j)) if (!TOP_KEYS.includes(k)) L.extra[k] = j[k];
  if (j.rooms === undefined) L.problems.push('"rooms" is missing');
  else if (!isObj(j.rooms)) L.problems.push('"rooms" must be an object');
  else for (const [slot, r] of Object.entries(j.rooms)) L.rooms[slot] = roomFromJson(r);
  return L;
}
function roomFromJson(r) {
  const R = blankRoom();
  R.tiles = [];
  if (!isObj(r)) { R.problems.push('room is not an object'); return R; }
  if (!Array.isArray(r.tiles)) R.problems.push('"tiles" must be an array of 13 strings');
  else r.tiles.forEach((row, i) => {
    if (typeof row === 'string') R.tiles.push(row);
    else { R.problems.push('tiles row ' + i + ' is not a string'); R.tiles.push(''); }
  });
  R.puzzle = r.puzzle === undefined ? null : r.puzzle;
  R.combat = r.combat === undefined ? false : r.combat;
  R.random_enemies = r.random_enemies === undefined ? null : r.random_enemies;
  const list = (key, conv) => {
    if (r[key] === undefined) return [];
    if (!Array.isArray(r[key])) { R.problems.push('"' + key + '" must be an array'); return []; }
    return r[key].map(conv);
  };
  R.enemies = list('enemies', e => {
    const o = isObj(e) ? e : {};
    const E = { kind: o.kind, element: o.element === undefined ? null : o.element, x: o.x, y: o.y,
      hp: o.hp === undefined ? null : o.hp, still: o.still === undefined ? false : o.still, extra: {} };
    for (const k of Object.keys(o)) if (!ENEMY_KEYS.includes(k)) E.extra[k] = o[k];
    if (!isObj(e)) E.extra._bad = true;
    return E;
  });
  R.objects = list('objects', e => {
    const o = isObj(e) ? e : {};
    const O = { type: o.type, x: o.x, y: o.y, extra: {} };
    const keys = OBJ_KEYS[o.type] || [];
    if (o.type === 'torch') O.lit = o.lit === undefined ? false : o.lit;
    if (o.type === 'lever') O.pulled = o.pulled === undefined ? false : o.pulled;
    if (o.type === 'chest') { O.contents = o.contents === undefined ? null : o.contents; O.hidden = o.hidden === undefined ? false : o.hidden; }
    if (o.type === 'shrine') O.element = o.element === undefined ? null : o.element;
    for (const k of Object.keys(o)) if (!['type', 'x', 'y'].includes(k) && !keys.includes(k)) O.extra[k] = o[k];
    if (!isObj(e)) O.extra._bad = true;
    return O;
  });
  for (const k of Object.keys(r)) if (!ROOM_KEYS.includes(k)) R.extra[k] = r[k];
  return R;
}

/** Editor model -> ordered plain object (spec key order, defaults omitted). */
function toJsonObj(L) {
  const o = {};
  o.format = L.format;
  o.kind = L.kind;
  o.number = L.number;
  if (L.name !== '' && L.name !== null && L.name !== undefined) o.name = L.name;
  if (L.theme !== null && L.theme !== undefined) o.theme = L.theme;
  o.rooms = {};
  const order = [...SLOTS[L.kind === 'lair' ? 'lair' : 'cave'], ...Object.keys(L.rooms)];
  for (const slot of order) {
    if (!L.rooms[slot] || o.rooms[slot]) continue;
    o.rooms[slot] = roomToJsonObj(L.rooms[slot]);
  }
  for (const [k, v] of Object.entries(L.extra)) o[k] = v;
  return o;
}
function roomToJsonObj(R) {
  const o = { tiles: R.tiles.slice() };
  if (R.puzzle !== null && R.puzzle !== undefined) o.puzzle = R.puzzle;
  if (R.combat !== false) o.combat = R.combat;
  if (R.enemies.length) o.enemies = R.enemies.map(e => {
    const x = { kind: e.kind };
    if (e.element !== null && e.element !== undefined) x.element = e.element;
    x.x = e.x; x.y = e.y;
    if (e.hp !== null && e.hp !== undefined) x.hp = e.hp;
    if (e.still !== false) x.still = e.still;
    for (const [k, v] of Object.entries(e.extra)) if (k !== '_bad') x[k] = v;
    return x;
  });
  if (R.random_enemies !== null && R.random_enemies !== undefined) o.random_enemies = R.random_enemies;
  if (R.objects.length) o.objects = R.objects.map(b => {
    const x = { type: b.type, x: b.x, y: b.y };
    if (b.type === 'torch' && b.lit !== false) x.lit = b.lit;
    if (b.type === 'lever' && b.pulled !== false) x.pulled = b.pulled;
    if (b.type === 'chest') {
      if (b.contents !== null && b.contents !== undefined) x.contents = b.contents;
      if (b.hidden !== false) x.hidden = b.hidden;
    }
    if (b.type === 'shrine' && b.element !== null && b.element !== undefined) x.element = b.element;
    for (const [k, v] of Object.entries(b.extra)) if (k !== '_bad') x[k] = v;
    return x;
  });
  for (const [k, v] of Object.entries(R.extra)) o[k] = v;
  return o;
}
/** Pretty JSON like the spec example: tiles one row per line, enemies/objects one per line. */
function stringifyLevel(L) {
  if (L.format === 2) return stringify2(L);
  const o = toJsonObj(L);
  const inline = v => {
    if (isObj(v)) {
      const e = Object.entries(v);
      return e.length ? '{ ' + e.map(([k, x]) => JSON.stringify(k) + ': ' + inline(x)).join(', ') + ' }' : '{}';
    }
    if (Array.isArray(v)) return '[' + v.map(inline).join(', ') + ']';
    return JSON.stringify(v);
  };
  const block = (v, ind, key) => {
    const pad = '  '.repeat(ind), pad1 = '  '.repeat(ind + 1);
    if (Array.isArray(v)) {
      if (!v.length) return '[]';
      const simple = key === 'tiles' || key === 'enemies' || key === 'objects' || v.every(x => !isObj(x) && !Array.isArray(x));
      if (simple) return '[\n' + v.map(x => pad1 + inline(x)).join(',\n') + '\n' + pad + ']';
      return '[\n' + v.map(x => pad1 + block(x, ind + 1)).join(',\n') + '\n' + pad + ']';
    }
    if (isObj(v)) {
      const e = Object.entries(v);
      if (!e.length) return '{}';
      return '{\n' + e.map(([k, x]) => pad1 + JSON.stringify(k) + ': ' + block(x, ind + 1, k)).join(',\n') + '\n' + pad + '}';
    }
    return JSON.stringify(v);
  };
  return block(o, 0) + '\n';
}
const fileName = L => (L.kind === 'lair' ? 'lair' : 'cave') + '_' + L.number + '.json';

// ------------------------------------------------------------------ state
const S = {
  level: newLevel('cave', 1),
  slot: 'challenge',
  mode: 'paint',
  brush: '#',
  zoom: 2,
  grid: true,
  features: true,
  mage: { show: false, x: 8, y: 10 },
  sel: null,             // {list: 'enemies'|'objects'|'mage', i}
  tplEnemy: { kind: 'slime', element: null, hp: null, still: false },
  tplObject: { type: 'chest', lit: false, pulled: false, contents: 'gold:50', hidden: false, element: 'ice' },
  undo: [], redo: [],
  dirty: false,
  savedName: null,        // name of the file this came from / was saved as
  dirHandle: null,
  issues: null,           // last Check result
  flash: null,            // {slot, x, y, w, h, until}
  hover: null,
  drag: null,
};

// ------------------------------------------------------------------ assets
const A = window.EL_ASSETS || { themes: {}, enemies: {}, objects: {}, items: {}, mage: null };
const IMG = {};
let pendingImages = 0;
function img(key, src) {
  if (!src) return null;
  if (IMG[key]) return IMG[key];
  const im = new Image();
  pendingImages++;
  im.onload = im.onerror = () => { if (--pendingImages === 0) { renderAll(); } };
  im.src = src;
  IMG[key] = im;
  return im;
}
const ready = im => im && im.complete && im.naturalWidth > 0;
function preload() {
  for (const [i, t] of Object.entries(A.themes || {})) { img('wall' + i, t.wall); img('water' + i, t.water); }
  for (const [k, els] of Object.entries(A.enemies || {})) for (const [e, f] of Object.entries(els)) img('en_' + k + '_' + e, f.src);
  for (const [k, f] of Object.entries(A.objects || {})) img('ob_' + k, f.src);
  for (const [k, f] of Object.entries(A.items || {})) img('it_' + k, f.src);
  if (A.mage) img('mage', A.mage.src);
  for (const [k, f] of Object.entries(A.tiles || {})) img('tile_' + k, f.src);
  for (const [k, f] of Object.entries(A.relics || {})) img('re_' + k, f.src);
  for (const [k, f] of Object.entries(A.props || {})) img('pr_' + k, f.src);
}
function enemySprite(kind, element) {
  const els = (A.enemies || {})[kind];
  if (!els) return null;
  let el = element || (USUAL[kind] ? USUAL[kind][0] : 'neutral');
  if (el === 'region') el = 'neutral';
  const key = els[el] ? el : (els.any ? 'any' : (els.neutral ? 'neutral' : Object.keys(els)[0]));
  const f = els[key];
  return f ? { im: IMG['en_' + kind + '_' + key], w: f.w, h: f.h } : null;
}
function objSprite(o) {
  if (isF2()) { const s = objSprite2(o); if (s || o.type === 'prop') return s; }
  let k = o.type;
  if (o.type === 'lever') k = o.pulled === true ? 'lever_on' : 'lever_off';
  const f = (A.objects || {})[k];
  return f ? { im: IMG['ob_' + k], w: f.w, h: f.h } : null;
}
const CONTENT_ICON = { gold: 'gold_pile', key: 'key', heart: 'heart_container', mana: 'mana_potion', elixir: 'golden_apple', elixirs: 'golden_apple', potion: 'mana_potion', page: 'gem' };

// ------------------------------------------------------------------ helpers
const $ = id => document.getElementById(id);
const el = (tag, attrs, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (k === 'class') e.className = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (v !== undefined && v !== null && v !== false) e.setAttribute(k, v === true ? '' : v);
  }
  for (const k of kids) if (k !== null && k !== undefined) e.append(k);
  return e;
};
const room = () => S.level.rooms[S.slot] || null;
const tileAt = (R, x, y) => (R.tiles[y] || '')[x];
const sizeOk = R => isF2() ? sizeOk2(R) : R.tiles.length === ROWS && R.tiles.every(r => r.length === COLS);
/** Pad / trim to 16 x 13: missing cells become floor inside a wall ring. */
function normalizeRoom(R) {
  if (isF2()) { normalize2(R); return; }
  const blank = blankRoom().tiles;
  const t = [];
  for (let y = 0; y < ROWS; y++) {
    const row = R.tiles[y] || '';
    t.push((row + blank[y].slice(row.length)).slice(0, COLS));
  }
  R.tiles = t;
}
function setTile(R, x, y, ch) {
  if (!sizeOk(R)) normalizeRoom(R);
  const row = R.tiles[y];
  R.tiles[y] = row.slice(0, x) + ch + row.slice(x + 1);
}
function toast(msg, err) {
  const t = $('toast');
  t.textContent = msg; t.className = err ? 'err' : ''; t.hidden = false;
  clearTimeout(toast.t); toast.t = setTimeout(() => { t.hidden = true; }, err ? 5000 : 2500);
}

// ------------------------------------------------------------------ undo
const snap = () => JSON.stringify({ level: S.level, slot: S.slot });
function pushUndo(s) {
  S.undo.push(s || snap());
  if (S.undo.length > 300) S.undo.shift();
  S.redo = [];
}
function restore(s) {
  const o = JSON.parse(s);
  S.level = o.level; S.slot = o.slot; S.sel = null; S.doorForm = null;
}
function doUndo() {
  if (!S.undo.length) return;
  S.redo.push(snap()); restore(S.undo.pop()); markDirty(); renderAll();
}
function doRedo() {
  if (!S.redo.length) return;
  S.undo.push(snap()); restore(S.redo.pop()); markDirty(); renderAll();
}
/** Run a model change as one undo step. */
function commit(fn) {
  const before = snap();
  fn();
  if (snap() !== before) { pushUndo(before); markDirty(); }
  renderAll();
}
function markDirty() { S.dirty = true; S.issues = null; renderFileState(); }

// ------------------------------------------------------------------ rendering: room canvas
const cv = $('grid');
const mainCtx = cv.getContext('2d');
let ctx = mainCtx; // the tile painters draw on this; swatches swap in their own context

/** Tiles as the game will build them (door gaps, locks, seals, stairs applied). */
function effectiveTiles(R) {
  const t = [];
  const [W, H] = dims();
  for (let y = 0; y < H; y++) {
    const row = [];
    for (let x = 0; x < W; x++) {
      const c = tileAt(R, x, y);
      row.push(c === undefined ? '?' : c);
    }
    t.push(row);
  }
  if (S.features && isF2()) applyFeatures2(t, R, S.slot);
  else if (S.features) {
    for (const g of gapsFor(S.level, S.slot)) {
      const ch = { door: '.', lock: 'L', seal: 'S', crack: 'c' }[g.kind];
      for (const [x, y] of gapTiles(g.side)) t[y][x] = ch;
    }
    if (S.level.kind === 'lair' && S.slot === 'stairs') for (const [x, y] of STAIRS) t[y][x] = 'B';
  }
  return t;
}
const wallish = c => c === '#' || c === 'c' || c === 'L' || c === 'G';
const wet = c => c === '~' || c === 'i';

function drawRoom() {
  const R = room();
  const z = S.zoom;
  const [COLS, ROWS] = dims(); // format 2 rooms can be 2 screens wide / tall
  cv.width = COLS * TS * z; cv.height = ROWS * TS * z;
  cv.style.width = cv.width + 'px'; cv.style.height = cv.height + 'px';
  ctx.setTransform(z, 0, 0, z, 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.fillStyle = '#101216'; ctx.fillRect(0, 0, COLS * TS, ROWS * TS);
  if (!R) return;
  const theme = S.level.theme !== null && isInt(S.level.theme) ? S.level.theme : defaultTheme(S.level);
  const wallIm = IMG['wall' + theme], waterIm = IMG['water' + theme];
  const t = effectiveTiles(R);
  const at = (x, y) => (x < 0 || y < 0 || x >= COLS || y >= ROWS) ? '#' : t[y][x];
  const h = TS / 2;
  // Dual grid (world.rs render_hd): a display tile on every cell corner.
  if (ready(wallIm)) {
    for (let j = 0; j <= ROWS; j++) for (let i = 0; i <= COLS; i++) {
      const cs = [at(i - 1, j - 1), at(i, j - 1), at(i - 1, j), at(i, j)];
      const bits = f => cs.reduce((a, v) => (a << 1) | (f(v) ? 1 : 0), 0);
      const wb = bits(wallish);
      let src = wallIm, k = 0;
      if (wb !== 0) k = wb;
      else if (ready(waterIm) && cs.some(wet)) { src = waterIm; k = bits(v => !wet(v)); }
      ctx.drawImage(src, (k % 4) * TS, (k >> 2) * TS, TS, TS, i * TS - h, j * TS - h, TS, TS);
    }
  } else {
    for (let y = 0; y < ROWS; y++) for (let x = 0; x < COLS; x++) {
      const c = t[y][x];
      ctx.fillStyle = wallish(c) ? '#5b5f6b' : '#2c3a2e';
      ctx.fillRect(x * TS, y * TS, TS, TS);
    }
  }
  // Special tiles.
  for (let y = 0; y < ROWS; y++) for (let x = 0; x < COLS; x++) {
    const c = t[y][x], px = x * TS, py = y * TS;
    if (c === '~' && !ready(waterIm)) { ctx.fillStyle = '#2f6fbf'; ctx.fillRect(px, py, TS, TS); }
    else if (c === 'i') drawIce(px, py);
    else if (c === 'c') drawCrack(px, py);
    else if (c === 'o') drawPlate(px, py);
    else if (c === 'D') drawDecor(px, py);
    else if (c === 'L') drawLock(px, py);
    else if (c === 'S') drawSeal(px, py);
    else if (c === 'B') drawBarrier(px, py);
    else if (isF2() && drawSpecial2(c, px, py)) { /* pit, lava, pegs, hidden bridge, thorns, big lock */ }
    else if (!tileChars().includes(c)) { ctx.fillStyle = '#d02050'; ctx.fillRect(px + 2, py + 2, TS - 4, TS - 4); label(c === '?' ? '?' : c, px + h, py + h, '#fff'); }
  }
  // Grid.
  if (S.grid) {
    ctx.strokeStyle = 'rgba(255,255,255,0.10)'; ctx.lineWidth = 1 / z;
    ctx.beginPath();
    for (let x = 1; x < COLS; x++) { ctx.moveTo(x * TS, 0); ctx.lineTo(x * TS, ROWS * TS); }
    for (let y = 1; y < ROWS; y++) { ctx.moveTo(0, y * TS); ctx.lineTo(COLS * TS, y * TS); }
    ctx.stroke();
  }
  // Game-feature markers.
  if (S.features && isF2()) drawFeatureMarkers2(R, S.slot);
  else if (S.features) drawFeatureMarkers();
  // Objects then enemies (enemies stand on top).
  R.objects.forEach((o, i) => drawObject(o, S.sel && S.sel.list === 'objects' && S.sel.i === i));
  R.enemies.forEach((e, i) => drawEnemy(e, S.sel && S.sel.list === 'enemies' && S.sel.i === i));
  if (S.mage.show) drawMage();
  // Rectangle preview.
  if (S.drag && S.drag.type === 'rect' && S.drag.cur) {
    const [x0, y0, x1, y1] = rectOf(S.drag.start, S.drag.cur);
    ctx.fillStyle = 'rgba(95,179,255,0.25)'; ctx.fillRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
    ctx.strokeStyle = '#5fb3ff'; ctx.lineWidth = 1.5 / z; ctx.strokeRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
  }
  // Hover.
  if (S.hover && !S.drag) {
    ctx.strokeStyle = 'rgba(255,255,255,0.7)'; ctx.lineWidth = 1 / z;
    ctx.strokeRect(S.hover.x * TS + 0.5 / z, S.hover.y * TS + 0.5 / z, TS - 1 / z, TS - 1 / z);
  }
  // Check highlight.
  if (S.flash && S.flash.slot === S.slot && Date.now() < S.flash.until) {
    const f = S.flash, on = Math.floor(Date.now() / 250) % 2 === 0;
    ctx.strokeStyle = on ? '#ff6b6b' : '#ffe066'; ctx.lineWidth = 3 / z;
    ctx.strokeRect(f.x * TS, f.y * TS, f.w * TS, f.h * TS);
  }
}
function label(s, x, y, col, size) {
  ctx.font = 'bold ' + (size || 9) + 'px Consolas, monospace';
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  ctx.fillStyle = 'rgba(0,0,0,0.75)'; ctx.fillText(s, x + 0.7, y + 0.7);
  ctx.fillStyle = col; ctx.fillText(s, x, y);
}
function drawIce(px, py) {
  ctx.fillStyle = 'rgba(190,235,255,0.72)'; ctx.fillRect(px, py, TS, TS);
  ctx.strokeStyle = 'rgba(255,255,255,0.9)'; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(px + 4, py + 15); ctx.lineTo(px + 11, py + 8); ctx.moveTo(px + 12, py + 20); ctx.lineTo(px + 20, py + 12); ctx.stroke();
  ctx.strokeStyle = 'rgba(120,190,230,0.9)'; ctx.strokeRect(px + 0.5, py + 0.5, TS - 1, TS - 1);
}
function drawCrack(px, py) {
  ctx.fillStyle = 'rgba(40,20,10,0.25)'; ctx.fillRect(px, py, TS, TS);
  ctx.strokeStyle = '#140c08'; ctx.lineWidth = 1.6;
  ctx.beginPath();
  ctx.moveTo(px + 5, py + 3); ctx.lineTo(px + 10, py + 9); ctx.lineTo(px + 8, py + 14); ctx.lineTo(px + 13, py + 20);
  ctx.moveTo(px + 10, py + 9); ctx.lineTo(px + 17, py + 7); ctx.lineTo(px + 20, py + 12);
  ctx.moveTo(px + 8, py + 14); ctx.lineTo(px + 3, py + 17);
  ctx.stroke();
  ctx.strokeStyle = 'rgba(255,190,120,0.55)'; ctx.lineWidth = 1; ctx.strokeRect(px + 1, py + 1, TS - 2, TS - 2);
}
function drawPlate(px, py) {
  ctx.fillStyle = '#3d3f46'; ctx.fillRect(px + 3, py + 3, 18, 18);
  ctx.fillStyle = '#8e919c'; ctx.fillRect(px + 4, py + 4, 16, 16);
  ctx.fillStyle = '#b8bbc6'; ctx.fillRect(px + 4, py + 4, 16, 2);
  ctx.fillStyle = '#6b6e78'; ctx.fillRect(px + 7, py + 8, 10, 8);
  ctx.fillStyle = '#e8c24e'; ctx.fillRect(px + 11, py + 11, 2, 2);
}
function drawDecor(px, py) {
  ctx.fillStyle = 'rgba(150,95,40,0.35)'; ctx.fillRect(px, py, TS, TS);
  ctx.save(); ctx.beginPath(); ctx.rect(px, py, TS, TS); ctx.clip();
  ctx.strokeStyle = 'rgba(230,170,90,0.7)'; ctx.lineWidth = 1;
  ctx.beginPath();
  for (let d = -TS; d < TS; d += 6) { ctx.moveTo(px + d, py + TS); ctx.lineTo(px + d + TS, py); }
  ctx.stroke(); ctx.restore();
  ctx.strokeStyle = 'rgba(230,170,90,0.9)'; ctx.strokeRect(px + 0.5, py + 0.5, TS - 1, TS - 1);
}
function drawLock(px, py) {
  ctx.fillStyle = '#6b4020'; ctx.fillRect(px + 1, py + 1, TS - 2, TS - 2);
  ctx.fillStyle = '#8a5a30'; ctx.fillRect(px + 3, py + 3, TS - 6, TS - 6);
  ctx.fillStyle = '#e8c24e'; ctx.fillRect(px + 9, py + 7, 6, 6);
  ctx.fillStyle = '#1a1008'; ctx.fillRect(px + 11, py + 9, 2, 5);
}
function drawSeal(px, py) {
  ctx.fillStyle = 'rgba(20,20,26,0.55)'; ctx.fillRect(px, py, TS, TS);
  ctx.fillStyle = '#9aa0ad';
  for (let x = 3; x < TS; x += 6) ctx.fillRect(px + x, py + 1, 2, TS - 2);
  ctx.fillRect(px, py + 5, TS, 2); ctx.fillRect(px, py + 16, TS, 2);
}
function drawBarrier(px, py) {
  ctx.fillStyle = 'rgba(40,20,60,0.8)'; ctx.fillRect(px, py, TS, TS);
  ctx.fillStyle = '#5a4a70';
  for (let y = 3; y < TS; y += 5) ctx.fillRect(px + 2, py + y, TS - 4, 3);
  ctx.fillStyle = 'rgba(190,120,255,0.35)'; ctx.fillRect(px, py, TS, TS);
}
function drawFeatureMarkers() {
  const z = S.zoom;
  const col = { door: ['rgba(80,220,120,0.22)', '#50dc78'], lock: ['rgba(255,180,60,0.22)', '#ffb43c'],
    seal: ['rgba(255,180,60,0.22)', '#ffb43c'], crack: ['rgba(255,180,60,0.22)', '#ffb43c'] };
  for (const g of gapsFor(S.level, S.slot)) {
    const ts = gapTiles(g.side);
    const xs = ts.map(t => t[0]), ys = ts.map(t => t[1]);
    const x0 = Math.min(...xs), y0 = Math.min(...ys), x1 = Math.max(...xs), y1 = Math.max(...ys);
    const [fill, line] = col[g.kind];
    ctx.fillStyle = fill; ctx.fillRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
    ctx.strokeStyle = line; ctx.lineWidth = 1.5 / z; ctx.setLineDash([3 / z * 2, 2 / z * 2]);
    ctx.strokeRect(x0 * TS, y0 * TS, (x1 - x0 + 1) * TS, (y1 - y0 + 1) * TS);
    ctx.setLineDash([]);
    const cx = (x0 + x1 + 1) * TS / 2, cy = (y0 + y1 + 1) * TS / 2;
    const off = { n: [0, TS * 0.95], s: [0, -TS * 0.95], e: [-TS * 1.6, 0], w: [TS * 1.6, 0] }[g.side];
    if (z >= 2) label(g.label, cx + off[0], cy + off[1], line, 7);
  }
  if (S.level.kind === 'lair' && S.slot === 'stairs') {
    ctx.strokeStyle = '#c490ff'; ctx.lineWidth = 1.5 / z; ctx.setLineDash([4 / z, 3 / z]);
    ctx.strokeRect(7 * TS, 3 * TS, 2 * TS, 2 * TS); ctx.setLineDash([]);
    if (z >= 2) label('boss stairs', 8 * TS, 5 * TS + 6, '#c490ff', 7);
  }
}
/** Where a sprite of height h sits on tile (x, y): centred, feet near the tile bottom. */
function spritePos(x, y, w, h) {
  const cx = x * TS + TS / 2, cy = y * TS + TS / 2;
  return [Math.round(cx - w / 2), Math.round(h > TS ? cy + TS / 2 - h + 2 : cy - h / 2)];
}
function drawEnemy(e, selected) {
  const ok = isInt(e.x) && isInt(e.y);
  if (!ok) return;
  const sp = KINDS.includes(e.kind) ? enemySprite(e.kind, ELEMENTS.includes(e.element) ? e.element : null) : null;
  if (selected) selBox(e.x, e.y);
  if (sp && ready(sp.im)) {
    const [px, py] = spritePos(e.x, e.y, sp.w, sp.h);
    ctx.drawImage(sp.im, px, py);
  } else {
    const cx = e.x * TS + TS / 2, cy = e.y * TS + TS / 2;
    ctx.fillStyle = KINDS.includes(e.kind) ? '#d04040' : '#ff00aa';
    ctx.beginPath(); ctx.arc(cx, cy, 8, 0, Math.PI * 2); ctx.fill();
    label(String(e.kind || '?').slice(0, 2), cx, cy, '#fff');
  }
  // Element pip and badges.
  const pip = { fire: '#ff6a3a', ice: '#7fd8ff', storm: '#ffe45c', earth: '#9bd05a', neutral: '#c8c8c8' }[e.element];
  if (pip) { ctx.fillStyle = '#000'; ctx.fillRect(e.x * TS + 1, e.y * TS + 1, 6, 6); ctx.fillStyle = pip; ctx.fillRect(e.x * TS + 2, e.y * TS + 2, 4, 4); }
  if (e.still === true) label('●', e.x * TS + TS - 4, e.y * TS + 4, '#ffe066', 7);
}
function drawObject(o, selected) {
  if (!isInt(o.x) || !isInt(o.y)) return;
  if (selected) selBox(o.x, o.y);
  const px = o.x * TS, py = o.y * TS, cx = px + TS / 2, cy = py + TS / 2;
  const sp = objSprite(o);
  ctx.save();
  if (o.type === 'chest' && o.hidden === true) ctx.globalAlpha = 0.5;
  if (o.type === 'torch' && o.lit === true) {
    const g = ctx.createRadialGradient(cx, cy - 6, 1, cx, cy - 6, 16);
    g.addColorStop(0, 'rgba(255,200,80,0.8)'); g.addColorStop(1, 'rgba(255,120,20,0)');
    ctx.fillStyle = g; ctx.fillRect(px - 8, py - 14, TS + 16, TS + 16);
  }
  if (sp && ready(sp.im)) {
    // Format 2 art is anchored like the game (keepdraw.rs): centred, bottom on the tile bottom.
    const [sx, sy] = sp.anchor ? [Math.round(cx - sp.w / 2), (o.y + 1) * TS - sp.h + 2] : spritePos(o.x, o.y, sp.w, sp.h);
    if (o.type === 'prop' && o.solid === false) ctx.globalAlpha = 0.85;
    ctx.drawImage(sp.im, sx, sy);
  } else {
    const c = { torch: '#b0602a', block: '#8a8f9a', lever: '#a08050', chest: '#b07830', shrine: '#8080c0', ...(isF2() ? F2_FALLBACK : {}) }[o.type] || '#ff00aa';
    ctx.fillStyle = c; ctx.fillRect(px + 4, py + 4, TS - 8, TS - 8);
    label(String(o.type || '?').slice(0, 2), cx, cy, '#fff');
  }
  if (o.type === 'torch' && o.lit === true) {
    ctx.fillStyle = '#ffb030'; ctx.beginPath(); ctx.ellipse(cx, py + 3, 3, 5, 0, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = '#fff2a0'; ctx.beginPath(); ctx.ellipse(cx, py + 5, 1.5, 2.5, 0, 0, Math.PI * 2); ctx.fill();
  }
  if (o.type === 'shrine') {
    const c = { fire: '#ff6a3a', ice: '#7fd8ff', storm: '#ffe45c', earth: '#9bd05a' }[o.element] || '#ff00aa';
    ctx.fillStyle = c; ctx.beginPath(); ctx.arc(cx, py + 2, 5, 0, Math.PI * 2); ctx.fill();
    ctx.fillStyle = 'rgba(255,255,255,0.8)'; ctx.fillRect(cx - 2, py - 1, 2, 2);
  }
  ctx.restore();
  if (isF2()) drawObjExtras2(o, px, py);
  if (o.type === 'chest') {
    const kind = typeof o.contents === 'string' ? o.contents.split(':')[0] : '';
    const ic = isF2() && F2.CONTENT_ICON[kind] ? IMG['re_' + F2.CONTENT_ICON[kind]] : IMG['it_' + CONTENT_ICON[kind]];
    if (ready(ic)) ctx.drawImage(ic, px + TS - 10, py - 6, 12, 12);
    else if (kind) label(kind.slice(0, 2), px + TS - 4, py, '#ffe066', 7);
    if (o.hidden === true) label('H', px + 4, py + 4, '#9fe0ff', 8);
  }
}
function drawMage() {
  const m = S.mage;
  if (A.mage && ready(IMG.mage)) {
    const [px, py] = spritePos(m.x, m.y, A.mage.w, A.mage.h);
    ctx.globalAlpha = 0.92; ctx.drawImage(IMG.mage, px, py); ctx.globalAlpha = 1;
  } else {
    ctx.fillStyle = '#d04828'; ctx.fillRect(m.x * TS + 6, m.y * TS - 8, 12, 30);
  }
  if (S.sel && S.sel.list === 'mage') selBox(m.x, m.y);
}
function selBox(x, y) {
  ctx.strokeStyle = '#ffe066'; ctx.lineWidth = 2 / S.zoom;
  ctx.strokeRect(x * TS + 1 / S.zoom, y * TS + 1 / S.zoom, TS - 2 / S.zoom, TS - 2 / S.zoom);
}
setInterval(() => { if (S.flash && (Date.now() < S.flash.until + 300)) { drawRoom(); if (isF2()) drawMap(); } }, 250);

// ------------------------------------------------------------------ rendering: panels
function renderAll() {
  document.body.classList.toggle('f2', isF2());
  renderHeader(); renderTabs(); renderRoomPanel(); renderRoom2(); renderLairBar(); renderTools(); renderPlaceForm(); renderSelForm();
  renderCheck(); renderFileState(); drawRoom(); drawMap(); renderLegend();
  $('btnUndo').disabled = !S.undo.length; $('btnRedo').disabled = !S.redo.length;
}
function renderFileState() {
  const n = fileName(S.level);
  $('fileState').innerHTML = '';
  $('fileState').append(n, S.dirty ? el('span', { class: 'dirty' }, ' • unsaved') : '',
    S.dirHandle ? el('span', {}, '  → ' + S.dirHandle.name + '/') : '');
  document.title = (S.dirty ? '* ' : '') + n + ' - Elemental Legends Level Editor';
}
function renderHeader() {
  const L = S.level;
  $('fKind').value = L.kind === 'lair' ? 'lair' : 'cave';
  $('fKind').disabled = isF2();
  $('fKind').title = isF2() ? 'Format 2 is for lairs only (use New for a cave)' : '';
  $('fFormat').textContent = isF2() ? '2 - hand-designed lair (free room layout)' : '1 - fixed room slots';
  $('rowRelic').hidden = !isF2();
  if (isF2()) {
    const rs = $('fRelic');
    if (!rs.options.length) {
      rs.append(el('option', { value: '' }, '(none - Dark Tower)'));
      F2.RELICS.forEach(r => rs.append(el('option', { value: r }, F2.RELIC_NAMES[r])));
    }
    for (const o of [...rs.options]) if (o.dataset.bad) o.remove();
    if (L.relic !== null && !F2.RELICS.includes(L.relic)) { const o = el('option', { value: String(L.relic) }, String(L.relic) + ' (unknown)'); o.dataset.bad = '1'; rs.append(o); }
    rs.value = L.relic === null ? '' : String(L.relic);
  }
  $('fNumber').value = isInt(L.number) ? L.number : '';
  $('fNumber').max = L.kind === 'lair' ? 6 : 8;
  if (document.activeElement !== $('fName')) $('fName').value = typeof L.name === 'string' ? L.name : '';
  const ts = $('fTheme');
  if (!ts.options.length) {
    ts.append(el('option', { value: '' }, '(default)'));
    THEMES.forEach((n, i) => ts.append(el('option', { value: String(i) }, i + ' ' + n + ((A.themes || {})[i] ? '' : ' (no art)'))));
  }
  ts.value = L.theme === null ? '' : String(L.theme);
  const d = defaultTheme(L);
  $('themeNote').textContent = L.theme === null
    ? (L.kind === 'lair' ? 'Default: the lair\'s own set (' + d + ' ' + THEMES[d] + ').'
      : 'Default: the rocky cave set (' + d + ' ' + THEMES[d] + ').')
    : '';
}
function renderTabs() {
  const tabs = $('tabs');
  tabs.innerHTML = '';
  tabs.hidden = isF2();
  if (isF2()) return; // format 2: the room grid map / room list pick the room
  const slots = slotsFor(S.level);
  for (const k of Object.keys(S.level.rooms)) if (!slots.includes(k)) slots.push(k);
  if (!slots.includes(S.slot)) S.slot = slots[0];
  const bad = new Set((S.issues || []).filter(i => i.sev === 'error').map(i => i.slot));
  for (const s of slots) {
    const def = !!S.level.rooms[s];
    const known = slotsFor(S.level).includes(s);
    tabs.append(el('button', {
      class: s === S.slot ? 'active' : '', title: def ? 'Defined' : (s === 'mouth' ? 'Not in this cave' : 'Generated layout'),
      onclick: () => { S.slot = s; S.sel = null; renderAll(); },
    }, el('span', { class: 'dot' + (bad.has(s) ? ' bad' : def ? ' def' : '') }), s + (known ? '' : ' (unknown)')));
  }
}
function renderRoomPanel() {
  if (isF2()) { $('undefinedPane').hidden = true; return; } // renderRoom2 (lair.js)
  const R = room();
  $('roomSlotName').textContent = S.slot;
  $('roomFields').hidden = !R;
  $('undefinedPane').hidden = !!R;
  const L = S.level;
  const info = slotInfo(L, S.slot);
  $('roomInfo').textContent = info;
  if (!R) {
    $('undefinedText').textContent = S.slot === 'mouth' && L.kind !== 'lair'
      ? 'No mouth room: this is a two-room cave and the challenge room is the entrance. Define the mouth to make it a three-room cave.'
      : 'This room is not defined, so the game uses its built-in generated layout. Define it to start from an empty walled 16 x 13 room.';
    $('btnDefine').textContent = S.slot === 'mouth' && L.kind !== 'lair' ? 'Add mouth room' : 'Define room';
    return;
  }
  const ps = $('fPuzzle');
  ps.innerHTML = '';
  const allowed = puzzlesFor(L.kind, S.slot);
  const required = L.kind !== 'lair' && S.slot === 'challenge';
  if (!required) ps.append(el('option', { value: '' }, '(default)'));
  for (const p of allowed) ps.append(el('option', { value: p }, p));
  if (R.puzzle !== null && !allowed.includes(R.puzzle)) ps.append(el('option', { value: String(R.puzzle) }, String(R.puzzle) + ' (not allowed here)'));
  if (required && R.puzzle === null) ps.append(el('option', { value: '' }, '(missing!)'));
  ps.value = R.puzzle === null ? '' : String(R.puzzle);
  const showCombat = (L.kind === 'lair' && S.slot === 'hub') || R.combat !== false;
  $('rowCombat').hidden = !showCombat;
  $('fCombat').checked = R.combat === true;
  $('fRandomOn').checked = R.random_enemies !== null;
  $('fRandom').disabled = R.random_enemies === null;
  $('fRandom').value = R.random_enemies === null ? '' : R.random_enemies;
  $('randomNote').textContent = R.random_enemies === null
    ? (R.enemies.length ? 'Default: 0 extra (enemies are listed).' : 'Default: the usual number of random monsters.')
    : 'Extra region monsters at random free spots.';
  $('sizeWarn').hidden = sizeOk(R);
  $('btnUndefine').textContent = S.slot === 'mouth' && L.kind !== 'lair' ? 'Remove mouth (two-room cave)' : 'Remove room (use generated)';
}
function slotInfo(L, s) {
  const lair = L.kind === 'lair';
  const t = {
    mouth: 'Bottom room. Exit south to the overworld.',
    challenge: 'Must be beaten to open the north door to the treasure. Needs a puzzle.',
    treasure: 'Top room. Should hold a chest; its contents is the cave\'s reward.',
    entry: 'Bottom centre. Exit south.' + (L.number >= 2 ? ' The east wall hides the pantry behind a cracked stretch.' : ''),
    hub: 'Centre. North door locked until the key is found.',
    west: 'Its puzzle reveals or unlocks the key chest (chest with contents "key").',
    east: 'Solving its puzzle breaks the seal on the stairs.',
    stairs: 'Top. Boss staircase at columns 7-8, rows 3-4.',
    feast: 'Lower left, lair 1 only. Food room.',
    pantry: 'Lower right, lairs 2-6. Hidden food room.',
  };
  if (!lair && ['entry', 'hub', 'west', 'east', 'stairs', 'feast', 'pantry'].includes(s)) return 'Not a cave slot.';
  if (lair && ['mouth', 'challenge', 'treasure'].includes(s)) return 'Not a lair slot.';
  return t[s] || 'Unknown slot: the game ignores it.';
}
function renderTools() {
  document.querySelectorAll('#modeButtons button').forEach(b => b.classList.toggle('on', b.dataset.mode === S.mode));
  const bb = $('brushButtons');
  const fmt = isF2() ? '2' : '1';
  if (bb.dataset.fmt !== fmt) { bb.innerHTML = ''; bb.dataset.fmt = fmt; if (!brushes().some(b => b.ch === S.brush)) S.brush = '#'; }
  if (!bb.childElementCount) {
    for (const b of brushes()) {
      const c = el('canvas', { width: 24, height: 24 });
      bb.append(el('button', { 'data-brush': b.ch, title: b.name + ' (' + b.key + ')', onclick: () => { setBrush(b.ch); } },
        c, b.name, el('span', { class: 'key' }, b.key + '  ' + b.ch)));
    }
  }
  const theme = S.level.theme !== null && isInt(S.level.theme) ? S.level.theme : defaultTheme(S.level);
  bb.querySelectorAll('button').forEach(btn => {
    const ch = btn.dataset.brush;
    btn.classList.toggle('on', ch === S.brush && ['paint', 'rect', 'fill', 'pick'].includes(S.mode));
    drawSwatch(btn.querySelector('canvas'), ch, theme);
  });
  document.querySelectorAll('.zoom').forEach(b => b.classList.toggle('on', +b.dataset.zoom === S.zoom));
  $('tGrid').checked = S.grid; $('tFeatures').checked = S.features; $('tMage').checked = S.mage.show;
}
function drawSwatch(c, ch, theme) {
  const g = c.getContext('2d');
  g.imageSmoothingEnabled = false;
  g.clearRect(0, 0, 24, 24);
  const wall = IMG['wall' + theme], water = IMG['water' + theme];
  if (ch === '#' || ch === 'c') { if (ready(wall)) g.drawImage(wall, 3 * TS, 3 * TS, TS, TS, 0, 0, TS, TS); else { g.fillStyle = '#5b5f6b'; g.fillRect(0, 0, 24, 24); } }
  else if (ch === '~' || ch === 'i') { if (ready(water)) g.drawImage(water, 0, 0, TS, TS, 0, 0, TS, TS); else { g.fillStyle = '#2f6fbf'; g.fillRect(0, 0, 24, 24); } }
  else { if (ready(wall)) g.drawImage(wall, 0, 0, TS, TS, 0, 0, TS, TS); else { g.fillStyle = '#2c3a2e'; g.fillRect(0, 0, 24, 24); } }
  // Reuse the overlay painters on the swatch.
  ctx = g;
  try {
    if (ch === 'i') drawIce(0, 0); if (ch === 'c') drawCrack(0, 0); if (ch === 'o') drawPlate(0, 0); if (ch === 'D') drawDecor(0, 0);
    if (F2.TILE_NAMES[ch]) drawSpecial2(ch, 0, 0);
  } finally { ctx = mainCtx; }
}

// Property forms for enemies / objects (templates for placing, or the selection).
function selectRow(labelText, value, options, onchange) {
  const s = el('select', { onchange: e => onchange(e.target.value) });
  for (const [v, t] of options) s.append(el('option', { value: v }, t));
  if (![...s.options].some(o => o.value === value)) s.append(el('option', { value }, value + ' (invalid)'));
  s.value = value;
  return el('div', { class: 'row' }, el('label', {}, labelText), s);
}
function checkRow(labelText, text, checked, onchange) {
  return el('div', { class: 'row' }, el('label', {}, labelText),
    el('label', { class: 'inline' }, el('input', { type: 'checkbox', checked: checked, onchange: e => onchange(e.target.checked) }), text));
}
function enemyForm(e, apply) {
  const f = el('div', { class: 'form' });
  const pv = el('canvas', { width: 40, height: 40 });
  f.append(el('div', { class: 'preview' }, pv, el('span', { class: 'note' },
    KINDS.includes(e.kind) ? 'usual: ' + USUAL[e.kind][0] + ', ' + USUAL[e.kind][1] + ' HP (+region)' : '')));
  paintPreview(pv, enemySprite(e.kind, ELEMENTS.includes(e.element) ? e.element : null));
  f.append(selectRow('Kind', String(e.kind), KINDS.map(k => [k, k]), v => apply(x => { x.kind = v; })));
  f.append(selectRow('Element', e.element === null ? '' : String(e.element),
    [['', '(usual' + (USUAL[e.kind] ? ': ' + USUAL[e.kind][0] : '') + ')'], ...ELEMENTS.map(k => [k, k])],
    v => apply(x => { x.element = v === '' ? null : v; })));
  const hp = el('input', { type: 'number', min: 1, step: 1, placeholder: USUAL[e.kind] ? 'usual' : '', value: e.hp === null ? '' : e.hp,
    onchange: ev => apply(x => { const v = ev.target.value.trim(); x.hp = v === '' ? null : Number(v); }) });
  f.append(el('div', { class: 'row' }, el('label', {}, 'HP'), hp, el('span', { class: 'note' }, 'blank = usual')));
  f.append(checkRow('Still', 'pinned in place', e.still === true, v => apply(x => { x.still = v; })));
  return f;
}
function contentsParts(c) {
  if (typeof c !== 'string') return ['', ''];
  const m = /^([a-z]+):(\d+)$/.exec(c);
  if (m && CONTENT_COUNTED.includes(m[1])) return [m[1], m[2]];
  if (CONTENT_SIMPLE.includes(c)) return [c, ''];
  return ['custom', ''];
}
function objectForm(o, apply, isTpl) {
  const f = el('div', { class: 'form' });
  const pv = el('canvas', { width: 40, height: 40 });
  const f2 = isF2();
  f.append(el('div', { class: 'preview' }, pv, el('span', { class: 'note' }, {
    torch: 'Brazier; fire bolts light it. Solid.', block: 'Push block (lair plates puzzles). Solid.', lever: f2 ? 'Pulled by walking into it or with the whip; sets a flag.' : 'Pull by walking into it. Solid.',
    chest: f2 ? 'Chest. Hidden chests appear when the room\'s shutter is solved.' : 'Chest. In caves the treasure chest also gives the page.', shrine: 'Orb that switches the mage\'s element.',
    big_chest: 'Great chest: needs the big key, holds the relic.', ice_block: 'Slides until it hits something or reaches a plate.',
    floor_switch: 'Pressed by the mage or a pushed block.', crystal_switch: 'A bolt or the whip swaps the orange / blue barriers.',
    crystal: 'Lit by a bolt of its element; ordered ones must be lit 1, 2, 3...', whip_post: 'The Vine Whip pulls you to it (6 tiles).',
    boulder: 'Heaved aside with the Titan Gloves.', pot: 'Bolts smash it; sometimes a heart, magic or coins.', crate: 'Bolts smash it; sometimes a heart, magic or coins.',
    tablet: 'Lore / hints, read by walking up to it.', prop: 'Decoration from the art sheets.' }[o.type] || '')));
  paintPreview(pv, objSprite(o));
  f.append(selectRow('Type', String(o.type), (f2 ? F2.OBJ_TYPES : OBJ_TYPES).map(k => [k, k]), v => apply(x => {
    x.type = v;
    if (v === 'torch' && x.lit === undefined) x.lit = false;
    if (v === 'lever' && x.pulled === undefined && !f2) x.pulled = false;
    if (v === 'chest') { if (x.contents === undefined) x.contents = 'gold:50'; if (x.hidden === undefined) x.hidden = false; }
    if (v === 'shrine' && (x.element === undefined || x.element === null)) x.element = 'ice';
    if (f2) objDefaults2(x);
  })));
  if (o.type === 'torch') f.append(checkRow('Lit', 'starts lit', o.lit === true, v => apply(x => { x.lit = v; })));
  if (o.type === 'lever' && !f2) f.append(checkRow('Pulled', 'starts pulled', o.pulled === true, v => apply(x => { x.pulled = v; })));
  if (f2) objectFields2(f, o, apply);
  if (o.type === 'shrine') f.append(selectRow('Element', o.element === null || o.element === undefined ? '' : String(o.element),
    [['', '(choose)'], ...SHRINE_ELEMENTS.map(k => [k, k])], v => apply(x => { x.element = v === '' ? null : v; })));
  if (o.type === 'chest') {
    const simple = f2 ? F2.CONTENTS : CONTENT_SIMPLE;
    let [k, n] = contentsParts(o.contents);
    if (f2 && k === 'custom' && simple.includes(o.contents)) k = o.contents;
    const opts = [['', '(none)'], ...simple.map(c => [c, c]), ...CONTENT_COUNTED.map(c => [c, c + ':N']), ['custom', 'custom text']];
    const num = el('input', { type: 'number', min: 1, step: 1, class: 'short', value: n, hidden: !CONTENT_COUNTED.includes(k),
      onchange: ev => apply(x => { x.contents = k + ':' + Math.max(1, parseInt(ev.target.value, 10) || 1); }) });
    const sel = el('select', { onchange: ev => apply(x => {
      const v = ev.target.value;
      if (v === '') x.contents = null;
      else if (CONTENT_COUNTED.includes(v)) x.contents = v + ':' + (n || { gold: 50, bombs: 5, elixirs: 2 }[v]);
      else if (v === 'custom') x.contents = typeof x.contents === 'string' ? x.contents : '';
      else x.contents = v;
    }) });
    for (const [v, t] of opts) sel.append(el('option', { value: v }, t));
    sel.value = o.contents === null || o.contents === undefined ? '' : k;
    f.append(el('div', { class: 'row' }, el('label', {}, 'Contents'), sel, num));
    const txt = el('input', { type: 'text', value: typeof o.contents === 'string' ? o.contents : '', spellcheck: 'false',
      onchange: ev => apply(x => { const v = ev.target.value.trim(); x.contents = v === '' ? null : v; }) });
    f.append(el('div', { class: 'row' }, el('label', {}, ''), txt));
    f.append(checkRow('Hidden', f2 ? 'until the shutter is solved' : 'until the puzzle is solved', o.hidden === true, v => apply(x => { x.hidden = v; })));
  }
  void isTpl;
  return f;
}
function paintPreview(c, sp) {
  const g = c.getContext('2d');
  g.imageSmoothingEnabled = false; g.clearRect(0, 0, c.width, c.height);
  if (sp && ready(sp.im)) {
    const s = Math.min(1, 38 / Math.max(sp.w, sp.h));
    g.drawImage(sp.im, (c.width - sp.w * s) / 2, (c.height - sp.h * s) / 2, sp.w * s, sp.h * s);
  } else { g.fillStyle = '#555'; g.fillRect(12, 12, 16, 16); }
}
function renderPlaceForm() {
  const pf = $('placeForm');
  pf.innerHTML = '';
  if (S.mode === 'enemy') {
    $('placeTitle').textContent = 'New enemy';
    pf.append(enemyForm(S.tplEnemy, fn => { fn(S.tplEnemy); renderPlaceForm(); }));
    pf.append(el('div', { class: 'note' }, 'Click an empty tile to place. Click a placed enemy to select and drag it. Right click deletes.'));
  } else if (S.mode === 'object') {
    $('placeTitle').textContent = 'New object';
    pf.append(objectForm(S.tplObject, fn => { fn(S.tplObject); renderPlaceForm(); }, true));
    pf.append(el('div', { class: 'note' }, 'Click an empty tile to place. Click a placed object to select and drag it. Right click deletes.'));
  } else if (S.mode === 'select') {
    $('placeTitle').textContent = 'Select';
    pf.append(el('div', { class: 'note' }, 'Click an enemy, object or the mage to select it; drag to move; Del deletes; arrow keys nudge.'));
  } else {
    $('placeTitle').textContent = { paint: 'Paint', rect: 'Rectangle fill', fill: 'Flood fill', pick: 'Eyedropper' }[S.mode];
    pf.append(el('div', { class: 'note' }, {
      paint: 'Click or drag to paint the brush. Right click paints floor. Alt+click picks a tile.',
      rect: 'Drag a rectangle to fill it with the brush. Right-drag fills with floor.',
      fill: 'Click to flood-fill the connected area of the same tile.',
      pick: 'Click a tile to take its type as the brush.' }[S.mode]));
    pf.append(el('div', { class: 'note' }, 'Brush: ', el('b', {}, tileName(S.brush) + ' (' + S.brush + ')')));
  }
}
function selected() {
  const R = room();
  if (!S.sel || !R) return null;
  if (S.sel.list === 'mage') return S.mage;
  return R[S.sel.list][S.sel.i] || null;
}
function renderSelForm() {
  const sf = $('selForm');
  sf.innerHTML = '';
  const it = selected();
  if (!it) { sf.className = 'note'; sf.textContent = 'Nothing selected. Use Select (V) or click a placed thing with the Enemy / Object tool.'; return; }
  sf.className = '';
  if (S.sel.list === 'mage') { sf.append(el('div', { class: 'note' }, 'The mage (editor only, not saved) at ' + it.x + ', ' + it.y + '.')); return; }
  const list = S.sel.list, i = S.sel.i;
  const apply = fn => commit(() => { fn(room()[list][i]); });
  sf.append(el('div', { class: 'note' }, (list === 'enemies' ? 'Enemy' : 'Object') + ' #' + (i + 1) + ' at column ' + it.x + ', row ' + it.y));
  sf.append(list === 'enemies' ? enemyForm(it, apply) : objectForm(it, apply, false));
  sf.append(el('div', { class: 'row' },
    el('button', { class: 'small', onclick: () => copyToTemplate(it, list) }, 'Use as new-item template'),
    el('button', { class: 'small danger', onclick: deleteSelected }, 'Delete')));
}
function copyToTemplate(it, list) {
  if (list === 'enemies') { S.tplEnemy = { kind: it.kind, element: it.element, hp: it.hp, still: it.still }; setMode('enemy'); }
  else { S.tplObject = Object.assign({}, S.tplObject, JSON.parse(JSON.stringify(it))); delete S.tplObject.x; delete S.tplObject.y; setMode('object'); }
}
function renderLegend() {
  const lg = $('legend');
  const R = room();
  if (!R) { lg.textContent = ''; return; }
  const cnt = {};
  const [W, H] = dims();
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { const c = tileAt(R, x, y); cnt[c] = (cnt[c] || 0) + 1; }
  const blocks = R.objects.filter(o => o.type === 'block').length;
  lg.innerHTML = '';
  lg.append(`${R.enemies.length} enemies, ${R.objects.length} objects, ${cnt['o'] || 0} plates, ${blocks} blocks. `,
    'Markers: ', el('span', { class: 'sw', style: 'background:#50dc78' }), 'door gap',
    el('span', { class: 'sw', style: 'background:#ffb43c' }), 'lock / seal / crack',
    el('span', { class: 'sw', style: 'background:#c490ff' }), 'stairs',
    el('span', { class: 'sw', style: 'background:#e6aa5a' }), 'decor (solid, drawn as floor)');
}

// ------------------------------------------------------------------ Check
function runCheck() {
  const L = S.level, out = [];
  const add = (sev, slot, msg, x, y, w, h) => out.push({ sev, slot, msg, x, y, w: w || 1, h: h || 1 });
  if (L.format === 2) { runCheck2(L, add); S.issues = out; return out; }
  for (const p of L.problems || []) add('error', null, p);
  if (L.format !== 1) add('error', null, '"format" must be 1 (got ' + JSON.stringify(L.format) + ')');
  if (L.kind !== 'cave' && L.kind !== 'lair') add('error', null, 'unknown kind ' + JSON.stringify(L.kind) + ' (cave or lair)');
  const max = L.kind === 'lair' ? 6 : 8;
  if (!isInt(L.number) || L.number < 1 || L.number > max) add('error', null, 'number must be 1-' + max + ' for a ' + L.kind + ' (got ' + JSON.stringify(L.number) + ')');
  if (S.savedName && S.savedName !== fileName(L)) add('warn', null, 'opened as ' + S.savedName + ' but kind/number make it ' + fileName(L) + ' (the number must match the file name)');
  if (typeof L.name !== 'string') add('error', null, '"name" must be a string');
  else if (!NAME_RE.test(L.name)) add('error', null, 'name "' + L.name + '": upper-case letters, digits and basic punctuation only');
  if (L.theme !== null && (!isInt(L.theme) || L.theme < 0 || L.theme >= THEMES.length)) add('error', null, 'theme must be 0-' + (THEMES.length - 1) + ' (got ' + JSON.stringify(L.theme) + ')');
  for (const k of Object.keys(L.extra)) add('warn', null, 'unknown top-level key "' + k + '" (ignored by the game)');
  const known = slotsFor(L);
  const all = L.kind === 'lair' ? SLOTS.lair : SLOTS.cave;
  const slots = Object.keys(L.rooms);
  if (!slots.length && !(L.problems || []).length) add('warn', null, 'no rooms defined: the file changes nothing but the name / theme');
  for (const slot of slots) {
    if (!known.includes(slot)) {
      add('error', slot, all.includes(slot)
        ? '"' + slot + '" only exists in ' + (slot === 'feast' ? 'lair 1' : 'lairs 2-6')
        : 'unknown room slot "' + slot + '" for a ' + L.kind);
    }
    checkRoom(L, slot, L.rooms[slot], add);
  }
  S.issues = out;
  return out;
}
function checkRoom(L, slot, R, add) {
  for (const p of R.problems || []) add('error', slot, p);
  // Size and characters.
  if (R.tiles.length !== ROWS) add('error', slot, 'tiles has ' + R.tiles.length + ' rows, needs 13', 0, 0, COLS, ROWS);
  R.tiles.forEach((row, y) => { if (row.length !== COLS && y < ROWS) add('error', slot, 'row ' + y + ' has ' + row.length + ' characters, needs 16', 0, y, COLS, 1); });
  let bad = 0, ring = 0, firstRing = null;
  for (let y = 0; y < Math.min(R.tiles.length, ROWS); y++) for (let x = 0; x < R.tiles[y].length; x++) {
    const c = R.tiles[y][x];
    if (!TILE_CHARS.includes(c)) { if (bad++ < 5) add('error', slot, 'unknown tile character "' + c + '" at ' + x + ',' + y, x, y); }
    else if ((x === 0 || y === 0 || x === COLS - 1 || y === ROWS - 1) && c !== '#') { ring++; if (!firstRing) firstRing = [x, y]; }
  }
  if (bad > 5) add('error', slot, (bad - 5) + ' more unknown tile characters');
  if (ring) add('warn', slot, 'outer ring should be "#" (' + ring + ' tile' + (ring > 1 ? 's' : '') + ' not, first at ' + firstRing + ')', firstRing[0], firstRing[1]);
  // Puzzle / fields.
  const allowed = puzzlesFor(L.kind, slot);
  if (R.puzzle === null) {
    if (L.kind === 'cave' && slot === 'challenge') add('error', slot, 'the challenge room needs "puzzle": combat, freeze_plate or none');
  } else if (!PUZZLES.includes(R.puzzle)) add('error', slot, 'unknown puzzle ' + JSON.stringify(R.puzzle));
  else if (!allowed.includes(R.puzzle)) add('error', slot, 'puzzle "' + R.puzzle + '" is not allowed in the ' + L.kind + ' ' + slot + ' room (allowed: ' + allowed.join(', ') + ')');
  if (typeof R.combat !== 'boolean') add('error', slot, '"combat" must be true or false');
  else if (R.combat && !(L.kind === 'lair' && slot === 'hub')) add('warn', slot, '"combat": true is only used by the lair hub');
  if (R.random_enemies !== null && (!isInt(R.random_enemies) || R.random_enemies < 0)) add('error', slot, '"random_enemies" must be a whole number 0 or more');
  for (const k of Object.keys(R.extra)) add('warn', slot, 'unknown room key "' + k + '" (ignored)');
  const plates = [];
  for (let y = 0; y < ROWS; y++) for (let x = 0; x < COLS; x++) if (tileAt(R, x, y) === 'o') plates.push([x, y]);
  const count = (t, pred) => R.objects.filter(o => o.type === t && (!pred || pred(o))).length;
  const blocks = count('block');
  if (R.puzzle === 'plates') {
    if (blocks < plates.length) add('error', slot, 'plates puzzle with ' + plates.length + ' plates but only ' + blocks + ' blocks', plates[0] && plates[0][0], plates[0] && plates[0][1]);
    if (!plates.length) add('warn', slot, 'plates puzzle without any "o" plates');
  }
  if (R.puzzle === 'freeze_plate' && !plates.length) add('error', slot, 'freeze_plate room without plates ("o")');
  if (R.puzzle === 'freeze_plate' && plates.length && !R.enemies.length && R.random_enemies === 0) add('warn', slot, 'freeze_plate room with no monsters to freeze onto the plates');
  if (R.puzzle === 'torches' && !count('torch')) add('warn', slot, 'torches puzzle without any torch');
  if (R.puzzle === 'icebridge' && !R.tiles.some(r => r.includes('~'))) add('warn', slot, 'icebridge puzzle without water ("~")');
  if (R.puzzle === 'hidden' && !R.tiles.some(r => r.includes('c'))) add('warn', slot, 'hidden puzzle without a cracked wall ("c")');
  const keyChest = o => o.contents === 'key';
  if (L.kind === 'lair' && slot === 'west' && !count('chest', keyChest)) add('error', slot, 'lair west room has no key chest (a chest with "contents": "key")');
  if (!(L.kind === 'lair' && slot === 'west')) R.objects.forEach(o => { if (o.type === 'chest' && keyChest(o)) add('warn', slot, 'key chest outside the lair west room', o.x, o.y); });
  if (L.kind === 'lair' && slot === 'east' && (R.puzzle === 'icebridge' || R.puzzle === 'hidden') && !count('lever')) add('warn', slot, 'east ' + R.puzzle + ' puzzle needs a lever to pull');
  if (L.kind === 'cave' && slot === 'treasure' && !count('chest')) add('warn', slot, 'the treasure room should hold a chest (its contents is the reward)');
  if (L.kind === 'lair' && slot === 'stairs') {
    for (const [x, y] of STAIRS) { const c = tileAt(R, x, y); if (c !== '.' && c !== undefined) add('warn', slot, 'tile ' + x + ',' + y + ' is under the boss stairs (the game overwrites it)', x, y); }
  }
  // Door gaps: the tile just inside should be walkable.
  for (const g of gapsFor(L, slot)) {
    const inner = gapTiles(g.side).map(([x, y]) => [x + (g.side === 'w') - (g.side === 'e'), y + (g.side === 'n') - (g.side === 's')]);
    const blocked = inner.filter(([x, y]) => '#~cD'.includes(tileAt(R, x, y) || '#'));
    if (blocked.length === inner.length) add('warn', slot, 'the ' + { n: 'north', s: 'south', e: 'east', w: 'west' }[g.side] + ' doorway (' + g.label + ') is blocked from inside', inner[0][0], inner[0][1],
      g.side === 'n' || g.side === 's' ? 4 : 1, g.side === 'n' || g.side === 's' ? 1 : 3);
  }
  // Enemies.
  R.enemies.forEach((e, i) => {
    const w = 'enemy #' + (i + 1) + ' (' + e.kind + ')';
    if (e.extra._bad) { add('error', slot, 'enemy #' + (i + 1) + ' is not an object'); return; }
    if (!KINDS.includes(e.kind)) add('error', slot, 'unknown enemy kind ' + JSON.stringify(e.kind), e.x, e.y);
    if (e.element !== null && !ELEMENTS.includes(e.element)) add('error', slot, w + ': unknown element ' + JSON.stringify(e.element), e.x, e.y);
    if (!isInt(e.x) || !isInt(e.y) || e.x < 0 || e.x >= COLS || e.y < 0 || e.y >= ROWS) { add('error', slot, w + ': x must be 0-15 and y 0-12 (got ' + JSON.stringify(e.x) + ', ' + JSON.stringify(e.y) + ')'); return; }
    if (e.hp !== null && (typeof e.hp !== 'number' || !isFinite(e.hp) || e.hp <= 0)) add('error', slot, w + ': hp must be a positive number', e.x, e.y);
    if (typeof e.still !== 'boolean') add('error', slot, w + ': "still" must be true or false', e.x, e.y);
    const t = tileAt(R, e.x, e.y);
    if (t === '#' || t === 'c') add('error', slot, w + ' placed on a wall at ' + e.x + ',' + e.y, e.x, e.y);
    else if (t === '~' || t === 'D') add('warn', slot, w + ' placed on ' + TILE_NAMES[t] + ' at ' + e.x + ',' + e.y + ' (solid)', e.x, e.y);
    for (const k of Object.keys(e.extra)) add('warn', slot, w + ': unknown key "' + k + '"', e.x, e.y);
  });
  // Objects.
  const taken = {};
  R.objects.forEach((o, i) => {
    const w = 'object #' + (i + 1) + ' (' + o.type + ')';
    if (o.extra._bad) { add('error', slot, 'object #' + (i + 1) + ' is not an object'); return; }
    if (!OBJ_TYPES.includes(o.type)) add('error', slot, 'unknown object type ' + JSON.stringify(o.type), o.x, o.y);
    if (!isInt(o.x) || !isInt(o.y) || o.x < 0 || o.x >= COLS || o.y < 0 || o.y >= ROWS) { add('error', slot, w + ': x must be 0-15 and y 0-12'); return; }
    if (o.type === 'chest') {
      if (o.contents === null) add('warn', slot, 'chest without "contents"', o.x, o.y);
      else if (typeof o.contents !== 'string' || !CONTENT_RE.test(o.contents)) add('error', slot, 'unknown chest contents ' + JSON.stringify(o.contents), o.x, o.y);
      if (typeof o.hidden !== 'boolean') add('error', slot, w + ': "hidden" must be true or false', o.x, o.y);
    }
    if (o.type === 'shrine') {
      if (o.element === null) add('error', slot, 'shrine without an "element"', o.x, o.y);
      else if (!SHRINE_ELEMENTS.includes(o.element)) add('error', slot, 'unknown shrine element ' + JSON.stringify(o.element), o.x, o.y);
    }
    if (o.type === 'torch' && typeof o.lit !== 'boolean') add('error', slot, w + ': "lit" must be true or false', o.x, o.y);
    if (o.type === 'lever' && typeof o.pulled !== 'boolean') add('error', slot, w + ': "pulled" must be true or false', o.x, o.y);
    const t = tileAt(R, o.x, o.y);
    if (t === '#' || t === 'c' || t === '~' || t === 'D') add('warn', slot, w + ' placed on ' + TILE_NAMES[t] + ' at ' + o.x + ',' + o.y, o.x, o.y);
    const k = o.x + ',' + o.y;
    if (taken[k]) add('warn', slot, w + ' shares tile ' + k + ' with another object', o.x, o.y);
    taken[k] = true;
    for (const x of Object.keys(o.extra)) add('warn', slot, w + ': unknown key "' + x + '"', o.x, o.y);
  });
}
function renderCheck() {
  const ul = $('checkList'), sum = $('checkSummary');
  ul.innerHTML = '';
  if (!S.issues) { sum.textContent = ''; sum.className = ''; ul.append(el('li', { class: 'note' }, 'Press Check (K) to validate the level.')); return; }
  const errs = S.issues.filter(i => i.sev === 'error').length, warns = S.issues.length - errs;
  sum.textContent = errs ? errs + ' error' + (errs > 1 ? 's' : '') + (warns ? ', ' + warns + ' warning' + (warns > 1 ? 's' : '') : '')
    : warns ? 'OK, ' + warns + ' warning' + (warns > 1 ? 's' : '') : 'all good';
  sum.className = errs ? 'bad' : warns ? 'warn' : 'ok';
  if (!S.issues.length) ul.append(el('li', { class: 'note' }, 'No problems found. The game will accept this file.'));
  for (const it of S.issues) {
    const where = isF2() ? (isInt(it.slot) && S.level.rooms[it.slot] ? String(S.level.rooms[it.slot].id) : null) : it.slot;
    ul.append(el('li', { class: 'item ' + it.sev, onclick: () => gotoIssue(it) },
      el('span', { class: 'sev' }, it.sev === 'error' ? '✖' : '!'),
      el('span', {}, where ? el('span', { class: 'where' }, where + (isInt(it.x) ? ' ' + it.x + ',' + it.y : '') + '  ') : '', it.msg)));
  }
}
function gotoIssue(it) {
  const has = isF2() ? isInt(it.slot) && !!S.level.rooms[it.slot] : !!it.slot;
  if (has && S.level.rooms[it.slot]) { if (S.slot !== it.slot) S.doorForm = null; S.slot = it.slot; S.sel = null; }
  if (isF2() && has) S.flash = { slot: it.slot, x: it.x, y: it.y, w: it.w, h: it.h, until: Date.now() + 2200 }; // also flashes the room on the map
  if (has && isInt(it.x) && isInt(it.y)) {
    S.flash = { slot: it.slot, x: it.x, y: it.y, w: it.w, h: it.h, until: Date.now() + 2200 };
    const R = room();
    if (R) {
      const ei = R.enemies.findIndex(e => e.x === it.x && e.y === it.y && it.msg.includes('enemy'));
      const oi = R.objects.findIndex(o => o.x === it.x && o.y === it.y);
      if (ei >= 0) S.sel = { list: 'enemies', i: ei };
      else if (oi >= 0 && !it.msg.startsWith('enemy')) S.sel = { list: 'objects', i: oi };
    }
  }
  renderAll();
}

// ------------------------------------------------------------------ mouse on the grid
function tileFromEvent(ev) {
  const r = cv.getBoundingClientRect();
  const x = Math.floor((ev.clientX - r.left) / (TS * S.zoom)), y = Math.floor((ev.clientY - r.top) / (TS * S.zoom));
  const [W, H] = dims();
  return (x >= 0 && y >= 0 && x < W && y < H) ? { x, y } : null;
}
function rectOf(a, b) { return [Math.min(a.x, b.x), Math.min(a.y, b.y), Math.max(a.x, b.x), Math.max(a.y, b.y)]; }
function hitEntity(R, t, lists) {
  for (const list of lists) {
    if (list === 'mage') { if (S.mage.show && S.mage.x === t.x && S.mage.y === t.y) return { list: 'mage', i: 0 }; continue; }
    for (let i = R[list].length - 1; i >= 0; i--) if (R[list][i].x === t.x && R[list][i].y === t.y) return { list, i };
  }
  return null;
}
function paintLine(R, a, b, ch) {
  let x0 = a.x, y0 = a.y; const x1 = b.x, y1 = b.y;
  const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx + dy;
  for (;;) {
    setTile(R, x0, y0, ch);
    if (x0 === x1 && y0 === y1) break;
    const e2 = 2 * err;
    if (e2 >= dy) { err += dy; x0 += sx; }
    if (e2 <= dx) { err += dx; y0 += sy; }
  }
}
function floodFill(R, t, ch) {
  const from = tileAt(R, t.x, t.y);
  if (from === ch) return;
  const st = [[t.x, t.y]];
  const [COLS, ROWS] = dims();
  while (st.length) {
    const [x, y] = st.pop();
    if (x < 0 || y < 0 || x >= COLS || y >= ROWS || tileAt(R, x, y) !== from) continue;
    setTile(R, x, y, ch);
    st.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
  }
}
cv.addEventListener('contextmenu', e => e.preventDefault());
cv.addEventListener('mousedown', ev => {
  const R = room(), t = tileFromEvent(ev);
  if (!R || !t) return;
  ev.preventDefault();
  const right = ev.button === 2;
  if (ev.button !== 0 && !right) return;
  let mode = S.mode;
  if (ev.altKey && !right) mode = 'pick';
  if (mode === 'pick') {
    const c = tileAt(R, t.x, t.y);
    if (tileChars().includes(c)) { S.brush = c; if (S.mode === 'pick') S.mode = 'paint'; toast('Brush: ' + tileName(c)); }
    renderAll(); return;
  }
  if (mode === 'paint') {
    const before = snap();
    const ch = right ? '.' : S.brush;
    setTile(R, t.x, t.y, ch);
    S.drag = { type: 'paint', before, last: t, ch };
    drawRoom(); return;
  }
  if (mode === 'rect') { S.drag = { type: 'rect', start: t, cur: t, ch: right ? '.' : S.brush }; drawRoom(); return; }
  if (mode === 'fill') { commit(() => floodFill(R, t, right ? '.' : S.brush)); return; }
  // Entity tools.
  const lists = mode === 'enemy' ? ['enemies', 'objects', 'mage'] : mode === 'object' ? ['objects', 'enemies', 'mage'] : ['enemies', 'objects', 'mage'];
  const hit = hitEntity(R, t, lists);
  if (right) {
    if (hit && hit.list !== 'mage') commit(() => { room()[hit.list].splice(hit.i, 1); S.sel = null; });
    return;
  }
  if (hit) {
    S.sel = hit;
    S.drag = { type: 'move', before: snap(), hit, from: t };
    renderAll(); return;
  }
  if (mode === 'enemy') {
    commit(() => {
      const tp = S.tplEnemy;
      R.enemies.push({ kind: tp.kind, element: tp.element, x: t.x, y: t.y, hp: tp.hp, still: tp.still, extra: {} });
      S.sel = { list: 'enemies', i: R.enemies.length - 1 };
    });
  } else if (mode === 'object') {
    if (isF2()) {
      commit(() => { R.objects.push(newObject2(S.tplObject, t.x, t.y)); S.sel = { list: 'objects', i: R.objects.length - 1 }; });
      return;
    }
    commit(() => {
      const tp = S.tplObject, o = { type: tp.type, x: t.x, y: t.y, extra: {} };
      if (tp.type === 'torch') o.lit = !!tp.lit;
      if (tp.type === 'lever') o.pulled = !!tp.pulled;
      if (tp.type === 'chest') { o.contents = tp.contents === undefined ? null : tp.contents; o.hidden = !!tp.hidden; }
      if (tp.type === 'shrine') o.element = tp.element || 'ice';
      R.objects.push(o);
      S.sel = { list: 'objects', i: R.objects.length - 1 };
    });
  } else { S.sel = null; renderAll(); }
});
window.addEventListener('mousemove', ev => {
  const t = tileFromEvent(ev);
  const R = room();
  const onCanvas = ev.target === cv;
  const prev = S.hover;
  S.hover = onCanvas ? t : null;
  if (R && S.hover) {
    const c = tileAt(R, t.x, t.y);
    const ents = [...R.enemies.filter(e => e.x === t.x && e.y === t.y).map(e => e.kind), ...R.objects.filter(o => o.x === t.x && o.y === t.y).map(o => o.type)];
    $('hoverInfo').textContent = 'col ' + t.x + ', row ' + t.y + '  ' + tileName(c) + (ents.length ? '  [' + ents.join(', ') + ']' : '');
  } else $('hoverInfo').textContent = '';
  const d = S.drag;
  if (d && R && t) {
    if (d.type === 'paint') { paintLine(R, d.last, t, d.ch); d.last = t; }
    else if (d.type === 'rect') d.cur = t;
    else if (d.type === 'move') {
      const it = d.hit.list === 'mage' ? S.mage : R[d.hit.list][d.hit.i];
      if (it) { it.x = t.x; it.y = t.y; }
    }
    drawRoom();
  } else if ((prev && !t) || (t && (!prev || prev.x !== t.x || prev.y !== t.y))) drawRoom();
});
window.addEventListener('mouseup', () => {
  const d = S.drag;
  if (!d) return;
  S.drag = null;
  const R = room();
  if (d.type === 'rect' && R) {
    commit(() => {
      const [x0, y0, x1, y1] = rectOf(d.start, d.cur);
      for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) setTile(R, x, y, d.ch);
    });
    return;
  }
  if (d.type === 'paint' || d.type === 'move') {
    if (d.hit && d.hit.list === 'mage') { renderAll(); return; }
    if (snap() !== d.before) { pushUndo(d.before); markDirty(); }
    renderAll();
  }
});

// ------------------------------------------------------------------ commands
function setMode(m) { S.mode = m; if (m !== 'select' && m !== 'enemy' && m !== 'object') { /* keep selection */ } renderAll(); }
function setBrush(ch) { S.brush = ch; if (!['paint', 'rect', 'fill'].includes(S.mode)) S.mode = 'paint'; renderAll(); }
function deleteSelected() {
  if (!S.sel || S.sel.list === 'mage') return;
  const { list, i } = S.sel;
  commit(() => { room()[list].splice(i, 1); S.sel = null; });
}
function nudge(dx, dy) {
  const it = selected();
  if (!it) return;
  const [COLS, ROWS] = dims();
  const nx = Math.min(COLS - 1, Math.max(0, (it.x | 0) + dx)), ny = Math.min(ROWS - 1, Math.max(0, (it.y | 0) + dy));
  if (S.sel.list === 'mage') { S.mage.x = nx; S.mage.y = ny; renderAll(); return; }
  commit(() => { it.x = nx; it.y = ny; });
}
function confirmDiscard() {
  return !S.dirty || confirm('Discard unsaved changes to ' + fileName(S.level) + '?');
}
function loadLevel(L, name) {
  S.level = L; S.undo = []; S.redo = []; S.dirty = false; S.sel = null; S.savedName = name || null;
  S.doorForm = null; S.mapAdd = null; S.mapDrag = null;
  if (L.format === 2) {
    // Start in the entrance room.
    const e = L.rooms.findIndex(R => R.doors.some(d => d.to === 'exit'));
    S.slot = e >= 0 ? e : 0;
    if (!OBJ_TYPES.includes(S.tplObject.type) && !F2.OBJ_TYPES.includes(S.tplObject.type)) S.tplObject.type = 'chest';
  } else {
    const slots = slotsFor(L);
    S.slot = slots.find(s => L.rooms[s]) || Object.keys(L.rooms)[0] || slots[0];
    if (!OBJ_TYPES.includes(S.tplObject.type)) S.tplObject.type = 'chest';
  }
  runCheck();
  renderAll();
  const errs = S.issues.filter(i => i.sev === 'error').length;
  toast('Opened ' + (name || fileName(L)) + (errs ? ' - ' + errs + ' problem' + (errs > 1 ? 's' : '') + ', see Check' : ''), errs > 0);
}
function openText(text, name) {
  let j;
  try { j = JSON.parse(text); } catch (e) { toast('Not valid JSON: ' + e.message, true); return false; }
  loadLevel(fromJson(j), name);
  return true;
}
async function openFile(file) {
  if (!confirmDiscard()) return;
  openText(await file.text(), file.name);
}
function download() {
  const text = stringifyLevel(S.level), name = fileName(S.level);
  const a = el('a', { href: URL.createObjectURL(new Blob([text], { type: 'application/json' })), download: name });
  document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  S.dirty = false; S.savedName = name; renderFileState();
  toast('Downloaded ' + name + ' - move it into the levels/ folder');
}
async function save() {
  const errs = runCheck().filter(i => i.sev === 'error');
  renderAll();
  if (errs.length && !confirm(errs.length + ' problem(s) found by Check. The game will ignore the broken rooms. Save anyway?')) return;
  if (!S.dirHandle) { download(); return; }
  const name = fileName(S.level);
  try {
    if ((await S.dirHandle.queryPermission({ mode: 'readwrite' })) !== 'granted' &&
        (await S.dirHandle.requestPermission({ mode: 'readwrite' })) !== 'granted') throw new Error('permission denied');
    const fh = await S.dirHandle.getFileHandle(name, { create: true });
    const w = await fh.createWritable();
    await w.write(stringifyLevel(S.level));
    await w.close();
    S.dirty = false; S.savedName = name; renderFileState();
    toast('Saved ' + S.dirHandle.name + '/' + name);
    listFolder();
  } catch (e) { toast('Could not save into the folder (' + e.message + '); downloading instead', true); download(); }
}
async function chooseFolder() {
  if (!window.showDirectoryPicker) { toast('This browser cannot open folders; use Chrome or Edge. Save downloads the file instead.', true); return; }
  try {
    S.dirHandle = await window.showDirectoryPicker({ id: 'el-levels', mode: 'readwrite' });
  } catch (e) { return; }
  $('folderSection').hidden = false;
  renderFileState();
  listFolder();
}
async function listFolder() {
  if (!S.dirHandle) return;
  const ul = $('folderList');
  $('folderName').textContent = S.dirHandle.name + '/ - Save writes here.';
  const names = [];
  for await (const [n, h] of S.dirHandle.entries()) if (h.kind === 'file' && /\.json$/i.test(n)) names.push(n);
  const key = n => { const m = /^(cave|lair)_(\d+)\.json$/.exec(n); return m ? (m[1] === 'cave' ? 0 : 100) + +m[2] : 1000; };
  names.sort((a, b) => key(a) - key(b) || a.localeCompare(b));
  ul.innerHTML = '';
  if (!names.length) ul.append(el('li', { class: 'note' }, 'No .json files yet.'));
  for (const n of names) ul.append(el('li', { class: n === S.savedName ? 'current' : '', onclick: async () => {
    if (!confirmDiscard()) return;
    const f = await (await S.dirHandle.getFileHandle(n)).getFile();
    openText(await f.text(), n);
    listFolder();
  } }, n));
}
function showModal(id) {
  $('modalBack').hidden = false;
  for (const m of document.querySelectorAll('.modal')) m.hidden = m.id !== id;
}
function closeModal() { $('modalBack').hidden = true; }
const modalOpen = () => !$('modalBack').hidden;
function showJson() {
  $('jsonText').value = stringifyLevel(S.level);
  $('jsonNote').textContent = fileName(S.level);
  showModal('jsonModal');
}

// ------------------------------------------------------------------ wiring
function wire() {
  $('btnNew').onclick = () => { $('nKind').value = isF2() ? 'lair2' : S.level.kind === 'lair' ? 'lair' : 'cave'; $('nKind').onchange(); showModal('newModal'); };
  $('nKind').onchange = () => {
    const k = $('nKind').value;
    $('nNumber').max = k === 'cave' ? 8 : 6;
    $('newNote').textContent = k === 'lair2'
      ? 'A format 2 lair starts with one entrance room (exit south). Add rooms on the grid map, then doors between them.'
      : 'Every room starts as "not defined" (generated layout). Use Define room on a tab to draw one.';
  };
  $('btnNewOk').onclick = () => {
    const kind = $('nKind').value, n = parseInt($('nNumber').value, 10);
    const max = kind === 'cave' ? 8 : 6;
    if (!(n >= 1 && n <= max)) { toast('Number must be 1-' + max, true); return; }
    if (!confirmDiscard()) return;
    closeModal();
    if (kind === 'lair2') {
      S.level = newLevel2(n); S.undo = []; S.redo = []; S.dirty = false; S.sel = null; S.savedName = null; S.issues = null;
      S.slot = 0; S.doorForm = null; S.mapAdd = null;
      renderAll();
      return;
    }
    const L = newLevel(kind, n);
    S.level = L; S.undo = []; S.redo = []; S.dirty = false; S.sel = null; S.savedName = null; S.issues = null;
    S.slot = kind === 'lair' ? 'entry' : 'challenge';
    renderAll();
  };
  $('btnOpen').onclick = () => $('fileInput').click();
  $('fileInput').onchange = e => { const f = e.target.files[0]; if (f) openFile(f); e.target.value = ''; };
  $('btnSave').onclick = save;
  $('btnDownload').onclick = download;
  $('btnFolder').onclick = chooseFolder;
  $('btnRefresh').onclick = listFolder;
  if (!window.showDirectoryPicker) { $('btnFolder').title = 'Needs Chrome or Edge (File System Access API)'; $('btnFolder').disabled = true; }
  $('btnUndo').onclick = doUndo;
  $('btnRedo').onclick = doRedo;
  $('btnCheck').onclick = () => { runCheck(); renderAll(); };
  $('btnJson').onclick = showJson;
  $('btnHelp').onclick = () => showModal('helpModal');
  $('btnCopyJson').onclick = async () => {
    const t = $('jsonText');
    try { await navigator.clipboard.writeText(t.value); toast('Copied'); }
    catch (e) { t.select(); document.execCommand('copy'); toast('Copied'); }
  };
  $('btnLoadJson').onclick = () => { if (confirmDiscard() && openText($('jsonText').value, null)) closeModal(); };
  document.querySelectorAll('[data-close]').forEach(b => { b.onclick = closeModal; });
  $('modalBack').addEventListener('mousedown', e => { if (e.target === $('modalBack')) closeModal(); });

  $('fKind').onchange = e => {
    const v = e.target.value;
    const other = Object.keys(S.level.rooms).length;
    if (other && !confirm('Rooms already defined keep their slot names; slots that do not exist for a ' + v + ' will be flagged by Check. Change kind?')) { renderHeader(); return; }
    commit(() => { S.level.kind = v; if (isInt(S.level.number) && S.level.number > (v === 'lair' ? 6 : 8)) S.level.number = v === 'lair' ? 6 : 8; });
  };
  $('fRelic').onchange = e => commit(() => { S.level.relic = e.target.value === '' ? null : e.target.value; });
  wireMap();
  $('fNumber').onchange = e => { const n = parseInt(e.target.value, 10); commit(() => { S.level.number = isNaN(n) ? S.level.number : n; }); };
  $('fName').oninput = e => {
    const pos = e.target.selectionStart;
    e.target.value = e.target.value.toUpperCase();
    e.target.setSelectionRange(pos, pos);
  };
  $('fName').onchange = e => commit(() => { S.level.name = e.target.value.trim(); });
  $('fTheme').onchange = e => commit(() => { S.level.theme = e.target.value === '' ? null : +e.target.value; });
  $('fPuzzle').onchange = e => commit(() => { room().puzzle = e.target.value === '' ? null : e.target.value; });
  $('fCombat').onchange = e => commit(() => { room().combat = e.target.checked; });
  $('fRandomOn').onchange = e => commit(() => { room().random_enemies = e.target.checked ? 0 : null; });
  $('fRandom').onchange = e => commit(() => { const n = parseInt(e.target.value, 10); room().random_enemies = isNaN(n) ? 0 : Math.max(0, n); });
  $('btnDefine').onclick = () => commit(() => {
    const R = blankRoom();
    if (S.level.kind !== 'lair' && S.slot === 'challenge') R.puzzle = 'combat';
    S.level.rooms[S.slot] = R;
  });
  $('btnFixSize').onclick = () => commit(() => normalizeRoom(room()));
  $('btnUndefine').onclick = () => {
    if (!confirm('Remove the ' + S.slot + ' room? It will use the generated layout (undo brings it back).')) return;
    commit(() => { delete S.level.rooms[S.slot]; S.sel = null; });
  };
  document.querySelectorAll('#modeButtons button').forEach(b => { b.onclick = () => setMode(b.dataset.mode); });
  document.querySelectorAll('.zoom').forEach(b => { b.onclick = () => { S.zoom = +b.dataset.zoom; renderAll(); }; });
  $('tGrid').onchange = e => { S.grid = e.target.checked; drawRoom(); };
  $('tFeatures').onchange = e => { S.features = e.target.checked; drawRoom(); };
  $('tMage').onchange = e => { S.mage.show = e.target.checked; if (!S.mage.show && S.sel && S.sel.list === 'mage') S.sel = null; renderAll(); };

  // Drag and drop.
  let depth = 0;
  window.addEventListener('dragenter', e => { if ([...e.dataTransfer.types].includes('Files')) { depth++; $('dropOverlay').hidden = false; e.preventDefault(); } });
  window.addEventListener('dragleave', () => { if (--depth <= 0) { depth = 0; $('dropOverlay').hidden = true; } });
  window.addEventListener('dragover', e => e.preventDefault());
  window.addEventListener('drop', e => {
    e.preventDefault(); depth = 0; $('dropOverlay').hidden = true;
    const f = e.dataTransfer.files[0];
    if (f) openFile(f);
  });
  window.addEventListener('beforeunload', e => { if (S.dirty) { e.preventDefault(); e.returnValue = ''; } });

  window.addEventListener('keydown', e => {
    const tag = (e.target.tagName || '').toLowerCase();
    const typing = tag === 'input' || tag === 'textarea' || tag === 'select';
    const ctrl = e.ctrlKey || e.metaKey;
    if (ctrl && e.key.toLowerCase() === 's') { e.preventDefault(); save(); return; }
    if (ctrl && e.key.toLowerCase() === 'o') { e.preventDefault(); $('fileInput').click(); return; }
    if (e.key === 'Escape') { if (modalOpen()) closeModal(); else { S.sel = null; S.drag = null; S.mapAdd = null; renderAll(); } return; }
    if (typing || modalOpen()) return;
    if (ctrl && e.key.toLowerCase() === 'z') { e.preventDefault(); if (e.shiftKey) doRedo(); else doUndo(); return; }
    if (ctrl && e.key.toLowerCase() === 'y') { e.preventDefault(); doRedo(); return; }
    if (ctrl || e.altKey) return;
    const k = e.key;
    const b = brushes().find(x => x.key === k);
    if (b) { setBrush(b.ch); return; }
    const modes = { b: 'paint', r: 'rect', g: 'fill', i: 'pick', e: 'enemy', o: 'object', v: 'select' };
    if (modes[k.toLowerCase()] && !e.shiftKey) { setMode(modes[k.toLowerCase()]); return; }
    if (k === 'Delete' || k === 'Backspace') { e.preventDefault(); deleteSelected(); return; }
    const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (arrows[k] && S.sel) { e.preventDefault(); nudge(...arrows[k]); return; }
    if ((k === '[' || k === ']') && isF2()) {
      const n = S.level.rooms.length;
      if (n) { S.slot = ((isInt(S.slot) ? S.slot : 0) + (k === ']' ? 1 : n - 1)) % n; S.sel = null; S.doorForm = null; renderAll(); }
      return;
    }
    if (k === '[' || k === ']') {
      const slots = [...document.querySelectorAll('#tabs button')].map(x => x.textContent.replace(' (unknown)', ''));
      const i = slots.indexOf(S.slot);
      S.slot = slots[(i + (k === ']' ? 1 : slots.length - 1)) % slots.length]; S.sel = null; renderAll(); return;
    }
    if (k === '-' || k === '_') { S.zoom = Math.max(1, S.zoom - 1); renderAll(); return; }
    if (k === '=' || k === '+') { S.zoom = Math.min(3, S.zoom + 1); renderAll(); return; }
    if (k.toLowerCase() === 'l') { S.grid = !S.grid; renderAll(); return; }
    if (k.toLowerCase() === 'f') { S.features = !S.features; renderAll(); return; }
    if (k.toLowerCase() === 'm') { S.mage.show = !S.mage.show; renderAll(); return; }
    if (k.toLowerCase() === 'k') { runCheck(); renderAll(); return; }
    if (k === '?') { showModal('helpModal'); return; }
  });
}

// Expose a tiny API for scripted tests / the console.
window.ELEditor = {
  state: S, stringifyLevel, fromJson, runCheck, openText, keepGraph, solvable2,
  get json() { return stringifyLevel(S.level); },
};

preload();
wire();
S.level = newLevel('cave', 1);
renderAll();
if (!A.themes || !Object.keys(A.themes).length) toast('assets.js missing or empty: run python scripts/build_editor_assets.py (drawing plain colours for now)', true);
