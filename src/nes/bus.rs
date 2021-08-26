/// Implementation of the NES' Bus that connects the CPU, PPU and memory together
use crate::nes::memory::Memory;

const RAM_START_ADDR: u16 = 0x0000;
const RAM_MIRRORS_END_ADDR: u16 = 0x1FFF;
const PPU_REGISTERS_START_ADDR: u16 = 0x2000;
const PPU_REGISTERS_MIRRORS_END_ADDR: u16 = 0x3FFF;

const RAM_MIRROR_MASK: u16 = 0b00000111_11111111;
const PPU_MIRROR_MASK: u16 = 0b00100000_00000111;

pub struct Bus {
    cpu_ram: [u8; 2048],
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
            _ => {
                println!("Bus: Memory read at address {:#X} ignored (Returning 0)", addr);
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
            _ => {
                println!("Bus: Memory write of byte {:#X} at address {:#X} ignored", data, addr);
            }
        }
    }
}

impl Bus {
    pub fn new() -> Self {
        Bus { cpu_ram: [0; 2048] }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_mem_read_ram() {
        let mut bus = Bus::new();
        bus.cpu_ram[0x00] = 0xFF;
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_mem_write_ram() {
        let mut bus = Bus::new();
        bus.mem_write(0x00, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
    }

    #[test]
    fn test_bus_ram_mirroring() {
        // 0x0800 is mirrored into 0x00, 0x1000 and 0x1800
        let mut bus = Bus::new();
        bus.mem_write(0x0800, 0xFF);
        assert_eq!(bus.mem_read(0x00), 0xFF);
        assert_eq!(bus.mem_read(0x1000), 0xFF);
        assert_eq!(bus.mem_read(0x1800), 0xFF);
    }
}