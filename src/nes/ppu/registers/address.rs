/// PPU read/write address (two writes: most significant byte, least significant byte).
/// RAM address: 0x2006 - Bits: aaaa aaaa.
pub struct AddressRegister {
    value: (u8, u8), // (higher byte, lower byte)
    hi_ptr: bool,
}

impl AddressRegister {
    pub fn new() -> Self {
        AddressRegister {
            value: (0, 0),
            hi_ptr: true,
        }
    }

    pub fn get_address(&self) -> u16 {
        ((self.value.0 as u16) << 8) | (self.value.1 as u16)
    }

    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }

        if self.get_address() > 0x3FFF {
            let mirrored_addr = self.get_address() & 0b0011111111111111;
            self.set(mirrored_addr);
        }

        self.hi_ptr = !self.hi_ptr;
    }

    pub fn increment(&mut self, inc: u8) {
        let previous_lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(inc);

        if previous_lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }

        if self.get_address() > 0x3FFF {
            let mirrored_addr = self.get_address() & 0b0011111111111111;
            self.set(mirrored_addr);
        }
    }

    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }

    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xFF) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_addr_register_set() {
        let mut reg = AddressRegister::new();
        reg.set(0xFFAB);
        assert_eq!(reg.value.0, 0xFF);
        assert_eq!(reg.value.1, 0xAB);
    }

    #[test]
    fn test_ppu_addr_register_get_address() {
        let mut reg = AddressRegister::new();
        reg.set(0xFFAB);
        assert_eq!(reg.get_address(), 0xFFAB);
    }

    #[test]
    fn test_ppu_addr_register_update() {
        let mut reg = AddressRegister::new();
        reg.update(0x02);
        reg.update(0xFF);
        assert_eq!(reg.value.0, 0x02);
        assert_eq!(reg.value.1, 0xFF);
    }

    #[test]
    fn test_ppu_addr_register_increment() {
        let mut reg = AddressRegister::new();
        reg.set(0x02FF);
        reg.increment(1);
        assert_eq!(reg.value.0, 0x03);
        assert_eq!(reg.value.1, 0x00);
    }
}
