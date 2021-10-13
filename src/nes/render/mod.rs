pub mod frame;
pub mod palette;

use crate::nes::ppu::Ppu;
use crate::nes::render::frame::Frame;

pub fn render(ppu: &Ppu, frame: &mut Frame) {
    let bank = ppu.control_register_background_pattern_address();

    // Background
    for i in 0..0x03C0 {
        let tile = ppu.read_vram_at(i) as u16;
        let tile_column = i % 32;
        let tile_row = i / 32;
        let tile = ppu.chr_rom_slice(
            (bank + tile * 16) as usize,
            (bank + tile * 16 + 15) as usize,
        );
        let palette = background_pallet(ppu, tile_column, tile_row);

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALETTE[palette[0] as usize],
                    1 => palette::SYSTEM_PALETTE[palette[1] as usize],
                    2 => palette::SYSTEM_PALETTE[palette[2] as usize],
                    3 => palette::SYSTEM_PALETTE[palette[3] as usize],
                    _ => panic!("RGB system palette for background could not be calculated"),
                };
                frame.set_pixel(tile_column * 8 + x, tile_row * 8 + y, rgb)
            }
        }
    }

    // Sprites
    for i in (0..ppu.oam_data_size()).step_by(4).rev() {
        let tile_idx = ppu.read_oam_data_at(i + 1) as u16;
        let tile_x = ppu.read_oam_data_at(i + 3) as usize;
        let tile_y = ppu.read_oam_data_at(i) as usize;

        let flip_vertical = if ppu.read_oam_data_at(i + 2) >> 7 & 1 == 1 {
            true
        } else {
            false
        };

        let flip_horizontal = if ppu.read_oam_data_at(i + 2) >> 6 & 1 == 1 {
            true
        } else {
            false
        };

        let palette_idx = ppu.read_oam_data_at(i + 2) & 0b11;
        let sprite_palette = sprite_palette(ppu, palette_idx);
        let bank = ppu.control_register_sprite_pattern_address();

        let tile = ppu.chr_rom_slice(
            (bank + tile_idx * 16) as usize,
            (bank + tile_idx * 16 + 15) as usize,
        );

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];

            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => continue, // Transparent pixel - Skip coloring
                    1 => palette::SYSTEM_PALETTE[sprite_palette[1] as usize],
                    2 => palette::SYSTEM_PALETTE[sprite_palette[2] as usize],
                    3 => palette::SYSTEM_PALETTE[sprite_palette[3] as usize],
                    _ => panic!("RGB system palette for sprite could not be calculated"),
                };

                match (flip_horizontal, flip_vertical) {
                    (false, false) => frame.set_pixel(tile_x + x, tile_y + y, rgb),
                    (true, false) => frame.set_pixel(tile_x + 7 - x, tile_y + y, rgb),
                    (false, true) => frame.set_pixel(tile_x + x, tile_y + 7 - y, rgb),
                    (true, true) => frame.set_pixel(tile_x + 7 - x, tile_y + 7 - y, rgb),
                }
            }
        }
    }
}

fn background_pallet(ppu: &Ppu, tile_column: usize, tile_row: usize) -> [u8; 4] {
    let attr_table_idx = tile_row / 4 * 8 + tile_column / 4;
    let attr_byte = ppu.read_vram_at(0x3C0 + attr_table_idx);

    let pallet_idx = match (tile_column % 4 / 2, tile_row % 4 / 2) {
        (0, 0) => attr_byte & 0b11,
        (1, 0) => (attr_byte >> 2) & 0b11,
        (0, 1) => (attr_byte >> 4) & 0b11,
        (1, 1) => (attr_byte >> 6) & 0b11,
        (_, _) => panic!("Impossible background pallet calculated"),
    };

    let pallet_start = 1 + (pallet_idx as usize) * 4;
    [
        ppu.read_palette_table_at(0),
        ppu.read_palette_table_at(pallet_start),
        ppu.read_palette_table_at(pallet_start + 1),
        ppu.read_palette_table_at(pallet_start + 2),
    ]
}

fn sprite_palette(ppu: &Ppu, palette_idx: u8) -> [u8; 4] {
    let start = 0x11 + (palette_idx * 4) as usize;
    [
        0,
        ppu.read_palette_table_at(start),
        ppu.read_palette_table_at(start + 1),
        ppu.read_palette_table_at(start + 2),
    ]
}
