#![allow(non_snake_case)] // CPU Opcodes have capitalized names
#![allow(dead_code)] // TODO
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
    a: u8,
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
}

macro_rules! register_pair {
     ($regs:ident, $r1:ident, $r2:ident) => (
         {
            let r1 = $regs.$r1 as u16;
            let r2 = $regs.$r2 as u16;
            let result = r1<<8 | r2;
            result
         }
     )
}

macro_rules! set_register_pair {
     ($regs:ident, $r1:ident, $r2:ident, $n:ident) => (
         {
            $regs.$r1 = ($n >> 8) as u8;
            $regs.$r2 = $n as u8;
         }
     )
}

impl RegisterSet {
    pub fn new() -> RegisterSet {
        RegisterSet {
            a: 0, b: 0, c: 0, d:0, e: 0, h: 0, l: 0,
            f: 0,
            pc: 0, sp: 0
        }
    }

    pub fn hl(&self) -> u16 { register_pair!(self, h, l) }
    pub fn bc(&self) -> u16 { register_pair!(self, b, c) }
    pub fn de(&self) -> u16 { register_pair!(self, d, e) }
    pub fn set_hl(&mut self, n:u16) { set_register_pair!(self, h, l, n) }
    pub fn set_bc(&mut self, n:u16) { set_register_pair!(self, b, c, n) }
    pub fn set_de(&mut self, n:u16) { set_register_pair!(self, d, e, n) }
}

struct Clock {
    m: u32, // The Gameboy clock has a speed of about 4MHz, so it's easier
            // for us to note times here as actual time divided by 4.
    t: u32, // Actual time (always m*4)
}

impl Clock {
    fn new() -> Clock {
        Clock { m:0, t:0 }
    }

    fn tick(&mut self, t:u8) {
        self.m += t as u32;
        self.t += t as u32 * 4;
    }
}

pub struct Z80 {
    clock: Clock,
    regs: RegisterSet,
    mmu: ::mmu::MMU,
}

/// CPU Opcode Macro Definitions

/// LD   r,r         xx         4 ---- r=r
/// Load a register r1 with another register r2.
macro_rules! LDrr {
     ($cpu:ident, $r1:ident, $r2:ident) => (
         {
             $cpu.regs.$r1 = $cpu.regs.$r2;
             $cpu.clock.tick(1);
         }
     )
 }

/// LD   r,n         xx nn      8 ---- r=n
/// Load a register r with a number nn, which is read from where
/// the program counter is right now.
macro_rules! LDrn {
    ($cpu:ident, $r:ident) => (
        {
            $cpu.regs.$r = $cpu.read_immediate_byte();
            $cpu.clock.tick(2);
        }
    )
}

/// LD   r,(HL)      xx         8 ---- r=(HL)
/// Load a register r with the with the byte at a memory location,
/// where the memory location is the contents of the HL registers.
macro_rules! LDrHL {
    ($cpu:ident, $r:ident) => (
        {
            $cpu.regs.$r = $cpu.read_hl();
            $cpu.clock.tick(2);
        }
    )
}

/// LD   (HL),r      7x         8 ---- (HL)=r
/// Load the location given by registers HL with contents of register r
macro_rules! LDHLr {
    ($cpu:ident, $r:ident) => (
        {
            let r = $cpu.regs.$r;
            $cpu.write_hl(r);
            $cpu.clock.tick(2);
        }
    )
}

/// ADD  A,r         8x         4 z0hc A=A+r
/// Add a register to A and store the result in A
macro_rules! ADDr {
    ($cpu:ident, $r:ident) => (
        {
            let a = $cpu.regs.a;
            let r = $cpu.regs.$r;
            $cpu.regs.a = $cpu.add8(a, r);
            $cpu.clock.tick(1);
        }
    )
}

/// ADC  A,r         8x         4 z0hc A=A+r+cy
/// Add the the contents of register r to register a. If the carry bit is set,
/// add 1 to the result.

macro_rules! ADCr {
    ($cpu:ident, $r:ident) => (
        {
            let a = $cpu.regs.a;
            let r = $cpu.regs.$r;
            $cpu.regs.a = $cpu.add8(a, r);
            if $cpu.flag_is_set(CARRY) {
                // This will never overflow, so the direct add is ok
                $cpu.regs.a += 1;
            }
            $cpu.clock.tick(1);
        }
    )
}

/// ADD  HL,rr     x9           8 -0hc HL = HL+rr     ;rr may be BC,DE,HL,SP
/// Add register pair rr to registers HL
macro_rules! ADDHLrr {
    ($cpu:ident, $rs:ident) => (
        {
            let hl = $cpu.regs.hl();
            let rs = $cpu.regs.$rs();
            let result = $cpu.add16(hl, rs);
            $cpu.regs.set_hl(result);
            $cpu.clock.tick(2);
        }
    )
}

