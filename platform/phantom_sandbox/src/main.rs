use phantom::nes::memory::Memory;
use phantom::nes::cpu::Cpu;
use phantom::nes::bus::Bus;
use phantom::nes::cartridge::Rom;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;
use rand::Rng;


fn main() {
    // For it to work, change the cpu impl so that the program rom is loaded into memory position
    // 0x0600 onwards, instead of 0x8000 as it should usually be. (because it is a simple test rom).

    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Snake Game", (32.0 * 10.0) as u32, (32.0 * 10.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 32, 32)
        .unwrap();

    // Load game
    let raw_rom = std::fs::read("snake.nes").unwrap();
    println!("Opened file snake.nes");
    let rom = Rom::new(&raw_rom).unwrap();

    let bus = Bus::new(rom);
    let mut cpu = Cpu::new(bus);
    cpu.reset();

    let mut screen_state = [0 as u8; 32 * 3 * 32];
    let mut rng = rand::thread_rng();

    cpu.run_with_callback(move |cpu| {
        handle_user_input(cpu, &mut event_pump);
        cpu.mem_write(0xFE, rng.gen_range(1..16));

        if read_screen_state(cpu, &mut screen_state) {
            texture.update(None, &screen_state, 32 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
        }

        ::std::thread::sleep(std::time::Duration::new(0, 70_000));
    });

    fn handle_user_input(cpu: &mut Cpu, event_pump: &mut EventPump) {
        event_pump.poll_iter().for_each(|event| {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    std::process::exit(0);
                }
                Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                    cpu.mem_write(0xFF, 0x77);
                }
                Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                    cpu.mem_write(0xFF, 0x73);
                },
                Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                    cpu.mem_write(0xFF, 0x61);
                },
                Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                    cpu.mem_write(0xFF, 0x64);
                }
                _ => {/* do nothing */}
            }
        });
    }

    fn color(byte: u8) -> Color {
        // Just black and white matter
        match byte {
            0 => sdl2::pixels::Color::BLACK,
            1 => sdl2::pixels::Color::WHITE,
            2 | 9 => sdl2::pixels::Color::GREY,
            3 | 10 => sdl2::pixels::Color::RED,
            4 | 11 => sdl2::pixels::Color::GREEN,
            5 | 12 => sdl2::pixels::Color::BLUE,
            6 | 13 => sdl2::pixels::Color::MAGENTA,
            7 | 14 => sdl2::pixels::Color::YELLOW,
            _ => sdl2::pixels::Color::CYAN,
        }
    }

    fn read_screen_state(cpu: &Cpu, frame: &mut [u8; 32 * 3 * 32]) -> bool {
        let mut frame_idx = 0;
        let mut update_required = false;

        for i in 0x0200..0x600 {
            let color_idx = cpu.mem_read(i as u16);
            let (b1, b2, b3) = color(color_idx).rgb();
            if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
                frame[frame_idx] = b1;
                frame[frame_idx + 1] = b2;
                frame[frame_idx + 2] = b3;
                update_required = true;
            }
            frame_idx += 3;
        }
        update_required
    }
}
