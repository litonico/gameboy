mod types {
    pub struct Byte(u8);
    pub struct Address(u16);
    pub struct Word(u16);
}

pub mod cpu;
pub mod mmu;
pub mod gpu;