impl Z80 {
    pub fn new() -> Z80 {
        Z80 {
            clock: Clock::new(),
            regs: RegisterSet::new(),
            mmu: ::mmu::MMU::new(),
        }
    }

    // Utilities
    fn read_hl(&mut self) -> u8 {
        let hl = self.regs.hl();
        self.mmu.read(hl)
    }

    fn write_hl(&mut self, b: u8) {
        let hl = self.regs.hl();
        self.mmu.write_byte(hl, b);
    }

    fn read_immediate_byte(&mut self) -> u8 {
        let n = self.mmu.read(self.regs.pc);
        self.regs.pc += 1;
        n
    }

    fn read_immediate_word(&mut self) -> u16 {
        let small_byte = self.mmu.read(self.regs.pc) as u16;
        self.regs.pc += 1;
        let large_byte = self.mmu.read(self.regs.pc) as u16;
        self.regs.pc += 1;
        let nn = large_byte<<8 | small_byte;
        nn
    }

    fn flag_is_set(&mut self, flag: u8) -> bool {
        self.regs.f & flag == flag
    }

    fn clear_flags(&mut self) {
        self.regs.f = 0x0;;
    }

    fn set_flag(&mut self, flag: u8) {
        self.regs.f = self.regs.f | flag;
    }

    fn unset_flag(&mut self, flag: u8) {
        let inverse_flag : u8 = !flag;
        self.regs.f = self.regs.f & inverse_flag;
    }

    // Arithmetic Utilities

    fn add8(&mut self, a:u8, b:u8) -> u8 {
        // Cheat by holding result in a u16
        let overflowing_sum : u16 = a as u16 + b as u16;
        // Set the appropriate flags
        self.clear_flags();
        if overflowing_sum == 0 { self.set_flag(ZERO) }
        if overflowing_sum > 0xFF { self.set_flag(CARRY) }
        // Mask result to 8 bits
        let sum = overflowing_sum as u8;
        sum
    }

    fn add16(&mut self, a:u16, b:u16) -> u16 {
        let overflowing_sum : u32 = a as u32 + b as u32;
        // Set the appropriate flags
        self.clear_flags();
        if overflowing_sum == 0 { self.set_flag(ZERO) }
        if overflowing_sum > 0xFFFF { self.set_flag(CARRY) }
        // Mask result to 8 bits
        let sum = overflowing_sum as u16;
        sum
    }

    fn inc16(&mut self, n:u16) -> u16 {
        let overflowing_sum : u32 = n as u32 + 1;
        self.clear_flags();
        if overflowing_sum == 0 { self.set_flag(ZERO) }
        // NOTE(Lito): The manual says to NOT set the carry flag when INC
        // overflows. That doesn't sound right, but let's leave it for now
        let sum = overflowing_sum as u16;
        sum
    }

    // Z-80 CPU Instruction Set
    // ---- --- ----------- ---

    fn CCF(&mut self) { // Complement carry flag.
        if self.flag_is_set(CARRY) {
            self.unset_flag(CARRY);
        } else {
            self.set_flag(CARRY);
        }
        self.clock.tick(1)
    }

    /// 8-bit Load Commands
    /// ----- ---- --------

    /// LD   r,r         xx         4 ---- r=r
    /// Load a register r1 with another register r2.
    /// See the macro definition LDrr above.
    /// This works for registers a, b, c, d, e, h, and l!

    //TODO(Lito): there's GOTTA be a way to metaprogram most of this away.

    // a
    fn LDrr_aa(&mut self) { LDrr!(self,a,a); }
    fn LDrr_ab(&mut self) { LDrr!(self,a,b); }
    fn LDrr_ac(&mut self) { LDrr!(self,a,c); }
    fn LDrr_ad(&mut self) { LDrr!(self,a,d); }
    fn LDrr_ae(&mut self) { LDrr!(self,a,e); }
    fn LDrr_ah(&mut self) { LDrr!(self,a,h); }
    fn LDrr_al(&mut self) { LDrr!(self,a,l); }

    // b
    fn LDrr_ba(&mut self) { LDrr!(self,b,a); }
    fn LDrr_bb(&mut self) { LDrr!(self,b,b); }
    fn LDrr_bc(&mut self) { LDrr!(self,b,c); }
    fn LDrr_bd(&mut self) { LDrr!(self,b,d); }
    fn LDrr_be(&mut self) { LDrr!(self,b,e); }
    fn LDrr_bh(&mut self) { LDrr!(self,b,h); }
    fn LDrr_bl(&mut self) { LDrr!(self,b,l); }

