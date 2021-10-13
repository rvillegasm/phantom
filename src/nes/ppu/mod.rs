/// Implementation of the NES' PPU (picture-processing unit)
mod registers;

use crate::nes::cartridge::MirroringMode;
use crate::nes::ppu::registers::address::AddressRegister;
use crate::nes::ppu::registers::control::ControlRegister;
use crate::nes::ppu::registers::mask::MaskRegister;
use crate::nes::ppu::registers::scroll::ScrollRegister;
use crate::nes::ppu::registers::status::StatusRegister;

pub struct Ppu {
    vram: [u8; 2048],
    chr_rom: Vec<u8>,
    mirroring_mode: MirroringMode,

    addr_register: AddressRegister,
    ctrl_register: ControlRegister,
    mask_register: MaskRegister,
    scroll_register: ScrollRegister,
    status_register: StatusRegister,

    oam_addr_register: u8,
    oam_data_register: [u8; 64 * 4],
    palette_table: [u8; 32],

    internal_data_buffer: u8,

    scanline: u16,
    cycles: usize,
    nmi_interrupt: Option<u8>,
}

impl Ppu {
    pub fn new(chr_rom: Vec<u8>, mirroring_mode: MirroringMode) -> Self {
        Ppu {
            vram: [0; 2048],
            chr_rom,
            mirroring_mode,
            addr_register: AddressRegister::new(),
            ctrl_register: ControlRegister::new(),
            mask_register: MaskRegister::new(),
            scroll_register: ScrollRegister::new(),
            status_register: StatusRegister::new(),
            oam_addr_register: 0,
            oam_data_register: [0; 64 * 4],
            palette_table: [0; 32],
            internal_data_buffer: 0,
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }

    pub fn read_palette_table_at(&self, index: usize) -> u8 {
        self.palette_table[index]
    }

    pub fn read_vram_at(&self, index: usize) -> u8 {
        self.vram[index]
    }

    pub fn chr_rom_slice(&self, from: usize, to: usize) -> &[u8] {
        &self.chr_rom[from..=to]
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        self.cycles += cycles as usize;

        if self.cycles >= 341 {
            self.cycles = self.cycles - 341;
            self.scanline += 1;

            if self.scanline == 241 {
                self.status_register.set_vblank_started_flag(true);
                self.status_register.set_sprite_zero_hit_flag(false);
                if self.ctrl_register.has_vblank_nmi_flag() {
                    self.nmi_interrupt = Some(1);
                }
            }

            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.status_register.set_sprite_zero_hit_flag(false);
                self.status_register.reset_vblank_status_flag();
                return true;
            }
        }
        return false;
    }

    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }

    pub fn read_data_register(&mut self) -> u8 {
        let addr = self.addr_register.get_address();
        self.increment_vram_address();

        match addr {
            0x0000..=0x1FFF => {
                let result = self.internal_data_buffer;
                self.internal_data_buffer = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2FFF => {
                let result = self.internal_data_buffer;
                self.internal_data_buffer = self.vram[self.mirror_vram_address(addr) as usize];
                result
            }
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
                let mirrored_addr = addr - 0x10;
                self.palette_table[(mirrored_addr - 0x3f00) as usize]
            }
            0x3000..=0x3EFF => panic!(
                "Address space 0x3000..0x3EFF is not expected to be used, requested = {}",
                addr
            ),
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize],
            _ => panic!("Unexpected access to mirrored memory address {}", addr),
        }
    }

    pub fn write_to_data_register(&mut self, data: u8) {
        let addr = self.addr_register.get_address();

        match addr {
            0x0000..=0x1FFF => {
                println!("Attempt to write to chr ROM address {}", addr);
            }
            0x2000..=0x2FFF => {
                self.vram[self.mirror_vram_address(addr) as usize] = data;
            }
            0x3000..=0x3EFF => unimplemented!(
                "Address space 0x3000..0x3EFF is not expected to be used, requested = {}",
                addr
            ),
            0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
                let mirrored_addr = addr - 0x10;
                self.palette_table[(mirrored_addr - 0x3F00) as usize] = data;
            }
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize] = data,
            _ => panic!("Unexpected access to mirrored memory address {}", addr),
        }

        self.increment_vram_address();
    }

    pub fn write_to_address_register(&mut self, value: u8) {
        self.addr_register.update(value);
    }

    pub fn write_to_control_register(&mut self, value: u8) {
        let prev_nmi_flag = self.ctrl_register.has_vblank_nmi_flag();
        self.ctrl_register.update(value);
        if !prev_nmi_flag
            && self.ctrl_register.has_vblank_nmi_flag()
            && self.status_register.has_vblank_started()
        {
            self.nmi_interrupt = Some(1);
        }
    }

    pub fn control_register_background_pattern_address(&self) -> u16 {
        self.ctrl_register.background_pattern_address()
    }

    pub fn control_register_sprite_pattern_address(&self) -> u16 {
        self.ctrl_register.sprite_pattern_address()
    }

    pub fn write_to_mask_register(&mut self, value: u8) {
        self.mask_register.update(value);
    }

    pub fn write_to_scroll_register(&mut self, value: u8) {
        self.scroll_register.write(value);
    }

    pub fn read_status_register(&mut self) -> u8 {
        let stat_reg_snapshot = self.status_register.snapshot();
        self.status_register.reset_vblank_status_flag();
        self.addr_register.reset_latch();
        self.scroll_register.reset_latch();
        stat_reg_snapshot
    }

    pub fn write_to_oam_address_register(&mut self, value: u8) {
        self.oam_addr_register = value;
    }

    pub fn write_to_oam_data_register(&mut self, value: u8) {
        self.oam_data_register[self.oam_addr_register as usize] = value;
        self.oam_addr_register = self.oam_addr_register.wrapping_add(1);
    }

    pub fn read_oam_data_register(&self) -> u8 {
        self.oam_data_register[self.oam_addr_register as usize]
    }

    pub fn write_to_oam_dma_register(&mut self, data: &[u8; 256]) {
        data.iter()
            .for_each(|x| self.write_to_oam_data_register(*x));
    }

    pub fn oam_data_size(&self) -> usize {
        self.oam_data_register.len()
    }

    pub fn read_oam_data_at(&self, index: usize) -> u8 {
        self.oam_data_register[index]
    }

    fn increment_vram_address(&mut self) {
        self.addr_register
            .increment(self.ctrl_register.vram_address_increment());
    }

    fn mirror_vram_address(&self, addr: u16) -> u16 {
        // Mirror down 0x3000-0x3eff to 0x2000-0x2eff
        let mirrored_vram = addr & 0b0010111111111111;
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x0400;
        match (&self.mirroring_mode, name_table) {
            (MirroringMode::Horizontal, 2) | (MirroringMode::Horizontal, 1) => vram_index - 0x0400,
            (MirroringMode::Vertical, 2)
            | (MirroringMode::Vertical, 3)
            | (MirroringMode::Horizontal, 3) => vram_index - 0x0800,
            _ => vram_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Ppu {
        fn new_with_empty_rom_hor() -> Self {
            Ppu::new(vec![0; 2048], MirroringMode::Horizontal)
        }

        fn new_with_empty_rom_ver() -> Self {
            Ppu::new(vec![0; 2048], MirroringMode::Vertical)
        }
    }

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_address_register(0x23);
        ppu.write_to_address_register(0x05);
        ppu.write_to_data_register(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_control_register(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_address_register(0x23);
        ppu.write_to_address_register(0x05);
        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.addr_register.get_address(), 0x2306);
        assert_eq!(ppu.read_data_register(), 0x66);
    }

    #[test]
    fn test_ppu_vram_increment() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_control_register(0b0100); // addr increments of 32
        ppu.vram[0x01FF] = 0xAB;
        ppu.vram[0x01FF + 32] = 0xCD;
        ppu.vram[0x01FF + 32 + 32] = 0xEF;

        ppu.write_to_address_register(0x21);
        ppu.write_to_address_register(0xFF);

        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xAB);
        assert_eq!(ppu.read_data_register(), 0xCD);
        assert_eq!(ppu.read_data_register(), 0xEF);
    }

    #[test]
    fn test_ppu_vram_page_cross() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_control_register(0); // addr increments of 1
        ppu.vram[0x01FF] = 0xAB;
        ppu.vram[0x0200] = 0xCD;

        ppu.write_to_address_register(0x21);
        ppu.write_to_address_register(0xFF);

        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xAB);
        assert_eq!(ppu.read_data_register(), 0xCD);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_address_register(0x24);
        ppu.write_to_address_register(0x05);
        ppu.write_to_data_register(0xAB); // write to a

        ppu.write_to_address_register(0x28);
        ppu.write_to_address_register(0x05);
        ppu.write_to_data_register(0xCD); // write to B

        ppu.write_to_address_register(0x20);
        ppu.write_to_address_register(0x05);
        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xAB);

        ppu.write_to_address_register(0x2C);
        ppu.write_to_address_register(0x05);
        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xCD);
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = Ppu::new_with_empty_rom_ver();
        ppu.write_to_address_register(0x28);
        ppu.write_to_address_register(0x05);
        ppu.write_to_data_register(0xAB); // write to a

        ppu.write_to_address_register(0x24);
        ppu.write_to_address_register(0x05);
        ppu.write_to_data_register(0xCD); // write to B

        ppu.write_to_address_register(0x20);
        ppu.write_to_address_register(0x05);
        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xAB);

        ppu.write_to_address_register(0x2C);
        ppu.write_to_address_register(0x05);
        ppu.read_data_register(); // get data into buffer
        assert_eq!(ppu.read_data_register(), 0xCD);
    }

    #[test]
    fn test_ppu_status_register_reset_latch() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.vram[0x0305] = 0xAB;

        ppu.write_to_address_register(0x21);
        ppu.write_to_address_register(0x23);
        ppu.write_to_address_register(0x05);

        ppu.read_data_register();
        assert_ne!(ppu.read_data_register(), 0xAB);

        ppu.read_status_register(); // resets latch

        ppu.write_to_address_register(0x23);
        ppu.write_to_address_register(0x05);

        ppu.read_data_register();
        assert_eq!(ppu.read_data_register(), 0xAB);
    }

    #[test]
    fn test_ppu_status_register_vblank() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.status_register.set_vblank_started_flag(true);

        let status = ppu.read_status_register();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status_register.snapshot() >> 7, 0);
    }

    #[test]
    fn test_ppu_oam_data_register_read_write() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.write_to_oam_address_register(0x10);
        ppu.write_to_oam_data_register(0xAB);
        ppu.write_to_oam_data_register(0xCD);

        ppu.write_to_oam_address_register(0x10);
        assert_eq!(ppu.read_oam_data_register(), 0xAB);
        ppu.write_to_oam_address_register(0x11);
        assert_eq!(ppu.read_oam_data_register(), 0xCD);
    }

    #[test]
    fn test_ppu_oam_dma_register() {
        let mut ppu = Ppu::new_with_empty_rom_hor();

        let mut data = [0xAB; 256];
        data[0] = 0xCD;
        data[255] = 0xEF;
        ppu.write_to_oam_address_register(0x10);
        ppu.write_to_oam_dma_register(&data);

        ppu.write_to_oam_address_register(0xF); // wrap around
        assert_eq!(ppu.read_oam_data_register(), 0xEF);
        ppu.write_to_oam_address_register(0x10);
        assert_eq!(ppu.read_oam_data_register(), 0xCD);
        ppu.write_to_oam_address_register(0x11);
        assert_eq!(ppu.read_oam_data_register(), 0xAB);
    }

    #[test]
    fn test_ppu_write_to_ctrl_register_gen_interrupt() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.status_register.set_vblank_started_flag(true);
        ppu.write_to_control_register(0b10000000);
        assert_eq!(ppu.nmi_interrupt, Some(1));
    }

    #[test]
    fn test_ppu_tick_gen_interrupt() {
        let mut ppu = Ppu::new_with_empty_rom_hor();
        ppu.scanline = 240;
        ppu.cycles = 340;
        ppu.write_to_control_register(0b10000000);
        ppu.tick(1);
        assert_eq!(ppu.nmi_interrupt, Some(1));
    }
}
