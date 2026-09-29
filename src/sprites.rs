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

/// Perceived brightness, 0..2550.
fn lum(c: u32) -> u32 {
    ((c >> 16) & 255) * 3 + ((c >> 8) & 255) * 6 + (c & 255)
}
/// Near-black outline pixels.
fn is_line(c: u32) -> bool {
    c != 0 && lum(c) < 500
}

/// SNES-style polish: light from the top-left, shadow on the lower-right, and a soft
/// outline tinted from the sprite's own colours. Adds a 1px transparent border.
pub fn polish(src: &Sprite) -> Sprite {
    let (w, h) = (src.w + 2, src.h + 2);
    let mut out = Sprite::new(w, h);
    let get = |x: i32, y: i32| if x < 0 || y < 0 || x >= src.w || y >= src.h { 0 } else { src.px[(y * src.w + x) as usize] };
    let edge = |c: u32| c == 0 || is_line(c);
    for y in 0..src.h {
        for x in 0..src.w {
            let c = get(x, y);
            if c == 0 {
                continue;
            }
            let mut v = c;
            if !is_line(c) {
                if edge(get(x, y - 1)) {
                    v = mix(v, WHITE, 0.28);
                } else if edge(get(x - 1, y)) {
                    v = mix(v, WHITE, 0.14);
                }
                if edge(get(x, y + 1)) {
                    v = mix(v, BLACK, 0.30);
                } else if edge(get(x + 1, y)) {
                    v = mix(v, BLACK, 0.16);
                }
            }
            out.px[((y + 1) * w + x + 1) as usize] = v;
        }
    }
    let snap = out.clone();
    for y in 0..h {
        for x in 0..w {
            if snap.px[(y * w + x) as usize] != 0 {
                continue;
            }
            let mut nb = 0u32;
            for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let c = snap.px[(ny * w + nx) as usize];
                    if c != 0 {
                        nb = c;
                        break;
                    }
                }
            }
            // Pixels already outlined in black keep their original crisp edge.
            if nb != 0 && !is_line(nb) {
                out.px[(y * w + x) as usize] = mix(nb, rgb(0x080410), 0.78);
            }
        }
    }
    out
}

pub fn spr_with(rows: &[&str], p: impl Fn(char) -> u32) -> Spr {
    let img = polish(&Sprite::from_rows(rows, &p));
    let map = |f: &dyn Fn(u32) -> u32| {
        let mut s = img.clone();
        for px in s.px.iter_mut() {
            if *px != 0 {
                *px = f(*px);
            }
        }
        s
    };
    Spr {
        white: map(&|_| WHITE),
        ice: map(&|c| if is_line(c) { rgb(0x2c78c8) } else { mix(rgb(0xc4ecfc), c, 0.15) }),
        frost: map(&|c| mix(c, rgb(0xa4e4fc), 0.45)),
        img,
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

/// Four-frame walk cycle [element][frame]: idle, left step, idle, right step.
/// Only the feet row differs, so the robe and staff stay steady.
fn mage_frames(base: &[&str; 16], step_a: &'static str, step_b: &'static str) -> Vec<Vec<Spr>> {
    let with_feet = |f: &'static str| -> Vec<&str> {
        let mut v: Vec<&str> = base[..15].to_vec();
        v.push(f);
        v
    };
    let (a, b) = (with_feet(step_a), with_feet(step_b));
    (0..5)
        .map(|e| vec![spr_with(base, tint(e)), spr_with(&a, tint(e)), spr_with(base, tint(e)), spr_with(&b, tint(e))])
        .collect()
}

pub struct Sprites {
    /// [element][walk frame]
    pub mage_d: Vec<Vec<Spr>>,
    pub mage_u: Vec<Vec<Spr>>,
    pub mage_s: Vec<Vec<Spr>>,
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
            mage_d: mage_frames(&MAGE_D, "..kkk.....kk....", "....kk.....kkk.."),
            mage_u: mage_frames(&MAGE_U, "..kkk.....kk....", "....kk.....kkk.."),
            mage_s: mage_frames(&MAGE_S, "..kkk....kkk....", "....kkkkk......."),
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
    /// Floor variants (0-1 plain, 2-3 with details); `floor` is variant 0.
    pub floors: Vec<Sprite>,
    pub walls: Vec<Sprite>,
    pub floor: Sprite,
    pub map_col: u32,
    /// Masonry walls get a lit top edge and a shaded front face; trees and boulders don't.
    pub bevel: bool,
}

fn theme(name: &'static str, floors: Vec<Sprite>, walls: Vec<Sprite>, map_col: u32, bevel: bool) -> Theme {
    Theme { name, floor: floors[0].clone(), floors, walls, map_col, bevel }
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / 16_777_216.0
    }
}
fn shade(c: u32, f: f32) -> u32 {
    if f >= 0.0 {
        mix(c, WHITE, f)
    } else {
        mix(c, BLACK, -f)
    }
}
/// Subtle per-pixel light/dark noise so surfaces aren't flat.
fn texture(s: &mut Sprite, seed: u32, amt: f32) {
    let mut r = Lcg(seed);
    for px in s.px.iter_mut() {
        if *px != 0 {
            *px = shade(*px, (r.next() - 0.5) * amt);
        }
    }
}