    // c
    fn LDrr_ca(&mut self) { LDrr!(self,c,a); }
    fn LDrr_cb(&mut self) { LDrr!(self,c,b); }
    fn LDrr_cc(&mut self) { LDrr!(self,c,c); }
    fn LDrr_cd(&mut self) { LDrr!(self,c,d); }
    fn LDrr_ce(&mut self) { LDrr!(self,c,e); }
    fn LDrr_ch(&mut self) { LDrr!(self,c,h); }
    fn LDrr_cl(&mut self) { LDrr!(self,c,l); }

    // d
    fn LDrr_da(&mut self) { LDrr!(self,d,a); }
    fn LDrr_db(&mut self) { LDrr!(self,d,b); }
    fn LDrr_dc(&mut self) { LDrr!(self,d,c); }
    fn LDrr_dd(&mut self) { LDrr!(self,d,d); }
    fn LDrr_de(&mut self) { LDrr!(self,d,e); }
    fn LDrr_dh(&mut self) { LDrr!(self,d,h); }
    fn LDrr_dl(&mut self) { LDrr!(self,d,l); }

    // e
    fn LDrr_ea(&mut self) { LDrr!(self,e,a); }
    fn LDrr_eb(&mut self) { LDrr!(self,e,b); }
    fn LDrr_ec(&mut self) { LDrr!(self,e,c); }
    fn LDrr_ed(&mut self) { LDrr!(self,e,d); }
    fn LDrr_ee(&mut self) { LDrr!(self,e,e); }
    fn LDrr_eh(&mut self) { LDrr!(self,e,h); }
    fn LDrr_el(&mut self) { LDrr!(self,e,l); }

    // h
    fn LDrr_ha(&mut self) { LDrr!(self,h,a); }
    fn LDrr_hb(&mut self) { LDrr!(self,h,b); }
    fn LDrr_hc(&mut self) { LDrr!(self,h,c); }
    fn LDrr_hd(&mut self) { LDrr!(self,h,d); }
    fn LDrr_he(&mut self) { LDrr!(self,h,e); }
    fn LDrr_hh(&mut self) { LDrr!(self,h,h); }
    fn LDrr_hl(&mut self) { LDrr!(self,h,l); }

    // l
    fn LDrr_la(&mut self) { LDrr!(self,l,a); }
    fn LDrr_lb(&mut self) { LDrr!(self,l,b); }
    fn LDrr_lc(&mut self) { LDrr!(self,l,c); }
    fn LDrr_ld(&mut self) { LDrr!(self,l,d); }
    fn LDrr_le(&mut self) { LDrr!(self,l,e); }
    fn LDrr_lh(&mut self) { LDrr!(self,l,h); }
    fn LDrr_ll(&mut self) { LDrr!(self,l,l); }

/// LD   r,n         xx nn      8 ---- r=n
/// Load a register r with a constant n, read from
/// the immediate value under the progam counter
/// This works for registers a, b, c, d, e, h, and l!

