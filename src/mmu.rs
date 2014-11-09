mod MMU {
    pub fn rb(addr: u16) -> u8 {} // read 8-bit byte
    pub fn rw(addr: u16) -> u16 {} // read 16-bit word
    pub fn wb(addr: u16, val: u8) {} // write 8-bits
    pub fn ww(addr: u16, val: u16) {} // write 16-bits TODO: Still dunno types
}
