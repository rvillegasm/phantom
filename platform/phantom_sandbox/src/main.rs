use phantom::nes::bus::Bus;
use phantom::nes::cartridge::Rom;
use phantom::nes::cpu::Cpu;
use phantom::nes::render::frame::Frame;
use phantom::nes::ppu::Ppu;
use phantom::nes::render;
use phantom::nes::joypad;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;

use std::collections::HashMap;

fn main() {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Phantom NES", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    // Load game
    let raw_rom = std::fs::read("pacman.nes").unwrap();
    let rom = Rom::new(&raw_rom).unwrap();

    let mut frame = Frame::new();

    let mut keymap = HashMap::new();
    keymap.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    keymap.insert(Keycode::Up, joypad::JoypadButton::UP);
    keymap.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    keymap.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    keymap.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    keymap.insert(Keycode::Return, joypad::JoypadButton::START);
    keymap.insert(Keycode::A, joypad::JoypadButton::BUTTON_A);
    keymap.insert(Keycode::S, joypad::JoypadButton::BUTTON_B);

    // Game cycle logic
    let bus = Bus::new(rom, move |ppu: &Ppu, joypad: &mut joypad::Joypad| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data(), 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        handle_user_input(joypad, &keymap, &mut event_pump);
    });

    let mut cpu = Cpu::new(bus);

    cpu.reset();
    cpu.run();
}

fn handle_user_input(joypad: &mut joypad::Joypad, keymap: &HashMap<Keycode, joypad::JoypadButton>, event_pump: &mut EventPump) {
    event_pump.poll_iter().for_each(|event| {
        match event {
            Event::Quit { .. }
            | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => std::process::exit(0),
            Event::KeyDown { keycode, .. } => {
                if let Some(joypad_button) = keymap.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                    joypad.set_button_status(*joypad_button, true);
                }
            }
            Event::KeyUp { keycode, .. } => {
                if let Some(joypad_button) = keymap.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                    joypad.set_button_status(*joypad_button, false);
                }
            }
            _ => { /* Do Nothing */ }
        }
    });
}