    fn LDrn_a(&mut self) { LDrn!(self,a); }
    fn LDrn_b(&mut self) { LDrn!(self,b); }
    fn LDrn_c(&mut self) { LDrn!(self,c); }
    fn LDrn_d(&mut self) { LDrn!(self,d); }
    fn LDrn_e(&mut self) { LDrn!(self,e); }
    fn LDrn_h(&mut self) { LDrn!(self,h); }
    fn LDrn_l(&mut self) { LDrn!(self,l); }

/// LD   r,(HL)      xx         8 ---- r=(HL)
/// Load a register r with the memory at location given by the HL registers
    fn LDrHLm_a(&mut self) { LDrHL!(self,a); }
    fn LDrHLm_b(&mut self) { LDrHL!(self,b); }
    fn LDrHLm_c(&mut self) { LDrHL!(self,c); }
    fn LDrHLm_d(&mut self) { LDrHL!(self,d); }
    fn LDrHLm_e(&mut self) { LDrHL!(self,e); }
    fn LDrHLm_h(&mut self) { LDrHL!(self,h); }
    fn LDrHLm_l(&mut self) { LDrHL!(self,l); }

/// LD   (HL),r      7x         8 ---- (HL)=r
/// Load the location given by registers HL with contents of register r
    fn LDHLmr_a(&mut self) { LDHLr!(self,a); }
    fn LDHLmr_b(&mut self) { LDHLr!(self,b); }
    fn LDHLmr_c(&mut self) { LDHLr!(self,c); }
    fn LDHLmr_d(&mut self) { LDHLr!(self,d); }
    fn LDHLmr_e(&mut self) { LDHLr!(self,e); }
    fn LDHLmr_h(&mut self) { LDHLr!(self,h); }
    fn LDHLmr_l(&mut self) { LDHLr!(self,l); }

/// LD   (HL),n      36 nn     12 ----
/// Load the immediate value n into the location given by HL
    fn LDHLmn(&mut self) {
        let n = self.read_immediate_byte();
        self.write_hl(n);

        self.clock.tick(3);
    }

/// LD   A,(BC)      0A         8 ----
/// Load register a with the value at location BC
    fn LDABCm(&mut self) {
        let bc = self.regs.bc();
        let bc_value = self.mmu.read(bc);
        self.regs.a = bc_value;

        self.clock.tick(2);
    }

/// LD   A,(DE)      1A         8 ----
/// Load register a with the value at location DE
    fn LDADEm(&mut self) {
        let de = self.regs.de();
        let de_value = self.mmu.read(de);
        self.regs.a = de_value;

        self.clock.tick(2);
    }

/// LD   A,(nn)      FA nn nn   16 ----
/// Load register a with the value at location nn, where nn is a two-byte
/// immediate value with the least significant byte first.
    fn LDAnn(&mut self) {
        let nn = self.read_immediate_word();
        let nn_value = self.mmu.read(nn);
        self.regs.a = nn_value;
        self.clock.tick(4);
    }

/// LD   (BC),A      02         8 ----
/// Load location BC with the contents of A
    fn LDBCmA(&mut self) {
        let a = self.regs.a;
        let bc = self.regs.bc();
        self.mmu.write_byte(bc, a);
        self.clock.tick(2);
    }

/// LD   (DE),A      12         8 ----
/// Load location DE with the contents of A
    fn LDDEmA(&mut self) {
        let a = self.regs.a;
        let de = self.regs.de();
        self.mmu.write_byte(de, a);

        self.clock.tick(2);
    }

/// LD   (nn),A      EA nn nn  16 ----
/// Load location at immediate word nn with the contents of register a
    fn LDnmA(&mut self) {
        let nn = self.read_immediate_word();
        let a = self.regs.a;
        self.mmu.write_byte(nn, a);

        self.clock.tick(4);
    }

/// LDI  (HL),A      22         8 ---- (HL)=A, HL=HL+1
/// Load location HL with contents of register a, then increment HL
    fn LDIHLmA(&mut self) {
        let a = self.regs.a;
        self.write_hl(a);

        let hl = self.regs.hl();
        let hli = self.inc16(hl);
        self.regs.set_hl(hli);

        self.clock.tick(2);
    }

/// LDI  A,(HL)      2A         8 ---- A=(HL), HL=HL+1
/// Load register a with contents of location HL, then increment HL
    fn LDIAHLm(&mut self) {
        let hl_value = self.read_hl();
        self.regs.a = hl_value;

        let hl = self.regs.hl();
        let hli = self.inc16(hl);
        self.regs.set_hl(hli);

        self.clock.tick(2);
    }

/*
OPCODES
=======

# 8-bit Load Commands
# ----- ---- --------
LD   A,(FF00+n)  F0 nn     12 ---- read from io-port n (memory FF00+n)
LD   (FF00+n),A  E0 nn     12 ---- write to io-port n (memory FF00+n)
LD   A,(FF00+C)  F2         8 ---- read from io-port C (memory FF00+C)
LD   (FF00+C),A  E2         8 ---- write to io-port C (memory FF00+C)
LDD  (HL),A      32         8 ---- (HL)=A, HL=HL-1
LDD  A,(HL)      3A         8 ---- A=(HL), HL=HL-1
*/


/*
# 16-bit Load Commands
# ------ ---- --------
LD   rr,nn       x1 nn nn  12 ---- rr=nn (rr may be BC,DE,HL or SP)
LD   SP,HL       F9         8 ---- SP=HL
PUSH rr          x5        16 ---- SP=SP-2  (SP)=rr   (rr may be BC,DE,HL,AF)
POP  rr          x1        12 (AF) rr=(SP)  SP=SP+2   (rr may be BC,DE,HL,AF)

*/
/// ADD  A,r         8x         4 z0hc A=A+r
/// Add any register to A and store the result in A
    fn ADDr_a(&mut self) { ADDr!(self,a); }
    fn ADDr_b(&mut self) { ADDr!(self,b); }
    fn ADDr_c(&mut self) { ADDr!(self,c); }
    fn ADDr_d(&mut self) { ADDr!(self,d); }
    fn ADDr_e(&mut self) { ADDr!(self,e); }
    fn ADDr_h(&mut self) { ADDr!(self,h); }
    fn ADDr_l(&mut self) { ADDr!(self,l); }

/// ADD  A,n         C6 nn      8 z0hc A=A+n
/// Add the immediate value to A
    fn ADDn(&mut self) {
        let a = self.regs.a;
        let n = self.read_immediate_byte();
        self.regs.a = self.add8(a, n);
        self.clock.tick(2);
    }

/// ADD  A,(HL)      86         8 z0hc A=A+(HL)
/// Add the contents of location HL to register a
    fn ADDHL(&mut self) {
        let a = self.regs.a;
        let hl = self.read_hl();
        self.regs.a = self.add8(a, hl);
        self.clock.tick(2);
    }

/// ADC  A,r         8x         4 z0hc A=A+r+cy
/// Add the contents of register r to register a. If the carry bit is set,
/// add 1 to the result.
    fn ADCr_a(&mut self) { ADCr!(self,a); }
    fn ADCr_b(&mut self) { ADCr!(self,b); }
    fn ADCr_c(&mut self) { ADCr!(self,c); }
    fn ADCr_d(&mut self) { ADCr!(self,d); }
    fn ADCr_e(&mut self) { ADCr!(self,e); }
    fn ADCr_h(&mut self) { ADCr!(self,h); }
    fn ADCr_l(&mut self) { ADCr!(self,l); }

/// ADC  A,n         CE nn      8 z0hc A=A+n+cy
/// Add the immediate value n to register a. If the carry bit is set,
/// add 1 to the result.
    fn ADCn(&mut self) {

        let a = self.regs.a;
        let n = self.read_immediate_byte();
        self.regs.a = self.add8(a, n);
        if self.flag_is_set(CARRY) {
            self.regs.a += 1;
        }
        self.clock.tick(2);
    }

/// ADC  A,(HL)      8E         8 z0hc A=A+(HL)+cy
/// Add the contents of location HL to register a. If the carry bit is set,
/// add 1 to the result.
    fn ADCHL(&mut self) {

        let a = self.regs.a;
        let hl = self.read_hl();
        self.regs.a = self.add8(a, hl);
        if self.flag_is_set(CARRY) {
            self.regs.a += 1;
        }

        self.clock.tick(2);
    }


/// ADD  HL,rr     x9           8 -0hc HL = HL+rr     ;rr may be BC,DE,HL,SP
/// Add register pair rr to registers HL
    fn ADDHLBC(&mut self) { ADDHLrr!(self, bc); }
    fn ADDHLDE(&mut self) { ADDHLrr!(self, de); }
    fn ADDHLHL(&mut self) { ADDHLrr!(self, hl); }
    // SP is a single register, not a pair, so the macro can't be used
    fn ADDHLSP(&mut self){
        let hl = self.regs.hl();
        let sp = self.regs.sp;
        let result = self.add16(hl, sp);
        self.regs.set_hl(result);

        self.clock.tick(2);
    }

/// INC  rr        x3           8 ---- rr = rr+1      ;rr may be BC,DE,HL,SP
/// Increment register pair rr
    fn INCBC(&mut self) {
        let bc = self.regs.bc();
        let result = self.inc16(bc);
        self.regs.set_bc(result);

        self.clock.tick(2);
    }
    fn INCDE(&mut self) {
        let de = self.regs.de();
        let result = self.inc16(de);
        self.regs.set_de(result);

        self.clock.tick(2);
    }
    fn INCHL(&mut self) {
        let hl = self.regs.hl();
        let result = self.inc16(hl);
        self.regs.set_hl(result);

        self.clock.tick(2);
    }
    fn INCSP(&mut self) {
        let sp = self.regs.sp;
        let spi = self.inc16(sp);
        self.regs.sp = spi;

        self.clock.tick(2);
    }

/*

# 8-bit Arithmetic Commands
# ----- ---------- --------
SUB  r           9x         4 z1hc A=A-r
/// Subtract register r from register a
SUB  n           D6 nn      8 z1hc A=A-n
SUB  (HL)        96         8 z1hc A=A-(HL)
SBC  A,r         9x         4 z1hc A=A-r-cy
SBC  A,n         DE nn      8 z1hc A=A-n-cy
SBC  A,(HL)      9E         8 z1hc A=A-(HL)-cy
AND  r           Ax         4 z010 A=A & r
AND  n           E6 nn      8 z010 A=A & n
AND  (HL)        A6         8 z010 A=A & (HL)
XOR  r           Ax         4 z000
XOR  n           EE nn      8 z000
XOR  (HL)        AE         8 z000
OR   r           Bx         4 z000 A=A | r
OR   n           F6 nn      8 z000 A=A | n
OR   (HL)        B6         8 z000 A=A | (HL)
CP   r           Bx         4 z1hc compare A-r
CP   n           FE nn      8 z1hc compare A-n
CP   (HL)        BE         8 z1hc compare A-(HL)
INC  r           xx         4 z0h- r=r+1
INC  (HL)        34        12 z0h- (HL)=(HL)+1
DEC  r           xx         4 z1h- r=r-1
DEC  (HL)        35        12 z1h- (HL)=(HL)-1
DAA              27         4 z-0x decimal adjust akku
CPL              2F         4 -11- A = A xor FF


# 16-bit Arithmetic Commands
# ------ ---------- --------
DEC  rr        xB           8 ---- rr = rr-1      ;rr may be BC,DE,HL,SP
ADD  SP,dd     E8          16 00hc SP = SP +/- dd ;dd is 8bit signed number
LD   HL,SP+dd  F8          12 00hc HL = SP +/- dd ;dd is 8bit signed number

# Rotate and Shift Commands
# ------ --- ----- --------
RLCA           07           4 000c rotate akku left
RLA            17           4 000c rotate akku left through carry
RRCA           0F           4 000c rotate akku right
RRA            1F           4 000c rotate akku right through carry
RLC  r         CB 0x        8 z00c rotate left
RLC  (HL)      CB 06       16 z00c rotate left
RL   r         CB 1x        8 z00c rotate left through carry
RL   (HL)      CB 16       16 z00c rotate left through carry
RRC  r         CB 0x        8 z00c rotate right
RRC  (HL)      CB 0E       16 z00c rotate right
RR   r         CB 1x        8 z00c rotate right through carry
RR   (HL)      CB 1E       16 z00c rotate right through carry
SLA  r         CB 2x        8 z00c shift left arithmetic (b0=0)
SLA  (HL)      CB 26       16 z00c shift left arithmetic (b0=0)
SWAP r         CB 3x        8 z000 exchange low/hi-nibble
SWAP (HL)      CB 36       16 z000 exchange low/hi-nibble
SRA  r         CB 2x        8 z00c shift right arithmetic (b7=b7)
SRA  (HL)      CB 2E       16 z00c shift right arithmetic (b7=b7)
SRL  r         CB 3x        8 z00c shift right logical (b7=0)
SRL  (HL)      CB 3E       16 z00c shift right logical (b7=0)

# Single Bit Operation Commands
# ------ --- --------- --------
BIT  n,r       CB xx        8 z01- test bit n
BIT  n,(HL)    CB xx       12 z01- test bit n
SET  n,r       CB xx        8 ---- set bit n
SET  n,(HL)    CB xx       16 ---- set bit n
RES  n,r       CB xx        8 ---- reset bit n
RES  n,(HL)    CB xx       16 ---- reset bit n


# CPU Control Commands
# --- ------- --------
CCF            3F           4 -00c cy=cy xor 1
SCF            37           4 -001 cy=1
NOP            00           4 ---- no operation
HALT           76         N*4 ---- halt until interrupt occurs (low power)
STOP           10 00        ? ---- low power standby mode (VERY low power)
DI             F3           4 ---- disable interrupts, IME=0
EI             FB           4 ---- enable interrupts, IME=1

# Jump Commands
# ---- --------
JP   nn        C3 nn nn    16 ---- jump to nn, PC=nn
JP   HL        E9           4 ---- jump to HL, PC=HL
JP   f,nn      xx nn nn 16;12 ---- conditional jump if nz,z,nc,c
JR   PC+dd     18 dd       12 ---- relative jump to nn (PC=PC+/-7bit)
JR   f,PC+dd   xx dd     12;8 ---- conditional relative jump if nz,z,nc,c
CALL nn        CD nn nn    24 ---- call to nn, SP=SP-2, (SP)=PC, PC=nn
CALL f,nn      xx nn nn 24;12 ---- conditional call if nz,z,nc,c
RET            C9          16 ---- return, PC=(SP), SP=SP+2
RET  f         xx        20;8 ---- conditional return if nz,z,nc,c
RETI           D9          16 ---- return and enable interrupts (IME=1)
RST  n         xx          16 ---- call to 00,08,10,18,20,28,30,38
 */
    fn NOP(&mut self) {
        self.clock.tick(1);
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
            0x08 => self.LDnmSP(),
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
            0x22 => self.LDIHLmA(),
            0x23 => self.INCHL(),
            0x24 => self.INCr_h(),
            0x25 => self.DECr_h(),
            0x26 => self.LDrn_h(),
            0x27 => self.XX(),
            0x28 => self.JRZn(),
            0x29 => self.ADDHLHL(),
            0x2A => self.LDIAHLm(),
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
            0xEA => self.LDnmA(),
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
            0xFA => self.LDAnm(),
            0xFB => self.EI(),
            0xFC => self.XX(),
            0xFD => self.XX(),
            0xFE => self.CPn(),
            0xFF => self.RST38()
        }
    */
    }
}

