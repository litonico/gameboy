mod mmu {

    use std::io::File;

    pub struct MMU {
        gpu: GPU,
        inbios: bool,
        bios: [u8, ..BIOS_SIZE],
        wram: [u8, ..WRAM_SIZE],
        eram: [u8, ..ERAM_SIZE],
        zram: [u8, ..ZRAM_SIZE],
    }

    impl MMU {
        pub fn rb(&self, addr: u16) -> u8 {  // read 8-bit byte
            match addr {
                // 0x000 ... 0x0100 is the BIOS, which boots up the gameboy
                // and draws the 'Nintendo' logo on screen. After that,
                // this area of memory can be overwritten by the cartridge.
                0x0000 ... 0x00FF => {
                    if self.inbios {self.bios[addr]} else {self.rom[addr]}
                },
                0x0100 => {self.inbios = false; self.rom[addr]},

                // ROM
                0x0101 ... 0x7FFF => self.rom[addr],
                0x8000 ... 0x9FFF => self.gpu.vram[addr], // Graphics VRAM
                0xA000 ... 0xBFFF => self.eram[addr & 0x1FFF], // External mem
                0xC000 ... 0xDFFF => self.wram[addr & 0x1FFF], // Working mem
                0xE000 ... 0xFDFF => self.wram[addr & 0x1FFF], // Shadowed mem
                // OAM is only 160 bytes
                0xFE00 ... 0xFEA0 => self.gpu.oam[addr & 0x00FF],
                // the rest is all 0's
                0xFEA1 ... 0xFE9F => 0x0,
                0xFF00 ... 0xFF7F => 0x0, // TODO: Input/Output
                0xFF80 ... 0xFFFF => self.zram[addr & 0x007F], // zero-page RAM
                _ => warn!("Memory access out of bounds")
            }
        }
        pub fn rw(addr: u16) -> u16 { // read 16-bit word
            self.rb(addr) + (self.rb(addr+1) << 8)
        } 

        pub fn wb(addr: u16, val: u8) {} // write 8-bits
        pub fn ww(addr: u16, val: u16) {} // write 16-bits TODO: Dunno types

        pub fn open(
    }
}
