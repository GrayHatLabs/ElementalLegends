// Pixel art. Sprites with 'a' (main) / 'b' (light) / 'j' (glow) pixels are
// palette-swapped per element, so one drawing gives fire, ice, storm, earth
// and neutral versions.
use crate::gfx::*;
use crate::world::Mul;

// Index order matches game::Elem: Fire, Ice, Storm, Earth, Neutral.
pub const EL_MAIN: [u32; 5] = [rgb(0xd82800), rgb(0x0070ec), rgb(0x8030c8), rgb(0x207c10), rgb(0x747474)];
pub const EL_LIGHT: [u32; 5] = [rgb(0xfc9838), rgb(0xa4e4fc), rgb(0xfce040), rgb(0x98d858), rgb(0xbcbcbc)];

pub fn pal(ch: char) -> u32 {
    rgb(match ch {
        'k' => 0x101010,
        'w' => 0xfcfcfc,
        'g' => 0xbcbcbc,
        'G' => 0x747474,
        'r' => 0xd82800,
        'o' => 0xfc7460,
        'y' => 0xfcbc3c,
        'b' => 0x0058f8,
        'c' => 0x3cbcfc,
        'e' => 0x00a800,
        'l' => 0xb8f818,
        's' => 0xfcd8a8,
        'd' => 0xa85020,
        'm' => 0xf878f8,
        'p' => 0x9818a8,
        _ => 0xff00ff,
    })
}

fn tint(el: usize) -> impl Fn(char) -> u32 {
    move |ch| match ch {
        'a' => EL_MAIN[el],
        'b' | 'j' => EL_LIGHT[el],
        _ => pal(ch),
    }
}

/// A sprite plus its hit-flash (white) and frozen (ice) silhouettes.
pub struct Spr {
    pub img: Sprite,
    pub white: Sprite,
    pub ice: Sprite,
}

pub fn spr_with(rows: &[&str], p: impl Fn(char) -> u32) -> Spr {
    Spr {
        img: Sprite::from_rows(rows, &p),
        white: Sprite::from_rows(rows, |_| WHITE),
        ice: Sprite::from_rows(rows, |c| if c == 'k' { rgb(0x0058f8) } else { rgb(0xa4e4fc) }),
    }
}
fn spr(rows: &[&str]) -> Spr {
    spr_with(rows, pal)
}
fn tinted(rows: &[&str]) -> Vec<Spr> {
    (0..5).map(|e| spr_with(rows, tint(e))).collect()
}

const MAGE_D: [&str; 16] = [
    ".......kk.......",
    "......kaak......",
    ".....kaabk......",
    "....kaaaabk.....",
    "...kaaaaaabk....",
    "..kkkkkkkkkkk.j.",
    "....kssssssk.kjk",
    "....kskssksk..d.",
    "....kwwwwwwk..d.",
    "...kaawwwwaak.d.",
    "..kaabwwwwbaasd.",
    "..ksaabwwbaak.d.",
    "...kaaabbaaak.d.",
    "..kaaaabbaaaakd.",
    "..kaaaaaaaaaak..",
    "...kkk....kkk...",
];
const MAGE_U: [&str; 16] = [
    ".......kk.......",
    "......kaak......",
    ".....kaabk......",
    "....kaaaabk.....",
    "...kaaaaaabk....",
    "..kkkkkkkkkkk.j.",
    "....kaaaaaak.kjk",
    "....kaaaaaak..d.",
    "....kaaaaaak..d.",
    "...kaaaaaaaak.d.",
    "..kaaaabbaaaasd.",
    "..ksaaabbaaak.d.",
    "...kaaabbaaak.d.",
    "..kaaaabbaaaakd.",
    "..kaaaaaaaaaak..",
    "...kkk....kkk...",
];
const MAGE_S: [&str; 16] = [
    ".....kk.........",
    "....kaak........",
    "....kaabk.......",
    "...kaaaabk......",
    "...kaaaaabk.....",
    "..kkkkkkkkkkk...",
    "....kaassssk..j.",
    "....kaasksk..kjk",
    "....kaawwww...d.",
    "...kaaawwwk...d.",
    "...kaabbwwkssdd.",
    "...kaaabbak...d.",
    "...kaaabbak...d.",
    "..kaaaabbaak..d.",
    "..kaaaaaaaak....",
    "...kkk..kkk.....",
];

