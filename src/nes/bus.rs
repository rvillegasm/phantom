/// Implementation of the NES' Bus that connects the CPU, PPU and memory together
use crate::nes::cartridge::Rom;
use crate::nes::memory::Memory;
use crate::nes::ppu::Ppu;

const RAM_START_ADDR: u16 = 0x0000;
const RAM_MIRRORS_END_ADDR: u16 = 0x1FFF;

const PPU_CTRL_REGISTER: u16 = 0x2000;
const PPU_MASK_REGISTER: u16 = 0x2001;
const PPU_STATUS_REGISTER: u16 = 0x2002;
const PPU_OAM_ADDR_REGISTER: u16 = 0x2003;
const PPU_OAM_DATA_REGISTER: u16 = 0x2004;
const PPU_SCROLL_REGISTER: u16 = 0x2005;
const PPU_ADDR_REGISTER: u16 = 0x2006;
const PPU_DATA_REGISTER: u16 = 0x2007;
const PPU_OAM_DMA_REGISTER: u16 = 0x4014;

const PPU_REGISTERS_MIRRORS_START_ADDR: u16 = 0x2008;
const PPU_REGISTERS_MIRRORS_END_ADDR: u16 = 0x3FFF;

const RAM_MIRROR_MASK: u16 = 0b00000111_11111111;
const PPU_MIRROR_MASK: u16 = 0b00100000_00000111;

const PRG_ROM_START_ADDR: u16 = 0x8000;
const PRG_ROM_END_ADDR: u16 = 0xFFFF;

pub struct Bus<'call> {
    cpu_ram: [u8; 2048],
    prg_rom: Vec<u8>,
    ppu: Ppu,

    cycles: usize,
    game_loop_callback: Box<dyn FnMut(&Ppu) + 'call>,
}

impl Memory for Bus<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM_START_ADDR..=RAM_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & RAM_MIRROR_MASK;
                self.cpu_ram[mirrored_addr as usize]
            }
            PPU_CTRL_REGISTER
            | PPU_MASK_REGISTER
            | PPU_OAM_ADDR_REGISTER
            | PPU_SCROLL_REGISTER
            | PPU_ADDR_REGISTER
            | PPU_OAM_DMA_REGISTER => {
                panic!(
                    "Bus: Attempted to read from write-only PPU address {:#X}",
                    addr
                );
            }
            PPU_STATUS_REGISTER => self.ppu.read_status_register(),
            PPU_OAM_DATA_REGISTER => self.ppu.read_oam_data_register(),
            PPU_DATA_REGISTER => self.ppu.read_data_register(),
            PPU_REGISTERS_MIRRORS_START_ADDR..=PPU_REGISTERS_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & PPU_MIRROR_MASK;
                self.mem_read(mirrored_addr)
            }
            PRG_ROM_START_ADDR..=PRG_ROM_END_ADDR => self.read_prg_rom(addr),
            _ => {
                println!(
                    "Bus: Memory read at address {:#X} ignored (Returning 0)",
                    addr
                );
                0
            }
        }
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM_START_ADDR..=RAM_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & RAM_MIRROR_MASK;
                self.cpu_ram[mirrored_addr as usize] = data;
            }
            PPU_CTRL_REGISTER => {
                self.ppu.write_to_control_register(data);
            }
            PPU_MASK_REGISTER => {
                self.ppu.write_to_mask_register(data);
            }
            PPU_STATUS_REGISTER => {
                panic!("Bus: Attempted to write to PPU Status register 0x2002");
            }
            PPU_OAM_ADDR_REGISTER => {
                self.ppu.write_to_oam_address_register(data);
            }
            PPU_OAM_DATA_REGISTER => {
                self.ppu.write_to_oam_data_register(data);
            }
            PPU_SCROLL_REGISTER => {
                self.ppu.write_to_scroll_register(data);
            }
            PPU_ADDR_REGISTER => {
                self.ppu.write_to_address_register(data);
            }
            PPU_DATA_REGISTER => {
                self.ppu.write_to_data_register(data);
            }
            PPU_REGISTERS_MIRRORS_START_ADDR..=PPU_REGISTERS_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & PPU_MIRROR_MASK;
                self.mem_write(mirrored_addr, data);
            }
            PPU_OAM_DMA_REGISTER => {
                let mut buffer: [u8; 256] = [0; 256];
                let hi = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }

                self.ppu.write_to_oam_dma_register(&buffer);
            }
            PRG_ROM_START_ADDR..=PRG_ROM_END_ADDR => {
                panic!("Bus: Attempted to write to PRG_ROM address {:#X}", addr);
            }
            _ => {
                println!(
                    "Bus: Memory write of byte {:#X} at address {:#X} ignored",
                    data, addr
                );
            }
        }
    }
}

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, game_loop_callback: F) -> Bus<'call>
    where
        F: FnMut(&Ppu) + 'call
    {
        Bus {
            cpu_ram: [0; 2048],
            prg_rom: rom.prg_rom,
            ppu: Ppu::new(rom.chr_rom, rom.screen_mirroring),
            cycles: 0,
            game_loop_callback: Box::from(game_loop_callback),
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        // https://wiki.nesdev.com/w/index.php/Catch-up
        // ppu clock is three times faster than cpu's
        self.cycles += cycles as usize;
        let generate_new_frame = self.ppu.tick(cycles * 3);
        if generate_new_frame {
            (self.game_loop_callback)(&self.ppu);
        }
    }

    pub fn poll_nmi_status(&mut self) -> Option<u8> {
        self.ppu.poll_nmi_interrupt()
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000; // set addr relative to 0
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000; // Mirror if needed
        }
        self.prg_rom[addr as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nes::cartridge::tests;

    #[test]
    fn test_bus_mem_read_ram() {
        let mut bus = Bus::new(tests::create_simple_test_rom(), |ppu: &Ppu| {});
        bus.cpu_ram[0x00] = 0xFF;
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_mem_write_ram() {
        let mut bus = Bus::new(tests::create_simple_test_rom(), |ppu: &Ppu| {});
        bus.mem_write(0x00, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_ram_mirroring() {
        // 0x0800 is mirrored into 0x00, 0x1000 and 0x1800
        let mut bus = Bus::new(tests::create_simple_test_rom(), |ppu: &Ppu| {});
        bus.mem_write(0x0800, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
        assert_eq!(bus.mem_read(0x1000), 0xFF);
        assert_eq!(bus.mem_read(0x1800), 0xFF);
    }
}
