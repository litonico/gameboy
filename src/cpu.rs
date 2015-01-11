// Wolves and the Ravens - Rogue Valley
// Holding - Grouper 
// In for the Kill - Billie Marten
// Sam Brooks
// Jose Gonzales - Stay in the Shade
// Lo-Fang
// Dan Grossman UW CS341


const ZERO      : u8 = 0x80;
const SUBTRACT  : u8 = 0x40;
const HALFCARRY : u8 = 0x20;
const CARRY     : u8 = 0x10;

struct RegisterSet {
    // 8-bit registers
    a: u8, // TODO: check with ADDr and stuff for types
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    // `Flags` register 
    // The z80 uses one-byte numbers, so
    // subtracting two 2-byte numbers requires an 8-bit (half) carry
    // as well as a full carry.
    f: u8, // 0x80: zero
           // 0x40: subtraction
           // 0x20: half-carry
           // 0x10: carry

    // 16-bit registers
    pc: u16, // `Program Counter`: keeps track of where the cpu is executing
    sp: u16, // `Stack Pointer`: used with PUSH and POP to keep a stack

    // replace with type clock?
    m: u8,
    t: u8,
}

struct Clock {
    m: u8,
    t: u8,
}

struct Z80 {
    _clock: Clock,
    _r: RegisterSet,
    MMU: ::mmu::MMU,
    ALU: ::alu::ALU,
}

macro_rules! _load(
    ($cpu:ident, $reg1:ident, $reg2:ident) => ({
        $cpu._r.$reg1 = $cpu._r.$reg2;
        $cpu._clock.m += 1;
        $cpu._clock.t += 4;
    })
);

// Load the mmu instruction of the current program counter into $reg 
macro_rules! _load_n( 
    ($cpu:ident, $reg:ident) => ({
        $cpu._r.$reg = $cpu.MMU.rb($cpu._r.pc)
        $cpu._r.pc += 1;

        $cpu._clock.m += 2;
        $cpu._clock.t += 8;
    })
);

macro_rules! _load_nn( // Load 
    ($cpu:ident, $reg1:ident, $reg2:ident) => ({
        $cpu._r.$reg1 = $cpu.MMU.rb($cpu._r.pc)
        $cpu._r.pc += 1;
        $cpu._r.$reg2 = $cpu.MMU.rb($cpu._r.pc)
        $cpu._r.pc += 1;


        $cpu._clock.m += 3;
        $cpu._clock.t += 12;
    })
);

impl Z80 {

    fn NOP(&mut self) {
        self._clock.m += 1;
        self._clock.t += 4;
    }

    /* Triggers a compiler bug
    macro_rules! (
        push($cpu:ident, $reg1:ident, $reg2:ident) => ({
            $cpu._r.sp -= 1;
            $cpu.MMU.wb($cpu._r.sp, $cpu._r.$reg1);
            $cpu._r.sp -= 1;
            $cpu.MMU.wb($cpu._r.sp, $cpu._r.$reg2);
        })
    )
    */

    fn PUSHBC(&mut self) { // Push B and C to the stack
        self._r.sp -= 1;
        self.MMU.wb(self._r.sp, self._r.b);
        self._r.sp -= 1;
        self.MMU.wb(self._r.sp, self._r.c);

        self._r.m = 3; 
        self._r.t = 12;
    }


    fn reset(&mut self) {
        self._r.a = 0; self._r.b = 0; self._r.c = 0; self._r.d = 0; 
        self._r.e = 0; self._r.h = 0; self._r.l = 0; self._r.f = 0; 
        self._r.sp = 0; 
        self._r.pc = 0; 

        self._clock.m = 0; 
        self._clock.t = 0; 
    }


    fn ADDr_e(&mut self) { // TODO: _r.a may need to be longer than u8
        self._r.a += self._r.e;
        self._r.f = 0;
        // x & 255 is used to strip a number longer than 8 bits to that length
        if !(self._r.a & 255 == 0) {
            // combine 0x80 with any other flags by way of the |= operator
            self._r.f |= 0x80;
        }
        if self._r.a > 255 {
            self._r.f |= 0x10;
        }

        self._r.a &= 255;
 
        self._r.m = 1; 
        self._r.t = 4;
    }

