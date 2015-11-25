pub struct MMU {
    gpu: ::gpu::GPU,
    inbios: bool,
    bios: [u8; 0x0100],
    rom:  [u8; (1024*8)],
    wram: [u8; (1024*8)],
    eram: [u8; (1024*8)],
    zram: [u8; (1024*8)],
}

impl MMU {
    pub fn new() -> MMU {
        MMU {
            gpu: ::gpu::GPU::new(),
            inbios: true,
            bios: [0; 0x100],
            rom : [0; (1024*8)],
            wram: [0; (1024*8)],
            eram: [0; (1024*8)],
            zram: [0; (1024*8)],
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        let addr = address as usize;
        match addr {
            // When the gameboy starts up, all reads from 0x000 ... 0x0100
            // are redirected to the BIOS, which boots up the gameboy
            // and draws the 'Nintendo' logo on screen. After that,
            // the gameboy will read from 0x0100, which is a signal that
            // startup is over, and this area of memory can be used by
            // the cartridge.
            0x0000 ... 0x00FF => {
                if self.inbios {self.bios[addr]} else {self.rom[addr]}
            },
            0x0100 => {self.inbios = false; self.rom[addr]},

            // ROM
            0x0101 ... 0x7FFF => self.rom[addr],
            // Graphics VRAM
            0x8000 ... 0x9FFF => self.gpu.vram[addr],
            // External memory
            0xA000 ... 0xBFFF => self.eram[addr & 0x1FFF],
            // Working memory
            0xC000 ... 0xDFFF => self.wram[addr & 0x1FFF],
            // Shadowed memory - redirects to the working memory
            0xE000 ... 0xFDFF => self.wram[addr & 0x1FFF],
            // OAM is only 160 bytes
            0xFE00 ... 0xFEA0 => self.gpu.oam[addr & 0x00FF],
            // The rest is all 0's
            // (We use 0x0 as 0 because it is a cute cat face)
            0xFEA1 ... 0xFEFF => 0x0,
            0xFF00 ... 0xFF7F => 0x0, // TODO: Input/Output
            0xFF80 ... 0xFFFF => self.zram[addr & 0x007F], // zero-page RAM
            _ => {println!("Memory access out of bounds"); 0x0}
        }
    }
    pub fn rw(&mut self, addr: u16) -> u16 { // read 16-bit word
        //(self.rb(addr) as u16)+ (self.rb(addr+1) as u16 << 8)
        0x0 // TODO
    }

    pub fn write_byte(&mut self, address: u16, val: u8) { // write 8-bits
        let addr = address as usize;
        self.wram[addr - 0xC000] = val;
    }

    pub fn write_word(&mut self, addr: u16, val: u16) {} // write 16-bits TODO: Dunno types

    /*
    pub fn open() {
        let rom_path = &Path::new("pokemon.gb");
        File::open(rom_path).read_to_end();
    }
    */
}

#[test]
fn test_writing_a_byte() {
    let mut mmu = MMU::new();
    mmu.write_byte(0xC001, 0x05);
    assert_eq!(mmu.read(0xC001), 0x05);

}