fn grass(base: u32, seed: u32, detail: u8) -> Sprite {
    let mut s = Sprite::new(16, 16);
    s.fill(0, 0, 16, 16, base);
    texture(&mut s, seed, 0.22);
    let mut r = Lcg(seed ^ 0x9e37);
    let (lt, dk) = (shade(base, 0.25), shade(base, -0.3));
    for _ in 0..5 {
        let x = (r.next() * 13.0) as i32 + 1;
        let y = (r.next() * 12.0) as i32 + 2;
        s.fill(x, y, 1, 2, lt);
        s.fill(x + 1, y - 1, 1, 2, lt);
        s.fill(x, y + 2, 2, 1, dk);
    }
    match detail {
        1 => {
            for (i, &(x, y)) in [(4, 5), (11, 3), (8, 11)].iter().enumerate() {
                let petal = [rgb(0xf878f8), rgb(0xfce040), WHITE][i];
                s.fill(x - 1, y, 3, 1, petal);
                s.fill(x, y - 1, 1, 3, petal);
                s.fill(x, y, 1, 1, rgb(0xfcbc3c));
                s.fill(x, y + 2, 1, 1, dk);
            }
        }
        2 => {
            s.disc(10, 10, 2, rgb(0x747468));
            s.fill(9, 9, 2, 1, rgb(0xa8a89c));
            s.fill(9, 12, 3, 1, dk);
            s.disc(4, 5, 1, rgb(0x5c5c54));
        }
        3 => {
            s.blend(3, 3, 10, 9, shade(base, -0.4), 0.35);
            for &(x, y) in &[(5, 5), (8, 7), (10, 4), (6, 9)] {
                s.fill(x, y, 2, 2, shade(base, 0.35));
            }
        }
        _ => {}
    }
    s
}
fn tree(floor: &Sprite, seed: u32) -> Sprite {
    let mut s = floor.clone();
    s.blend(1, 11, 14, 5, BLACK, 0.4);
    s.disc(8, 8, 7, rgb(0x042404));
    s.disc(8, 7, 6, rgb(0x0c5418));
    s.disc(7, 6, 5, rgb(0x187220));
    s.disc(6, 5, 3, rgb(0x2c902c));
    s.disc(5, 4, 1, rgb(0x6cc048));
    let mut r = Lcg(seed);
    for _ in 0..10 {
        let (x, y) = (3 + (r.next() * 10.0) as i32, 2 + (r.next() * 10.0) as i32);
        let c = if r.next() < 0.5 { rgb(0x0c4414) } else { rgb(0x48a838) };
        if (x - 8) * (x - 8) + (y - 7) * (y - 7) < 30 {
            s.fill(x, y, 1, 1, c);
        }
    }
    s.fill(7, 13, 2, 3, rgb(0x5c3410));
    s.fill(7, 13, 1, 3, rgb(0x8c5020));
    s
}
/// Irregular stone slabs with bevelled edges and grout.
fn flagstone(base: u32, seed: u32, detail: u8) -> Sprite {
    let mut s = Sprite::new(16, 16);
    let grout = shade(base, -0.45);
    s.fill(0, 0, 16, 16, grout);
    let mut r = Lcg(seed);
    let stones = [(0, 0, 9, 8), (9, 0, 7, 8), (0, 8, 5, 8), (5, 8, 11, 8)];
    for &(x, y, w, h) in &stones {
        let c = shade(base, r.next() * 0.18 - 0.09);
        s.fill(x, y, w - 1, h - 1, c);
        s.fill(x, y, w - 1, 1, shade(c, 0.16));
        s.fill(x, y, 1, h - 1, shade(c, 0.08));
        s.fill(x, y + h - 2, w - 1, 1, shade(c, -0.18));
        s.fill(x + w - 2, y, 1, h - 1, shade(c, -0.1));
    }
    texture(&mut s, seed ^ 0x51, 0.12);
    match detail {
        1 => {
            for &(x, y) in &[(3, 2), (4, 3), (5, 3), (6, 4), (6, 5), (12, 10), (11, 11), (11, 12)] {
                s.fill(x, y, 1, 1, grout);
            }
        }
        2 => {
            for &(x, y) in &[(1, 13), (2, 12), (3, 13), (13, 1), (14, 2)] {
                s.fill(x, y, 2, 1, rgb(0x2c6c1c));
            }
        }
        3 => {
            s.disc(11, 4, 1, shade(base, -0.3));
            s.fill(2, 10, 2, 2, shade(base, 0.2));
        }
        _ => {}
    }
    s
}
/// Bricks with per-brick colour variation and a bevel on every brick.
fn bricks(base: u32, seed: u32) -> Sprite {
    let mut s = Sprite::new(16, 16);
    s.fill(0, 0, 16, 16, shade(base, -0.55));
    let mut r = Lcg(seed);
    let rows: [(i32, &[(i32, i32)]); 2] = [(0, &[(0, 7), (8, 7)]), (8, &[(-4, 7), (4, 7), (12, 7)])];
    for (y, bs) in rows {
        for &(x, w) in bs {
            let c = shade(base, r.next() * 0.2 - 0.1);
            s.fill(x, y, w, 7, c);
            s.fill(x, y, w, 1, shade(c, 0.22));
            s.fill(x, y, 1, 7, shade(c, 0.1));
            s.fill(x, y + 6, w, 1, shade(c, -0.25));
            s.fill(x + w - 1, y, 1, 7, shade(c, -0.15));
        }
    }
    texture(&mut s, seed ^ 0x77, 0.1);
    s
}
fn mud(seed: u32, detail: u8) -> Sprite {
    let mut s = Sprite::new(16, 16);
    let base = rgb(0x1c2c1c);
    s.fill(0, 0, 16, 16, base);
    texture(&mut s, seed, 0.25);
    match detail {
        1 | 2 => {
            let (x, y) = if detail == 1 { (5, 5) } else { (10, 11) };
            s.fill(x - 3, y - 1, 7, 3, rgb(0x1c3c44));
            s.fill(x - 2, y - 2, 5, 5, rgb(0x1c3c44));
            s.fill(x - 1, y - 1, 3, 1, rgb(0x4c7c8c));
        }
        3 => {
            for &x in &[4, 7, 11] {
                s.fill(x, 5, 1, 7, rgb(0x5c7c3c));
                s.fill(x - 1, 4, 1, 2, rgb(0x8ca84c));
            }
        }
        _ => {
            s.fill(12, 3, 1, 1, rgb(0x3c5c3c));
            s.fill(5, 13, 1, 1, rgb(0x3c5c3c));
        }
    }
    s
}
fn boulder(floor: &Sprite) -> Sprite {
    let mut s = floor.clone();
    s.blend(1, 11, 14, 5, BLACK, 0.4);
    s.disc(8, 9, 7, rgb(0x101c10));
    s.disc(8, 8, 6, rgb(0x34503a));
    s.disc(7, 7, 4, rgb(0x4c6c4c));
    s.disc(6, 6, 2, rgb(0x78a060));
    s.fill(9, 11, 4, 1, rgb(0x1c2c1c));
    s
}
fn basalt(seed: u32, detail: u8) -> Sprite {
    let mut s = flagstone(rgb(0x2c0c08), seed, if detail == 1 { 1 } else { 0 });
    if detail >= 2 {
        let glow = if detail == 2 { rgb(0xfc7400) } else { rgb(0xd84000) };
        for &(x, y) in &[(9, 4), (10, 5), (11, 5), (12, 6), (3, 11), (4, 12)] {
            s.fill(x, y, 1, 1, glow);
        }
    }
    s
}
fn floors(f: impl Fn(u32, u8) -> Sprite, seed: u32) -> Vec<Sprite> {
    (0..4).map(|v| f(seed + v as u32 * 31, if v < 2 { 0 } else { v as u8 - 1 })).collect()
}
fn wall_set(f: impl Fn(u32) -> Sprite, seed: u32) -> Vec<Sprite> {
    (0..3).map(|v| f(seed + v * 17)).collect()
}
fn with_specks(mut s: Sprite, spots: &[(i32, i32)], c: u32) -> Sprite {
    for &(x, y) in spots {
        s.fill(x, y, 2, 1, c);
    }
    s
}