    fn CPr_e(&mut self) {
        let mut i = self._r.a; // Store A
        i -= self._r.b;
        self._r.f |= 0x40;
        if !(i & 255 == 0) {
            self._r.f |= 0x80;
        }
        if i < 0 {
            self._r.m |= 0x10;
        }

        self._r.m = 1; 
        self._r.t = 4;
    }


    fn POPHL(&mut self) {
        self._r.l = self.MMU.rb(self._r.sp); // read from stack pointer address
        self._r.sp += 1;
        self._r.h = self.MMU.rb(self._r.sp);
        self._r.sp += 1;

        self._r.m = 3;
        self._r.t = 12;
    }

    fn LDAmm(&mut self) { // Read from location into A
        let addr = self.MMU.rw(self._r.pc);
        self._r.pc += 2; // It's a full 16 bits, so advance twice
        self._r.a = self.MMU.rb(addr);

        self._r.m = 4;
        self._r.t = 16;
    }

    fn call(&mut self, opcode: u8) {
        /*
        match opcode {
            0x00 => self.NOP(),
            0x01 => self.LDBCnn(),
            0x02 => self.LDBCmA(),
            0x03 => self.INCBC(),
            0x04 => self.INCr_b(),
            0x05 => self.DECr_b(),
            0x06 => self.LDrn_b(),
            0x07 => self.RLCA(),
            0x08 => self.LDmmSP(),
            0x09 => self.ADDHLBC(),
            0x0A => self.LDABCm(),
            0x0B => self.DECBC(),
            0x0C => self.INCr_c(),
            0x0D => self.DECr_c(),
            0x0E => self.LDrn_c(),
            0x0F => self.RRCA(),

            0x10 => self.DJNZn(),
            0x11 => self.LDDEnn(),
            0x12 => self.LDDEmA(),
            0x13 => self.INCDE(),
            0x14 => self.INCr_d(),
            0x15 => self.DECr_d(),
            0x16 => self.LDrn_d(),
            0x17 => self.RLA(),
            0x18 => self.JRn(),
            0x19 => self.ADDHLDE(),
            0x1A => self.LDADEm(),
            0x1B => self.DECDE(),
            0x1C => self.INCr_e(),
            0x1D => self.DECr_e(),
            0x1E => self.LDrn_e(),
            0x1F => self.RRA(),

            0x20 => self.JRNZn(),
            0x21 => self.LDHLnn(),
            0x22 => self.LDHLIA(),
            0x23 => self.INCHL(),
            0x24 => self.INCr_h(),
            0x25 => self.DECr_h(),
            0x26 => self.LDrn_h(),
            0x27 => self.XX(),
            0x28 => self.JRZn(),
            0x29 => self.ADDHLHL(),
            0x2A => self.LDAHLI(),
            0x2B => self.DECHL(),
            0x2C => self.INCr_l(),
            0x2D => self.DECr_l(),
            0x2E => self.LDrn_l(),
            0x2F => self.CPL(),

            0x30 => self.JRNCn(),
            0x31 => self.LDSPnn(),
            0x32 => self.LDHLDA(),
            0x33 => self.INCSP(),
            0x34 => self.INCHLm(),
            0x35 => self.DECHLm(),
            0x36 => self.LDHLmn(),
            0x37 => self.SCF(),
            0x38 => self.JRCn(),
            0x39 => self.ADDHLSP(),
            0x3A => self.LDAHLD(),
            0x3B => self.DECSP(),
            0x3C => self.INCr_a(),
            0x3D => self.DECr_a(),
            0x3E => self.LDrn_a(),
            0x3F => self.CCF(),

            0x40 => self.LDrr_bb(),
            0x41 => self.LDrr_bc(),
            0x42 => self.LDrr_bd(),
            0x43 => self.LDrr_be(),
            0x44 => self.LDrr_bh(),
            0x45 => self.LDrr_bl(),
            0x46 => self.LDrHLm_b(),
            0x47 => self.LDrr_ba(),
            0x48 => self.LDrr_cb(),
            0x49 => self.LDrr_cc(),
            0x4A => self.LDrr_cd(),
            0x4B => self.LDrr_ce(),
            0x4C => self.LDrr_ch(),
            0x4D => self.LDrr_cl(),
            0x4E => self.LDrHLm_c(),
            0x4F => self.LDrr_ca(),

            0x50 => self.LDrr_db(),
            0x51 => self.LDrr_dc(),
            0x52 => self.LDrr_dd(),
            0x53 => self.LDrr_de(),
            0x54 => self.LDrr_dh(),
            0x55 => self.LDrr_dl(),
            0x56 => self.LDrHLm_d(),
            0x57 => self.LDrr_da(),
            0x58 => self.LDrr_eb(),
            0x59 => self.LDrr_ec(),
            0x5A => self.LDrr_ed(),
            0x5B => self.LDrr_ee(),
            0x5C => self.LDrr_eh(),
            0x5D => self.LDrr_el(),
            0x5E => self.LDrHLm_e(),
            0x5F => self.LDrr_ea(),

            0x60 => self.LDrr_hb(),
            0x61 => self.LDrr_hc(),
            0x62 => self.LDrr_hd(),
            0x63 => self.LDrr_he(),
            0x64 => self.LDrr_hh(),
            0x65 => self.LDrr_hl(),
            0x66 => self.LDrHLm_h(),
            0x67 => self.LDrr_ha(),
            0x68 => self.LDrr_lb(),
            0x69 => self.LDrr_lc(),
            0x6A => self.LDrr_ld(),
            0x6B => self.LDrr_le(),
            0x6C => self.LDrr_lh(),
            0x6D => self.LDrr_ll(),
            0x6E => self.LDrHLm_l(),
            0x6F => self.LDrr_la(),

            0x70 => self.LDHLmr_b(),
            0x71 => self.LDHLmr_c(),
            0x72 => self.LDHLmr_d(),
            0x73 => self.LDHLmr_e(),
            0x74 => self.LDHLmr_h(),
            0x75 => self.LDHLmr_l(),
            0x76 => self.HALT(),
            0x77 => self.LDHLmr_a(),
            0x78 => self.LDrr_ab(),
            0x79 => self.LDrr_ac(),
            0x7A => self.LDrr_ad(),
            0x7B => self.LDrr_ae(),
            0x7C => self.LDrr_ah(),
            0x7D => self.LDrr_al(),
            0x7E => self.LDrHLm_a(),
            0x7F => self.LDrr_aa(),

            0x80 => self.ADDr_b(),
            0x81 => self.ADDr_c(),
            0x82 => self.ADDr_d(),
            0x83 => self.ADDr_e(),
            0x84 => self.ADDr_h(),
            0x85 => self.ADDr_l(),
            0x86 => self.ADDHL(),
            0x87 => self.ADDr_a(),
            0x88 => self.ADCr_b(),
            0x89 => self.ADCr_c(),
            0x8A => self.ADCr_d(),
            0x8B => self.ADCr_e(),
            0x8C => self.ADCr_h(),
            0x8D => self.ADCr_l(),
            0x8E => self.ADCHL(),
            0x8F => self.ADCr_a(),

            0x90 => self.SUBr_b(),
            0x91 => self.SUBr_c(),
            0x92 => self.SUBr_d(),
            0x93 => self.SUBr_e(),
            0x94 => self.SUBr_h(),
            0x95 => self.SUBr_l(),
            0x96 => self.SUBHL(),
            0x97 => self.SUBr_a(),
            0x98 => self.SBCr_b(),
            0x99 => self.SBCr_c(),
            0x9A => self.SBCr_d(),
            0x9B => self.SBCr_e(),
            0x9C => self.SBCr_h(),
            0x9D => self.SBCr_l(),
            0x9E => self.SBCHL(),
            0x9F => self.SBCr_a(),

            0xA0 => self.ANDr_b(),
            0xA1 => self.ANDr_c(),
            0xA2 => self.ANDr_d(),
            0xA3 => self.ANDr_e(),
            0xA4 => self.ANDr_h(),
            0xA5 => self.ANDr_l(),
            0xA6 => self.ANDHL(),
            0xA7 => self.ANDr_a(),
            0xA8 => self.XORr_b(),
            0xA9 => self.XORr_c(),
            0xAA => self.XORr_d(),
            0xAB => self.XORr_e(),
            0xAC => self.XORr_h(),
            0xAD => self.XORr_l(),
            0xAE => self.XORHL(),
            0xAF => self.XORr_a(),

            0xB0 => self.ORr_b(),
            0xB1 => self.ORr_c(),
            0xB2 => self.ORr_d(),
            0xB3 => self.ORr_e(),
            0xB4 => self.ORr_h(),
            0xB5 => self.ORr_l(),
            0xB6 => self.ORHL(),
            0xB7 => self.ORr_a(),
            0xB8 => self.CPr_b(),
            0xB9 => self.CPr_c(),
            0xBA => self.CPr_d(),
            0xBB => self.CPr_e(),
            0xBC => self.CPr_h(),
            0xBD => self.CPr_l(),
            0xBE => self.CPHL(),
            0xBF => self.CPr_a(),

            0xC0 => self.RETNZ(),
            0xC1 => self.POPBC(),
            0xC2 => self.JPNZnn(),
            0xC3 => self.JPnn(),
            0xC4 => self.CALLNZnn(),
            0xC5 => self.PUSHBC(),
            0xC6 => self.ADDn(),
            0xC7 => self.RST00(),
            0xC8 => self.RETZ(),
            0xC9 => self.RET(),
            0xCA => self.JPZnn(),
            0xCB => self.MAPcb(),
            0xCC => self.CALLZnn(),
            0xCD => self.CALLnn(),
            0xCE => self.ADCn(),
            0xCF => self.RST08(),

            0xD0 => self.RETNC(),
            0xD1 => self.POPDE(),
            0xD2 => self.JPNCnn(),
            0xD3 => self.XX(),
            0xD4 => self.CALLNCnn(),
            0xD5 => self.PUSHDE(),
            0xD6 => self.SUBn(),
            0xD7 => self.RST10(),
            0xD8 => self.RETC(),
            0xD9 => self.RETI(),
            0xDA => self.JPCnn(),
            0xDB => self.XX(),
            0xDC => self.CALLCnn(),
            0xDD => self.XX(),
            0xDE => self.SBCn(),
            0xDF => self.RST18(),

            0xE0 => self.LDIOnA(),
            0xE1 => self.POPHL(),
            0xE2 => self.LDIOCA(),
            0xE3 => self.XX(),
            0xE4 => self.XX(),
            0xE5 => self.PUSHHL(),
            0xE6 => self.ANDn(),
            0xE7 => self.RST20(),
            0xE8 => self.ADDSPn(),
            0xE9 => self.JPHL(),
            0xEA => self.LDmmA(),
            0xEB => self.XX(),
            0xEC => self.XX(),
            0xED => self.XX(),
            0xEE => self.ORn(),
            0xEF => self.RST28(),

            0xF0 => self.LDAIOn(),
            0xF1 => self.POPAF(),
            0xF2 => self.LDAIOC(),
            0xF3 => self.DI(),
            0xF4 => self.XX(),
            0xF5 => self.PUSHAF(),
            0xF6 => self.XORn(),
            0xF7 => self.RST30(),
            0xF8 => self.LDHLSPn(),
            0xF9 => self.XX(),
            0xFA => self.LDAmm(),
            0xFB => self.EI(),
            0xFC => self.XX(),
            0xFD => self.XX(),
            0xFE => self.CPn(),
            0xFF => self.RST38()
        }
    */
    }
}


