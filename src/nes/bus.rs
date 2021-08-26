use crate::nes::cartridge::Rom;
/// Implementation of the NES' Bus that connects the CPU, PPU and memory together
use crate::nes::memory::Memory;

const RAM_START_ADDR: u16 = 0x0000;
const RAM_MIRRORS_END_ADDR: u16 = 0x1FFF;
const PPU_REGISTERS_START_ADDR: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END_ADDR: u16 = 0x3FFF;

const RAM_MIRROR_MASK: u16 = 0b00000111_11111111;
const PPU_MIRROR_MASK: u16 = 0b00100000_00000111;

const PRG_ROM_START_ADDR: u16 = 0x8000;
const PRG_ROM_END_ADDR: u16 = 0xFFFF;

pub struct Bus {
    cpu_ram: [u8; 2048],
    rom: Rom,
}

impl Memory for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM_START_ADDR..=RAM_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & RAM_MIRROR_MASK;
                self.cpu_ram[mirrored_addr as usize]
            }
            PPU_REGISTERS_START_ADDR..=PPU_REGISTERS_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & PPU_MIRROR_MASK;
                todo!("PPU access through the Bus not yet supported!")
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
            PPU_REGISTERS_START_ADDR..=PPU_REGISTERS_MIRRORS_END_ADDR => {
                let mirrored_addr = addr & PPU_MIRROR_MASK;
                todo!("PPU access through the Bus not yet supported!")
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

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_ram: [0; 2048],
            rom,
        }
    }

    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000; // set addr relative to 0
        if self.rom.prg_rom().len() == 0x4000 && addr >= 0x4000 {
            addr = addr % 0x4000; // Mirror if needed
        }
        self.rom.prg_rom()[addr as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nes::cartridge::tests;

    #[test]
    fn test_bus_mem_read_ram() {
        let mut bus = Bus::new(tests::create_simple_test_rom());
        bus.cpu_ram[0x00] = 0xFF;
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_mem_write_ram() {
        let mut bus = Bus::new(tests::create_simple_test_rom());
        bus.mem_write(0x00, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_ram_mirroring() {
        // 0x0800 is mirrored into 0x00, 0x1000 and 0x1800
        let mut bus = Bus::new(tests::create_simple_test_rom());
        bus.mem_write(0x0800, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
        assert_eq!(bus.mem_read(0x1000), 0xFF);
        assert_eq!(bus.mem_read(0x1800), 0xFF);
    }
}