pub struct Sprites {
    pub mage_d: Vec<Spr>,
    pub mage_u: Vec<Spr>,
    pub mage_s: Vec<Spr>,
    /// Indexed [EnemyKind][element].
    pub enemies: Vec<Vec<Spr>>,
    pub generator: Spr,
    pub chest: Spr,
    pub chest_open: Spr,
    pub apple: Spr,
    pub bread: Spr,
    pub meat: Spr,
    pub coin: Spr,
    pub gem: Spr,
    pub potion: Spr,
    pub heart: Spr,
    pub big_heart: Spr,
    pub hoard: Spr,
}

impl Sprites {
    pub fn new() -> Self {
        let slime = tinted(&[
            "....kkkkkk....",
            "..kkaaaaaakk..",
            ".kaabbaaaaaak.",
            ".kabbaaaaaaak.",
            "kaaawkaaawkaak",
            "kaaaaaaaaaaaak",
            "kaaaakkkkaaaak",
            ".kaaaaaaaaaak.",
            "..kkkkkkkkkk..",
        ]);
        let bat = tinted(&[
            "k....kaak....k",
            "kk..kaaaak..kk",
            "kakkabwwbakkak",
            "kaaaaawwaaaaak",
            ".kaaak..kaaak.",
            "..kk......kk..",
        ]);
        let skeleton = tinted(&[
            "....kkkkk....",
            "...kbbbbbk...",
            "...kbkbkbk...",
            "...kbbbbbk...",
            "....kbkbk....",
            "..kkkbbbkkk..",
            ".kb.kbbbk.bk.",
            "....kbkbk....",
            "....kbbbk....",
            "...kb...bk...",
            "...kb...bk...",
            "..kbb...bbk..",
        ]);
        let imp = tinted(&[
            ".k.......k.",
            ".kk.kkk.kk.",
            "..kaaaaak..",
            ".kaywaywak.",
            ".kaaaaaaak.",
            "..kaakaak..",
            ".kaaaaaaak.",
            "ka.kaaak.ak",
            "...ka.ak...",
            "..kk...kk..",
        ]);
        let ghost = tinted(&[
            "....kkkk....",
            "..kkbbbbkk..",
            ".kbbbbbbbbk.",
            ".kbkkbbkkbk.",
            "kbbkkbbkkbbk",
            "kbbbbbbbbbbk",
            "kbbbbkkbbbbk",
            "kbbbbbbbbbbk",
            "kbabbabbabbk",
            "kk.kk.kk.kkk",
        ]);
        let golem = tinted(&[
            "...kkkkkkkk...",
            "..kaaaaaaaak..",
            "..kaywaaywak..",
            "..kaaaaaaaak..",
            "kkkkaaaaaakkkk",
            "kaabkaaaakbaak",
            "kaabkaaaakbaak",
            "kaak.kaak.kaak",
            "kkk.kaaaak.kkk",
            "....kaakaak...",
            "...kaak.kaak..",
            "...kkkk.kkkk..",
        ]);
        let generator = tinted(&[
            "...kkkkkkkkkk...",
            "..kGGGGGGGGGGk..",
            ".kGGggggggggGGk.",
            ".kGgkkggggkkgGk.",
            ".kGgkkggggkkgGk.",
            ".kGggggkkggggGk.",
            ".kGGgggggggggGk.",
            "..kGGgkgkgkgGGk.",
            "..kkGGGGGGGGGkk.",
            ".kGGGkGkGkGkGGGk",
            "kGGGGGGGGGGGGGGk",
            "kkkkkkkkkkkkkkkk",
        ]);
        Sprites {
            mage_d: tinted(&MAGE_D),
            mage_u: tinted(&MAGE_U),
            mage_s: tinted(&MAGE_S),
            enemies: vec![slime, bat, skeleton, imp, ghost, golem],
            generator: generator.into_iter().nth(4).unwrap(),
            chest: spr(&[
                ".kkkkkkkkkkkk.",
                "kddddddddddddk",
                "kdyyyyyyyyyydk",
                "kddddddddddddk",
                "kkkkkkyykkkkkk",
                "kddddkyykddddk",
                "kddddkkkkddddk",
                "kddddddddddddk",
                "kdyyyyyyyyyydk",
                "kkkkkkkkkkkkkk",
            ]),
            chest_open: spr(&[
                ".kkkkkkkkkkkk.",
                "kddddddddddddk",
                "kkkkkkkkkkkkkk",
                "kkkkkkkkkkkkkk",
                "kddddddddddddk",
                "kddddddddddddk",
                "kdyyyyyyyyyydk",
                "kkkkkkkkkkkkkk",
            ]),
            apple: spr(&["...e...", "..ke...", ".rrkrr.", "rrwrrrr", "rrrrrrr", ".rrrrr.", "..r.r.."]),
            bread: spr(&["..kkkkkk..", ".kyyoyyok.", "kyyyyyyyyk", "koyyoyyyok", "kyyyyyyyyk", ".kkkkkkkk."]),
            meat: spr(&[
                "....kkkk..",
                "...koooork",
                "..kooroook",
                "..korooork",
                "..kooooork",
                ".kkkoookk.",
                "kwwk.kkk..",
                "kww.......",
            ]),
            coin: spr(&[".kkkk.", "kyyyyk", "kywyyk", "kyyyok", "kyyook", ".kkkk."]),
            gem: spr(&[".kkkkk.", "kmwmmmk", "kmmmmmk", ".kmmmk.", "..kmk..", "...k..."]),
            potion: spr(&["..kkk..", "...k...", "..kwk..", ".kbwbk.", "kbbbbbk", "kbcbbbk", "kbbbbbk", ".kkkkk."]),
            heart: spr(&[".rr.rr.", "rwrrrrr", "rrrrrrr", ".rrrrr.", "..rrr..", "...r..."]),
            big_heart: spr(&[
                ".kkk...kkk.",
                "krrrk.krrrk",
                "krwrrkrrrrk",
                "krwrrrrrrrk",
                "krrrrrrrrrk",
                ".krrrrrrrk.",
                "..krrrrrk..",
                "...krrrk...",
                "....krk....",
                ".....k.....",
            ]),
            hoard: spr(&[
                ".....kkkk.....",
                "....kyywyk....",
                "..kkkyyyykkk..",
                ".kyywyyyyyyok.",
                "kyyyyyoyywyyyk",
                "kyoyyyyyyyyyok",
                "kyyyyyyoyyyyyk",
                ".kkkkkkkkkkkk.",
            ]),
        }
    }
}