pub fn build_themes() -> Vec<Theme> {
    let g = rgb(0x1c4c14);
    let forest_floors: Vec<Sprite> = (0..4).map(|v| grass(g, 11 + v * 13, [0, 0, 1, 2][v as usize])).collect();
    let forest_walls = vec![tree(&forest_floors[0], 5), tree(&forest_floors[1], 9)];
    let forest = theme("GREENWOOD", forest_floors, forest_walls, rgb(0x2c8c2c), false);
    let crypt = theme(
        "OLD CRYPT",
        floors(|s, d| flagstone(rgb(0x34343e), s, d), 101),
        wall_set(|s| bricks(rgb(0x686878), s), 7),
        rgb(0x8888a0),
        true,
    );
    let swamp_floors = floors(mud, 211);
    let swamp_walls = vec![boulder(&swamp_floors[0]), boulder(&swamp_floors[1])];
    let swamp = theme("MIREFEN", swamp_floors, swamp_walls, rgb(0x508050), false);
    let volcano = theme(
        "EMBERPEAK",
        floors(basalt, 301),
        wall_set(|s| with_specks(bricks(rgb(0x3c3030), s), &[(5, 10), (12, 3)], rgb(0xfc5000)), 13),
        rgb(0xc04020),
        true,
    );
    // Dungeon interiors (and their boss arenas), indexed 3 + dungeon number.
    let shrine = theme(
        "OVERGROWN SHRINE",
        floors(|s, d| flagstone(rgb(0x243418), s, d), 401),
        wall_set(|s| with_specks(bricks(rgb(0x4c5c40), s), &[(1, 2), (9, 9), (13, 1)], rgb(0x2c8c2c)), 19),
        rgb(0x48a838),
        true,
    );
    let crypt_d = theme(
        "UNDERGROUND CRYPT",
        floors(|s, d| flagstone(rgb(0x20202a), s, d), 501),
        wall_set(|s| bricks(rgb(0x3c3c4c), s), 23),
        rgb(0x7878a0),
        true,
    );
    let castle = theme(
        "RUINED CASTLE",
        floors(|s, d| flagstone(rgb(0x3a3020), s, d), 601),
        wall_set(|s| bricks(rgb(0x7c6c50), s), 29),
        rgb(0xa8987c),
        true,
    );
    let fortress = theme(
        "DRAGON FORTRESS",
        floors(|s, d| basalt(s, d), 701),
        wall_set(|s| with_specks(bricks(rgb(0x2c2020), s), &[(4, 11), (12, 2)], rgb(0xfc6000)), 31),
        rgb(0xc04020),
        true,
    );
    let sanctuary = theme(
        "FORGOTTEN SANCTUARY",
        floors(|s, d| flagstone(rgb(0x1c1c3a), s, d), 801),
        wall_set(|s| bricks(rgb(0x3c3c6c), s), 37),
        rgb(0x7878d8),
        true,
    );
    let tower = theme(
        "DARK TOWER",
        floors(|s, d| flagstone(rgb(0x1c1028), s, d), 901),
        wall_set(|s| bricks(rgb(0x302040), s), 41),
        rgb(0x9818a8),
        true,
    );
    vec![forest, crypt, swamp, volcano, shrine, crypt_d, castle, fortress, sanctuary, tower]
}