// Register tests
#[test]
fn test_register_getting_pairs() {
    let mut cpu = Z80::new();
    assert_eq!(cpu.regs.hl(), 0x0);
    cpu.regs.h = 0x80;
    assert_eq!(cpu.regs.hl(), 0x8000);
    cpu.regs.l = 0x80;
    assert_eq!(cpu.regs.hl(), 0x8080);
}

#[test]
fn test_register_setting_hl() {
    let mut cpu = Z80::new();
    assert_eq!(cpu.regs.hl(), 0x0);
    cpu.regs.set_hl(0x8811);
    assert_eq!(cpu.regs.h, 0x88);
    assert_eq!(cpu.regs.l, 0x11);
}

// CPU tests
#[test]
fn test_the_clock_moves_forward_in_time() {
    let mut cpu = Z80::new();
    cpu.clock.tick(1);
    assert_eq!(cpu.clock.m, 1);
    assert_eq!(cpu.clock.t, 4);
    cpu.clock.tick(4);
    assert_eq!(cpu.clock.m, 1+4);
    assert_eq!(cpu.clock.t, 4+16);
}


#[test]
fn test_incrementing_16_bit_numbers() {
    let mut cpu = Z80::new();
    // Boring case - inc l
    let i = cpu.inc16(0x0);
    assert_eq!(i, 0x1);
    // Overflow
    let almost_overflowing = 0xFFFF;
    let overflowing = cpu.inc16(almost_overflowing);
    assert_eq!(overflowing, 0x0);
}

