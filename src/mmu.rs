use std::io::File;

pub struct MMU {
    gpu: &::gpu::GPU,
    inbios: bool,
    bios: [u8; 0x0100],
    rom: [u8; (1024*8)],
    wram: [u8; (1024*8)],
    eram: [u8; (1024*8)],
    zram: [u8; (1024*8)],
}

impl MMU {
    pub fn rb(&mut self, addr: u16) -> u8 {  // read 8-bit byte
        let raddr = addr as uint; // array access needs to be uint
        match addr {
            // When the gameboy starts up, reads from 0x000 ... 0x0100 
            // are redirected to the BIOS, which boots up the gameboy
            // and draws the 'Nintendo' logo on screen. After that,
            // the gameboy will read from 0x0100, which is a signal that
            // startup is over, and this area of memory can be used by 
            // the cartridge.
            0x0000 ... 0x00FF => {
                if self.inbios {self.bios[raddr]} else {self.rom[raddr]}
            },
            0x0100 => {self.inbios = false; self.rom[raddr]},

            // ROM
            0x0101 ... 0x7FFF => self.rom[raddr],
            0x8000 ... 0x9FFF => self.gpu.vram[raddr], // Graphics VRAM
            0xA000 ... 0xBFFF => self.eram[raddr & 0x1FFF], // External mem
            0xC000 ... 0xDFFF => self.wram[raddr & 0x1FFF], // Working mem
            0xE000 ... 0xFDFF => self.wram[raddr & 0x1FFF], // Shadowed mem
            // OAM is only 160 bytes
            0xFE00 ... 0xFEA0 => self.gpu.oam[raddr & 0x00FF],
            // the rest is all 0's
            0xFEA1 ... 0xFEFF => 0x0,
            0xFF00 ... 0xFF7F => 0x0, // TODO: Input/Output
            0xFF80 ... 0xFFFF => self.zram[raddr & 0x007F], // zero-page RAM
            _ => {println!("Memory access out of bounds"); 0x0}
        }
    }
    pub fn rw(&mut self, addr: u16) -> u16 { // read 16-bit word
        (self.rb(addr) as u16)+ (self.rb(addr+1) as u16 << 8)
    } 

    pub fn wb(&mut self, addr: u16, val: u8) {} // write 8-bits
    pub fn ww(&mut self, addr: u16, val: u16) {} // write 16-bits TODO: Dunno types

    pub fn open() {
        let rom_path = &Path::new("pokemon.gb");
        File::open(rom_path).read_to_end();
    }
}
