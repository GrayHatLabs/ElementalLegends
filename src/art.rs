//! Native-resolution art (screen pixels, drawn unscaled by the world camera),
//! embedded at build time by `scripts/import_art.py` into `art_gen.rs`.
//! Anything missing here falls back to the code-drawn sprites in `sprites.rs`.
use crate::art_gen::{SHEETS, TERRAIN};
use crate::gfx::Sprite;

pub struct AnimDef {
    pub name: &'static str,
    pub row: u32,
    pub frames: u32,
    pub fps: u32,
}

pub struct SheetDef {
    pub name: &'static str,
    pub cell: (i32, i32),
    pub data: &'static [u8],
    pub anims: &'static [AnimDef],
}

pub struct TerrainDef {
    pub theme: usize,
    pub wall: Option<&'static [u8]>,
    pub water: Option<&'static [u8]>,
    pub deco: &'static [&'static [u8]],
}

/// Decode a blob written by import_art.py: u32 w, u32 h, then w*h ARGB pixels.
pub fn decode(b: &[u8]) -> Sprite {
    let rd = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (w, h) = (rd(0) as i32, rd(4) as i32);
    let mut s = Sprite::new(w, h);
    for (i, p) in s.px.iter_mut().enumerate() {
        *p = rd(8 + i * 4);
    }
    s
}

fn crop(s: &Sprite, x: i32, y: i32, w: i32, h: i32) -> Sprite {
    let mut out = Sprite::new(w, h);
    for yy in 0..h {
        for xx in 0..w {
            let (sx, sy) = (x + xx, y + yy);
            if sx < s.w && sy < s.h {
                out.px[(yy * w + xx) as usize] = s.px[(sy * s.w + sx) as usize];
            }
        }
    }
    out
}

pub struct Anim {
    pub frames: Vec<Sprite>,
    pub fps: u32,
}

impl Anim {
    /// Frame for a counter that advances once per game frame (60 fps).
    pub fn at(&self, ticks: u32) -> &Sprite {
        let i = (ticks * self.fps / 60) as usize % self.frames.len();
        &self.frames[i]
    }
}

pub struct Sheet {
    pub cell: (i32, i32),
    anims: Vec<(&'static str, Anim)>,
}

impl Sheet {
    pub fn anim(&self, name: &str) -> Option<&Anim> {
        self.anims.iter().find(|a| a.0 == name).map(|a| &a.1)
    }
}

/// A 16-tile corner (Wang) tileset. Tile index = NW*8 + NE*4 + SW*2 + SE with
/// 1 = the "upper" terrain (wall for wall sets, floor for water sets).
pub struct Wang {
    pub size: i32,
    pub tiles: Vec<Sprite>,
}

impl Wang {
    fn from_atlas(s: &Sprite) -> Self {
        let size = s.w / 4;
        let tiles = (0..16).map(|k| crop(s, (k % 4) * size, (k / 4) * size, size, size)).collect();
        Wang { size, tiles }
    }
}

pub struct TerrainArt {
    pub wall: Option<Wang>,
    pub water: Option<Wang>,
    pub deco: Vec<Sprite>,
}

pub struct Art {
    sheets: Vec<(&'static str, Sheet)>,
    terrain: Vec<(usize, TerrainArt)>,
}

impl Art {
    pub fn load() -> Self {
        let sheets = SHEETS
            .iter()
            .map(|d| {
                let img = decode(d.data);
                let (cw, ch) = d.cell;
                let anims = d
                    .anims
                    .iter()
                    .map(|a| {
                        let frames = (0..a.frames as i32).map(|f| crop(&img, f * cw, a.row as i32 * ch, cw, ch)).collect();
                        (a.name, Anim { frames, fps: a.fps.max(1) })
                    })
                    .collect();
                (d.name, Sheet { cell: d.cell, anims })
            })
            .collect();
        let terrain = TERRAIN
            .iter()
            .map(|t| {
                let wall = t.wall.map(|b| Wang::from_atlas(&decode(b)));
                let water = t.water.map(|b| Wang::from_atlas(&decode(b)));
                let deco = t.deco.iter().map(|b| decode(b)).collect();
                (t.theme, TerrainArt { wall, water, deco })
            })
            .collect();
        Art { sheets, terrain }
    }

    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.0 == name).map(|s| &s.1)
    }

    pub fn terrain(&self, theme: usize) -> Option<&TerrainArt> {
        self.terrain.iter().find(|t| t.0 == theme).map(|t| &t.1)
    }
}