#[test]
fn test_setting_CPU_flags() {
    let mut cpu = Z80::new();
    cpu.set_flag(CARRY);
    assert!(cpu.flag_is_set(CARRY));
    cpu.set_flag(ZERO);
    assert!(cpu.flag_is_set(ZERO));
}

#[test]
fn test_the_alu_adds_8_bit_numbers() {
    let mut cpu = Z80::new();
    let added = cpu.add8(200, 100);
    assert_eq!(added, 44);
    assert!(cpu.flag_is_set(CARRY));
}

#[test]
fn test_the_alu_adds_16_bit_numbers() {
    let mut cpu = Z80::new();
    let added = cpu.add16(65535, 50);
    assert_eq!(added, 49);
    assert!(cpu.flag_is_set(CARRY));
}

#[test]
fn test_setting_flags() {
    let mut cpu = Z80::new();
    assert!(!cpu.flag_is_set(CARRY));
    cpu.set_flag(CARRY);
    assert!(cpu.flag_is_set(CARRY));
}

#[test]
fn test_unsetting_flags() {
    let mut cpu = Z80::new();
    cpu.set_flag(CARRY);
    cpu.unset_flag(CARRY);
    assert!(!cpu.flag_is_set(CARRY));
}

// OPCODES:
// 8-bit loads