/// Generated boss: a mirrored random pixel creature with an outline, eyes and a core.
/// Returns one sprite per element (Fire, Ice, Storm, Earth) so shifting bosses can recolour.
pub fn gen_boss(seed: u32, w: usize, h: usize) -> Vec<Spr> {
    let mut r = Mul(seed);
    let hw = w / 2;
    let mut g = vec![vec!['.'; w]; h];
    for y in 0..h {
        for x in 0..hw {
            let cx = (hw - 1 - x) as f64 / hw as f64;
            let cy = (y as f64 - h as f64 / 2.0).abs() / (h as f64 / 2.0);
            if r.f() < 0.9 - cx * 0.55 - cy * 0.35 {
                let ch = if r.f() < 0.22 { 'b' } else { 'a' };
                g[y][x] = ch;
                g[y][w - 1 - x] = ch;
            }
        }
    }
    let cy = (h as f64 * 0.55) as usize;
    for dy in 0..2 {
        for dx in 0..2 {
            g[cy + dy][hw - 1 + dx] = 'r';
        }
    }
    let ey = (h as f64 * 0.3) as usize;
    for dy in 0..2 {
        g[ey + dy][hw - 3] = 'w';
        g[ey + dy][hw + 2] = 'w';
    }
    let filled = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && g[y as usize][x as usize] != '.';
    let mut rows: Vec<String> = Vec::new();
    for y in -1..=h as i32 {
        let mut row = String::new();
        for x in -1..=w as i32 {
            if filled(x, y) {
                row.push(g[y as usize][x as usize]);
            } else if filled(x - 1, y) || filled(x + 1, y) || filled(x, y - 1) || filled(x, y + 1) {
                row.push('k');
            } else {
                row.push('.');
            }
        }
        rows.push(row);
    }
    let refs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
    (0..4)
        .map(|e| {
            spr_with(&refs, move |ch| match ch {
                'a' => EL_MAIN[e],
                'b' => EL_LIGHT[e],
                'k' => rgb(0x100808),
                'r' => rgb(0xfc3c3c),
                'w' => WHITE,
                _ => pal(ch),
            })
        })
        .collect()
}

// ---------------------------------------------------------------- terrain
pub struct Theme {
    pub name: &'static str,
    pub floor: Sprite,
    pub wall: Sprite,
    pub map_col: u32,
}

