#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! ELEMENTAL LEGENDS - SDL2 front end. Renders a 256x240 software framebuffer,
//! integer-scaled to the window (640x480 on the Anbernic RG35XX H).
mod audio;
mod game;
mod gfx;
mod snapshot;
mod sprites;
mod world;

use game::{Btn, Game, Input};
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use std::time::{Duration, Instant};

fn map_key(sc: Scancode) -> Option<Btn> {
    use Scancode::*;
    Some(match sc {
        Up | W => Btn::Up,
        Down | S => Btn::Down,
        Left | A => Btn::Left,
        Right | D => Btn::Right,
        Z | J | Space => Btn::Fire,
        X | K => Btn::Sub,
        C | L | LShift | RShift => Btn::Potion,
        Return | Escape | P => Btn::Start,
        M => Btn::Mute,
        _ => return None,
    })
}

fn map_button(b: Button) -> Option<Btn> {
    Some(match b {
        Button::DPadUp => Btn::Up,
        Button::DPadDown => Btn::Down,
        Button::DPadLeft => Btn::Left,
        Button::DPadRight => Btn::Right,
        Button::A | Button::X => Btn::Fire,
        Button::B => Btn::Sub,
        Button::Y | Button::LeftShoulder | Button::RightShoulder => Btn::Potion,
        Button::Start => Btn::Start,
        Button::Back => Btn::Mute,
        _ => return None,
    })
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--snapshot" || a == "--selftest") {
        let dir = args.get(i + 1).map(String::as_str).filter(|d| !d.starts_with("--"));
        let dir = if args[i] == "--snapshot" { Some(dir.unwrap_or("snapshots")) } else { dir };
        std::process::exit(snapshot::run(dir));
    }
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let controllers = sdl.game_controller().ok();
    let audio_sys = sdl.audio().ok();
    sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", "0");

    let fullscreen = args.iter().any(|a| a == "--fullscreen")
        || (cfg!(target_arch = "aarch64") && !args.iter().any(|a| a == "--windowed"));

    let mut wb = video.window("Elemental Legends", 960, 720);
    wb.position_centered().resizable();
    if fullscreen {
        wb.fullscreen_desktop();
    }
    let window = wb.build().map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut tex = tc
        .create_texture_streaming(PixelFormatEnum::ARGB8888, gfx::SW as u32, gfx::SH as u32)
        .map_err(|e| e.to_string())?;
    if fullscreen {
        sdl.mouse().show_cursor(false);
    }

    let mut pads: Vec<GameController> = Vec::new();
    let audio = audio_sys.as_ref().and_then(audio::open);
    let mut game = Game::new(audio);
    let mut scr = gfx::Screen::new();
    let mut input = Input::default();
    let mut events = sdl.event_pump()?;
    let step = Duration::from_micros(16_667);
    let mut last = Instant::now();
    let mut acc = Duration::ZERO;
    let mut back_held = false;

    'main: loop {
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'main,
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } => {
                    if let Some(b) = map_key(sc) {
                        input.set_key(b, true);
                    }
                }
                Event::KeyUp { scancode: Some(sc), .. } => {
                    if let Some(b) = map_key(sc) {
                        input.set_key(b, false);
                    }
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    if let Some(c) = controllers.as_ref().and_then(|g| g.open(which).ok()) {
                        pads.push(c);
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    if button == Button::Back {
                        back_held = true;
                    }
                    // SELECT + START quits (the usual handheld hotkey).
                    if button == Button::Start && back_held {
                        break 'main;
                    }
                    if let Some(b) = map_button(button) {
                        input.set_pad(b, true);
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    if button == Button::Back {
                        back_held = false;
                    }
                    if let Some(b) = map_button(button) {
                        input.set_pad(b, false);
                    }
                }
                Event::ControllerAxisMotion { axis, value, .. } => {
                    let v = value as i32;
                    match axis {
                        Axis::LeftX => {
                            input.set_axis(Btn::Left, v < -16000);
                            input.set_axis(Btn::Right, v > 16000);
                        }
                        Axis::LeftY => {
                            input.set_axis(Btn::Up, v < -16000);
                            input.set_axis(Btn::Down, v > 16000);
                        }
                        Axis::TriggerRight => input.set_axis(Btn::Fire, v > 12000),
                        Axis::TriggerLeft => input.set_axis(Btn::Sub, v > 12000),
                        _ => {}
                    }
                }
                Event::Window { win_event: WindowEvent::FocusLost, .. } => input.release_all(),
                _ => {}
            }
        }

        let now = Instant::now();
        acc += (now - last).min(Duration::from_millis(100));
        last = now;
        while acc >= step {
            game.update(&input);
            input.clear_pressed();
            acc -= step;
        }
        if game.quit {
            break;
        }

        game.draw(&mut scr);
        // SAFETY: a Vec<u32> is contiguous; ARGB8888 is a native-endian packed u32 format.
        let bytes = unsafe { std::slice::from_raw_parts(scr.px.as_ptr() as *const u8, scr.px.len() * 4) };
        tex.update(None, bytes, gfx::SW as usize * 4).map_err(|e| e.to_string())?;
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let (ww, wh) = canvas.output_size()?;
        let (gw, gh) = (gfx::SW as u32, gfx::SH as u32);
        let s = (ww / gw).min(wh / gh);
        let (dw, dh) = if s >= 1 {
            (gw * s, gh * s)
        } else {
            let f = (ww as f32 / gw as f32).min(wh as f32 / gh as f32);
            ((gw as f32 * f) as u32, (gh as f32 * f) as u32)
        };
        let dst = Rect::new(((ww - dw) / 2) as i32, ((wh - dh) / 2) as i32, dw.max(1), dh.max(1));
        canvas.copy(&tex, None, Some(dst))?;
        canvas.present();
        if acc < step {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    Ok(())
}