// Because a macro is used to generate this code,
// we hope we can get away with just testing one case.
// NOTE that this is dangerous, because it won't catch if
// we've forgotten to generate a case!
#[test]
fn test_the_instruction_set_can_LDrr() {
    let mut cpu = Z80::new();
    cpu.regs.b = 0x01;
    cpu.LDrr_ab();
    assert_eq!(cpu.regs.a, 0x01);
    assert_eq!(cpu.clock.t, 4);
}

#[test]
fn test_the_instruction_set_can_LDrn() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x05;
    // At the start of memory (should be in ROM, but... we can't write there)
    cpu.regs.pc = 0xC000;
    cpu.LDrn_a();
    // MMU is filled with zeros, so expect regs.a to be zeros now, too
    assert_eq!(cpu.regs.a, 0x00);
    assert_eq!(cpu.regs.pc, 0xC001);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDrHLm() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x01;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.mmu.write_byte(0xC001, 0x05);
    cpu.LDrHLm_a();
    assert_eq!(cpu.regs.a, 0x05);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDHLmr() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x01;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.LDHLmr_a();
    assert_eq!(cpu.mmu.read(0xC001), 0x01);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDHLmn() {
    let mut cpu = Z80::new();
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x05;
    cpu.regs.pc = 0xC000;
    cpu.mmu.write_byte(0xC000, 0x01);
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDHLmn();
    assert_eq!(cpu.mmu.read(0xC005), 0x01);
    assert_eq!(cpu.clock.t, 12);
}

