/// PPU register that is used to change the scroll position, that is,
/// to tell the PPU which pixel of the nametable selected through PPUCTRL should be
/// at the top left corner of the rendered screen. Typically, this register is written to
/// during vertical blanking, so that the next frame starts rendering from the desired location,
/// but it can also be modified during rendering in order to split the screen.
/// Changes made to the vertical scroll during rendering will only take effect on the next frame.
/// RAM address: 0x2005 - Bits: xxxx xxxx.
pub struct ScrollRegister {
    scroll_x: u8,
    scroll_y: u8,
    latch: bool,
}

impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            scroll_x: 0,
            scroll_y: 0,
            latch: false,
        }
    }

    pub fn write(&mut self, data: u8) {
        if !self.latch {
            self.scroll_x = data;
        } else {
            self.scroll_y = data;
        }

        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
