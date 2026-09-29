//! Headless test mode: `elementallegends --snapshot <dir>` plays a scripted run
//! without opening a window and writes PNG screenshots, so rendering and game
//! flow can be checked on machines (or CI) with no display.
use crate::game::{Btn, Game, Input};
use crate::gfx::{Screen, H, W};
use std::path::Path;

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// Minimal PNG writer (stored/uncompressed deflate blocks).
fn write_png(path: &Path, px: &[u32], w: usize, h: usize) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((w * 3 + 1) * h);
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            let p = px[y * w + x];
            raw.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    let mut z = vec![0x78, 0x01];
    for (i, chunk) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        let n = chunk.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |ty: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut c = ty.to_vec();
        c.extend_from_slice(data);
        out.extend_from_slice(&c);
        out.extend_from_slice(&crc32(&c).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    std::fs::write(path, out)
}

struct Runner {
    game: Game,
    scr: Screen,
    input: Input,
    dir: std::path::PathBuf,
}

impl Runner {
    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.game.update(&self.input);
            self.input.clear_pressed();
        }
    }
    fn tap(&mut self, b: Btn) {
        self.input.set_key(b, true);
        self.frames(1);
        self.input.set_key(b, false);
        self.frames(1);
    }
    fn hold(&mut self, b: Btn, n: usize) {
        self.input.set_key(b, true);
        self.frames(n);
        self.input.set_key(b, false);
    }
    fn shot(&mut self, name: &str) {
        self.game.draw(&mut self.scr);
        let p = self.dir.join(format!("{name}.png"));
        write_png(&p, &self.scr.px, W as usize, H as usize).expect("write png");
        println!("wrote {}", p.display());
    }
}

pub fn run(dir: &str) {
    std::fs::create_dir_all(dir).expect("create snapshot dir");
    let mut r = Runner { game: Game::new(None), scr: Screen::new(), input: Input::default(), dir: dir.into() };
    r.game.debug_no_save();
    r.frames(30);
    r.shot("01_title");
    r.game.debug_force_new_game_menu();
    r.tap(Btn::Start);
    r.frames(10);
    r.tap(Btn::Right);
    r.tap(Btn::Right);
    r.frames(20);
    r.shot("02_choose");
    r.tap(Btn::Start);
    r.frames(200);
    r.shot("03_intro");
    r.tap(Btn::Start);
    r.frames(20);
    r.shot("04_start_room_shop");
    // Walk east into the next screen and fight.
    r.hold(Btn::Up, 22);
    r.hold(Btn::Right, 140);
    r.frames(60);
    println!("after walk: {}", r.game.debug_summary());
    r.shot("05_next_room");
    r.input.set_key(Btn::Fire, true);
    r.frames(70);
    r.input.set_key(Btn::Fire, false);
    r.shot("06_combat");
    r.tap(Btn::Sub);
    r.frames(6);
    r.shot("07_spell");
    r.tap(Btn::Start);
    r.frames(2);
    r.shot("08_map");
    r.tap(Btn::Start);
    for n in [1usize, 5] {
        r.game.debug_lair(n);
        r.frames(100);
        r.shot(&format!("09_lair{n}_intro"));
        r.hold(Btn::Up, 2);
        r.input.set_key(Btn::Fire, true);
        for _ in 0..420 {
            let dx = r.game.debug_boss_dx();
            r.input.set_key(Btn::Left, dx < -4.0);
            r.input.set_key(Btn::Right, dx > 4.0);
            r.frames(1);
        }
        r.input.set_key(Btn::Left, false);
        r.input.set_key(Btn::Right, false);
        r.hold(Btn::Up, 1);
        println!("lair {n}: {}", r.game.debug_summary());
        r.input.set_key(Btn::Fire, false);
        r.shot(&format!("10_lair{n}_fight"));
    }
    println!("{}", r.game.debug_summary());
}