#[test]
fn test_the_instruction_set_can_LDABCm() {
    let mut cpu = Z80::new();
    cpu.regs.b = 0xC0;
    cpu.regs.c = 0x05;
    cpu.regs.a = 0x01;
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDABCm();
    assert_eq!(cpu.regs.a, 0x02);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDADEm() {
    let mut cpu = Z80::new();
    cpu.regs.d = 0xC0;
    cpu.regs.e = 0x05;
    cpu.regs.a = 0x01;
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDADEm();
    assert_eq!(cpu.regs.a, 0x02);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDAnn() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x01; // prime a with 1, so the failing case is more obvious
    cpu.regs.pc = 0xC000;
    cpu.mmu.write_byte(0xC000, 0x05);
    cpu.mmu.write_byte(0xC001, 0xC0);
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDAnn();
    assert_eq!(cpu.regs.a, 0x02);
    assert_eq!(cpu.clock.t, 16);
}

#[test]
fn test_the_instruction_set_can_LDBCmA() {
    let mut cpu = Z80::new();
    cpu.regs.b = 0xC0;
    cpu.regs.c = 0x05;
    cpu.regs.a = 0x01;
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDBCmA();
    assert_eq!(cpu.mmu.read(0xC005), 0x01);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDDEmA() {
    let mut cpu = Z80::new();
    cpu.regs.d = 0xC0;
    cpu.regs.e = 0x05;
    cpu.regs.a = 0x01;
    cpu.mmu.write_byte(0xC005, 0x02);
    cpu.LDDEmA();
    assert_eq!(cpu.mmu.read(0xC005), 0x01);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDnmA() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x04;
    cpu.regs.pc = 0xC000;
    cpu.mmu.write_byte(0xC000, 0x05);
    cpu.mmu.write_byte(0xC001, 0xC0);
    cpu.mmu.write_byte(0xC005, 0x01);
    cpu.LDnmA();
    assert_eq!(cpu.regs.a, 0x04);
    assert_eq!(cpu.clock.t, 16);
}

#[test]
fn test_the_instruction_set_can_LDIHLmA() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x01;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.mmu.write_byte(0xC001, 0x05);
    cpu.LDIHLmA();
    assert_eq!(cpu.mmu.read(0xC001), 0x01);
    assert_eq!(cpu.regs.l, 0x02);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_LDIAHLm() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x01;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.mmu.write_byte(0xC001, 0x05);
    cpu.LDIAHLm();
    assert_eq!(cpu.regs.a, 0x05);
    assert_eq!(cpu.regs.l, 0x02);
    assert_eq!(cpu.clock.t, 8);
}


// 16-bit loads

#[test]
fn test_the_instruction_set_can_ADDr() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.b = 0xC8;
    cpu.ADDr_b();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2C); // = 256 (carry bit) + 44
    assert_eq!(cpu.clock.t, 4);
}

#[test]
fn test_the_instruction_set_can_ADDn() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.pc = 0xC000;
    cpu.mmu.write_byte(0xC000, 0xC8);
    cpu.ADDn();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2C);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_ADDHL() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.mmu.write_byte(0xC001, 0xC8);
    cpu.ADDHL();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2C);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_ADCr_b() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.b = 0xC8;
    cpu.mmu.write_byte(0xC001, 0xC8);
    cpu.ADCr_b();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2D);
    assert_eq!(cpu.clock.t, 4);
}

#[test]
fn test_the_instruction_set_can_ADCn() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.pc = 0xC000;
    cpu.mmu.write_byte(0xC000, 0xC8);
    cpu.ADCn();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2D);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_ADCHL() {
    let mut cpu = Z80::new();
    cpu.regs.a = 0x64;
    cpu.regs.h = 0xC0;
    cpu.regs.l = 0x01;
    cpu.mmu.write_byte(0xC001, 0xC8);
    cpu.ADCHL();
    assert!(cpu.flag_is_set(CARRY));
    assert_eq!(cpu.regs.a, 0x2D);
    assert_eq!(cpu.clock.t, 8);
}

// 16-bit arithmetic
#[test]
fn test_the_instruction_set_can_ADDHLrr() {
    let mut cpu = Z80::new();
    cpu.regs.h = 0x10;
    cpu.regs.l = 0x01;
    cpu.regs.b = 0x11;
    cpu.regs.c = 0x02;
    cpu.ADDHLBC();
    assert_eq!(cpu.regs.hl(), 0x2103);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_ADDHLSP() { // Non-macro logic, so double-check
    let mut cpu = Z80::new();
    cpu.regs.h = 0x10;
    cpu.regs.l = 0x01;
    cpu.regs.sp = 0x1102;
    cpu.ADDHLSP();
    assert_eq!(cpu.regs.hl(), 0x2103);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_INCrr() {
    let mut cpu = Z80::new();
    cpu.regs.h = 0x10;
    cpu.regs.l = 0x01;
    cpu.INCHL();
    assert_eq!(cpu.regs.hl(), 0x1002);
    assert_eq!(cpu.clock.t, 8);
}

#[test]
fn test_the_instruction_set_can_INCSP() {
    let mut cpu = Z80::new();
    cpu.regs.sp = 0x1102;
    cpu.INCSP();
    assert_eq!(cpu.regs.sp, 0x1103);
    assert_eq!(cpu.clock.t, 8);
}


#[test]
fn test_the_instruction_set_can_CCF() {
    let mut cpu = Z80::new();
    cpu.CCF();
    assert!( cpu.flag_is_set(CARRY) );
    assert_eq!(cpu.clock.t, 4);
    cpu.CCF();
    assert!( !cpu.flag_is_set(CARRY) );
    assert_eq!(cpu.clock.t, 8);
}
