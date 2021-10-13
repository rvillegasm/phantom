// Memory segmentation:
//  _______________ $10000  _______________
// | PRG-ROM       |       |               |
// | Upper Bank    |       |               |
// |_ _ _ _ _ _ _ _| $C000 | PRG-ROM       |
// | PRG-ROM       |       |               |
// | Lower Bank    |       |               |
// |_______________| $8000 |_______________|
// | SRAM          |       | SRAM          |
// |_______________| $6000 |_______________|
// | Expansion ROM |       | Expansion ROM |
// |_______________| $4020 |_______________|
// | I/O Registers |       |               |
// |_ _ _ _ _ _ _ _| $4000 |               |
// | Mirrors       |       | I/O Registers |
// | $2000-$2007   |       |               |
// |_ _ _ _ _ _ _ _| $2008 |               |
// | I/O Registers |       |               |
// |_______________| $2000 |_______________|
// | Mirrors       |       |               |
// | $0000-$07FF   |       |               |
// |_ _ _ _ _ _ _ _| $0800 |               |
// | RAM           |       | RAM           |
// |_ _ _ _ _ _ _ _| $0200 |               |
// | Stack         |       |               |
// |_ _ _ _ _ _ _ _| $0100 |               |
// | Zero Page     |       |               |
// |_______________| $0000 |_______________|

pub trait Memory {
    fn mem_read(&mut self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr + 1, hi);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMem {
        memory: [u8; 2048],
    }

    impl Memory for TestMem {
        fn mem_read(&mut self, addr: u16) -> u8 {
            self.memory[addr as usize]
        }

        fn mem_write(&mut self, addr: u16, data: u8) {
            self.memory[addr as usize] = data;
        }
    }

    #[test]
    fn test_memory_trait_default_mem_read_16() {
        let mut mem = TestMem { memory: [0; 2048] };
        mem.memory[0x0000 as usize] = 0x10;
        mem.memory[0x0001 as usize] = 0x00;
        assert_eq!(mem.mem_read_u16(0x00), 0x0010);
    }

    #[test]
    fn test_memory_trait_default_mem_write_16() {
        let mut mem = TestMem { memory: [0; 2048] };
        mem.mem_write_u16(0x0000, 0x8000);
        assert_eq!(mem.memory[0x0000 as usize], 0x00);
        assert_eq!(mem.memory[0x0001 as usize], 0x80);
    }
}