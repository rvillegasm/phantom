use bitflags::bitflags;

bitflags! {
    // 7  bit  0
    // ---- ----
    // VPHB SINN
    // |||| ||||
    // |||| ||++- Base nametable address
    // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    // |||| |     (0: add 1, going across; 1: add 32, going down)
    // |||| +---- Sprite pattern table address for 8x8 sprites
    // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    // |||+------ Background pattern table address (0: $0000; 1: $1000)
    // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
    // |+-------- PPU master/slave select
    // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------- Generate an NMI at the start of the
    //            vertical blanking interval (0: off; 1: on)

    /// PPU control register. Tells the PPU how to manage:
    /// NMI enable (V), PPU master/slave (P), sprite height (H), background tile select (B),
    /// sprite tile select (S), increment mode (I), nametable select (NN).
    /// RAM address: 0x2000 - Bits: VPHB SINN.
    pub struct ControlRegister: u8 {
        const NAMETABLE_LO                 = 0b00000001;
        const NAMETABLE_HI                 = 0b00000010;
        const VRAM_ADDR_INCREMENT          = 0b00000100;
        const SPRITE_PATTERN_TABLE_ADDR    = 0b00001000;
        const BACKGROUND_PATTER_TABLE_ADDR = 0b00010000;
        const SPRITE_SIZE                  = 0b00100000;
        const MASTER_SLAVE_SELECT          = 0b01000000;
        const GENERATE_NMI                 = 0b10000000;
    }
}

impl ControlRegister {
    pub fn new() -> Self {
        ControlRegister::from_bits_truncate(0b00000000)
    }

    pub fn nametable_address(&self) -> u16 {
        match self.bits & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => panic!(
                "Impossible nametable address. Something went regarding the control register"
            ),
        }
    }

    pub fn sprite_pattern_address(&self) -> u16 {
        if !self.contains(ControlRegister::SPRITE_PATTERN_TABLE_ADDR) {
            0
        } else {
            0x1000
        }
    }

    pub fn background_pattern_address(&self) -> u16 {
        if !self.contains(ControlRegister::BACKGROUND_PATTER_TABLE_ADDR) {
            0
        } else {
            0x1000
        }
    }

    pub fn sprite_size(&self) -> u8 {
        if !self.contains(ControlRegister::SPRITE_SIZE) {
            8
        } else {
            16
        }
    }

    pub fn master_slave_select(&self) -> u8 {
        if !self.contains(ControlRegister::MASTER_SLAVE_SELECT) {
            0
        } else {
            1
        }
    }

    pub fn vram_address_increment(&self) -> u8 {
        if !self.contains(ControlRegister::VRAM_ADDR_INCREMENT) {
            1
        } else {
            32
        }
    }

    pub fn has_vblank_nmi_flag(&self) -> bool {
        return self.contains(ControlRegister::GENERATE_NMI);
    }

    pub fn update(&mut self, bits_data: u8) {
        self.bits = bits_data;
    }
}