fn grass(base: u32, tuft: u32, dot: u32) -> Sprite {
    let mut s = Sprite::new(16, 16);
    s.fill(0, 0, 16, 16, base);
    for &(x, y) in &[(2, 3), (9, 1), (12, 9), (5, 12), (13, 14)] {
        s.fill(x, y, 1, 2, tuft);
        s.fill(x + 1, y + 1, 1, 1, tuft);
    }
    s.fill(7, 7, 1, 1, dot);
    s.fill(1, 9, 1, 1, dot);
    s
}
fn flagstone(base: u32, line: u32, spot: u32) -> Sprite {
    let mut s = Sprite::new(16, 16);
    s.fill(0, 0, 16, 16, base);
    s.fill(0, 0, 16, 1, line);
    s.fill(0, 0, 1, 16, line);
    s.fill(0, 8, 8, 1, line);
    s.fill(8, 8, 1, 8, line);
    s.fill(4, 4, 1, 1, spot);
    s.fill(12, 3, 1, 1, spot);
    s.fill(11, 12, 2, 1, spot);
    s
}
fn bricks(base: u32, hi: u32, mortar: u32) -> Sprite {
    let mut s = Sprite::new(16, 16);
    s.fill(0, 0, 16, 16, base);
    s.fill(0, 7, 16, 1, mortar);
    s.fill(0, 15, 16, 1, mortar);
    for x in [7, 15] {
        s.fill(x, 0, 1, 7, mortar);
    }
    for x in [3, 11] {
        s.fill(x, 8, 1, 7, mortar);
    }
    s.fill(0, 0, 7, 1, hi);
    s.fill(8, 0, 7, 1, hi);
    s.fill(0, 8, 3, 1, hi);
    s.fill(4, 8, 7, 1, hi);
    s.fill(12, 8, 4, 1, hi);
    s
}

pub fn build_themes() -> Vec<Theme> {
    let forest = {
        let floor = grass(rgb(0x1c4c14), rgb(0x2c6c1c), rgb(0x48902c));
        let mut wall = floor.clone();
        wall.disc(8, 8, 7, rgb(0x042404));
        wall.disc(8, 7, 6, rgb(0x0c6c1c));
        wall.disc(6, 5, 2, rgb(0x48a838));
        wall.disc(10, 9, 1, rgb(0x2c8c2c));
        wall.fill(7, 14, 2, 2, rgb(0x6c3c10));
        Theme { name: "GREENWOOD", floor, wall, map_col: rgb(0x2c8c2c) }
    };
    let crypt = Theme {
        name: "OLD CRYPT",
        floor: flagstone(rgb(0x2c2c34), rgb(0x3c3c48), rgb(0x50505c)),
        wall: bricks(rgb(0x686878), rgb(0xa0a0b0), rgb(0x1c1c24)),
        map_col: rgb(0x8888a0),
    };
    let swamp = {
        let mut floor = Sprite::new(16, 16);
        floor.fill(0, 0, 16, 16, rgb(0x1c2c1c));
        floor.fill(3, 4, 4, 2, rgb(0x243c3c));
        floor.fill(10, 11, 5, 2, rgb(0x243c3c));
        floor.fill(12, 3, 1, 1, rgb(0x3c5c3c));
        floor.fill(5, 13, 1, 1, rgb(0x3c5c3c));
        let mut wall = floor.clone();
        wall.disc(8, 9, 7, rgb(0x101c10));
        wall.disc(8, 8, 6, rgb(0x3c5c3c));
        wall.disc(6, 6, 2, rgb(0x78a060));
        Theme { name: "MIREFEN", floor, wall, map_col: rgb(0x508050) }
    };
    let volcano = {
        let mut floor = flagstone(rgb(0x2c0c08), rgb(0x401410), rgb(0x6c2010));
        floor.fill(10, 5, 3, 1, rgb(0xfc7400));
        let mut wall = bricks(rgb(0x3c3030), rgb(0x6c5c5c), rgb(0x100808));
        for &(x, y) in &[(5, 10), (6, 11), (12, 3), (2, 4)] {
            wall.fill(x, y, 1, 1, rgb(0xfc5000));
        }
        Theme { name: "EMBERPEAK", floor, wall, map_col: rgb(0xc04020) }
    };
    let lair = Theme {
        name: "MONSTER LAIR",
        floor: flagstone(rgb(0x140c1c), rgb(0x201430), rgb(0x34204c)),
        wall: bricks(rgb(0x302040), rgb(0x5c4880), rgb(0x08040c)),
        map_col: rgb(0x5c4880),
    };
    let tower = Theme {
        name: "DARK TOWER",
        floor: flagstone(rgb(0x100000), rgb(0x200000), rgb(0x400000)),
        wall: bricks(rgb(0x500000), rgb(0x902020), rgb(0x100000)),
        map_col: rgb(0x902020),
    };
    vec![forest, crypt, swamp, volcano, lair, tower]
}
