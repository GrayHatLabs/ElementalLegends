// Pixel art. Sprites with 'a' (main) / 'b' (light) / 'j' (glow) pixels are
// palette-swapped per element, so one drawing gives fire, ice, storm, earth
// and neutral versions.
use crate::gfx::*;

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

/// A sprite plus its hit-flash (white), frozen (ice) and chilled (frost-tinted) variants.
pub struct Spr {
    pub img: Sprite,
    pub white: Sprite,
    #[allow(dead_code)]
    pub ice: Sprite,
    pub frost: Sprite,
}

pub fn spr_with(rows: &[&str], p: impl Fn(char) -> u32) -> Spr {
    let img = Sprite::from_rows(rows, &p);
    let mut frost = img.clone();
    for px in frost.px.iter_mut() {
        if *px != 0 {
            *px = mix(*px, rgb(0xa4e4fc), 0.45);
        }
    }
    Spr {
        img,
        white: Sprite::from_rows(rows, |_| WHITE),
        ice: Sprite::from_rows(rows, |c| if c == 'k' { rgb(0x2c78c8) } else { rgb(0xc4ecfc) }),
        frost,
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
    pub key: Spr,
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
            key: spr(&[
                ".kkk....",
                "kyyyk...",
                "kykyk...",
                "kyyykkkk",
                ".kkkyyyk",
                "....kykk",
                "....kyk.",
                ".....k..",
            ]),
        }
    }
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
    // Dungeon interiors (and their boss arenas), indexed 3 + dungeon number.
    let shrine = {
        let mut floor = flagstone(rgb(0x1c2c14), rgb(0x2c3c1c), rgb(0x3c5c24));
        floor.fill(2, 10, 3, 1, rgb(0x2c6c1c));
        let mut wall = bricks(rgb(0x4c5c40), rgb(0x7c9068), rgb(0x141c10));
        for &(x, y) in &[(1, 2), (2, 3), (9, 9), (10, 10), (13, 1)] {
            wall.fill(x, y, 2, 1, rgb(0x2c8c2c));
        }
        Theme { name: "OVERGROWN SHRINE", floor, wall, map_col: rgb(0x48a838) }
    };
    let crypt_d = Theme {
        name: "UNDERGROUND CRYPT",
        floor: flagstone(rgb(0x18181c), rgb(0x24242c), rgb(0x34343c)),
        wall: bricks(rgb(0x3c3c4c), rgb(0x60607c), rgb(0x0c0c10)),
        map_col: rgb(0x7878a0),
    };
    let castle = Theme {
        name: "RUINED CASTLE",
        floor: flagstone(rgb(0x2c2418), rgb(0x3c3020), rgb(0x54442c)),
        wall: bricks(rgb(0x7c6c50), rgb(0xa8987c), rgb(0x241c10)),
        map_col: rgb(0xa8987c),
    };
    let fortress = {
        let mut floor = flagstone(rgb(0x1c0c08), rgb(0x2c140c), rgb(0x4c1c10));
        floor.fill(11, 5, 3, 1, rgb(0xd84000));
        let mut wall = bricks(rgb(0x2c2020), rgb(0x544444), rgb(0x080404));
        for &(x, y) in &[(4, 11), (5, 12), (12, 2), (1, 5)] {
            wall.fill(x, y, 1, 1, rgb(0xfc6000));
        }
        Theme { name: "DRAGON FORTRESS", floor, wall, map_col: rgb(0xc04020) }
    };
    let sanctuary = Theme {
        name: "FORGOTTEN SANCTUARY",
        floor: flagstone(rgb(0x14142c), rgb(0x1c1c3c), rgb(0x3c3c7c)),
        wall: bricks(rgb(0x3c3c6c), rgb(0x7878b8), rgb(0x0c0c1c)),
        map_col: rgb(0x7878d8),
    };
    let tower = Theme {
        name: "DARK TOWER",
        floor: flagstone(rgb(0x140c1c), rgb(0x201430), rgb(0x34204c)),
        wall: bricks(rgb(0x302040), rgb(0x5c4880), rgb(0x08040c)),
        map_col: rgb(0x9818a8),
    };
    vec![forest, crypt, swamp, volcano, shrine, crypt_d, castle, fortress, sanctuary, tower]
}
