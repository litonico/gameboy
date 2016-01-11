pub struct GPU {
    pub vram: [u8; (1024*8)],
    pub oam:  [u8; 160],
}

impl GPU {
    pub fn new() -> GPU {
        GPU {
            vram: [0; (1024*8)],
            oam:  [0; 160],
        }
    }
    pub fn write_byte(&mut self, address: u16, val: u8) {}
}
