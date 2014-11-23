// Wolves and the Ravens - Rogue Valley
// Holding - Grouper 
// In for the Kill - Billie Marten
// Sam Brooks
// Jose Gonzales - Stay in the Shade
// Lo-Fang

use mmu;

struct RegisterSet {
    // 8-bit registers
    a: u8, // TODO: check with ADDr and stuff
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    // `Flags` register 
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
}

impl Z80 {
    fn reset(&mut self) {
        self._r.a = 0; self._r.b = 0; self._r.c = 0; self._r.d = 0; 
        self._r.e = 0; self._r.h = 0; self._r.l = 0; self._r.f = 0; 
        self._r.sp = 0; 
        self._r.pc = 0; 

        selself._clock.m = 0; 
        self._clock.t = 0; 
    }

    fn NOP(&mut self) {
        // Time taken
        self._r.m = 1;
        self._r.t = 4;
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
        if !(i & 255) {
            self._r.f |= 0x80;
        }
        if i < 0 {
            self._r.m |= 0x10;
        }

        self._r.m = 1; 
        self._r.t = 4;
    }

    fn PUSHBC(&mut self) { // Push B and C to the stack
        self._r.sp -= 1;
        mmu.wb(self._r.sp, self._r.b);
        self._r.sp -= 1;
        mmu.wb(self._r.sp, self._r.c);

        self._r.m = 3; 
        self._r.t = 12;
    }

    fn POPHL(&mut self) {
        self._r.l = mmu.rb(self._r.sp); // read from stack pointer address
        self._r.sp += 1;
        self._r.h = mmu.rb(self._r.sp);
        self._r.sp += 1;

        self._r.m = 3;
        self._r.t = 12;
    }

    fn LDAmm(&mut self) { // Read from location into A
        let addr = mmu.rw(self._r.pc);
        self._r.pc += 2; // It's a full 16 bits, so advance twice
        self._r.a = mmu.rb(addr);

        self._r.m = 4;
        self._r.t = 16;
    }
}


fn main() {
    let mut cpu = Z80 {
        _clock: Clock {
            m: 0, t: 0
        },

        _r: RegisterSet {
            a: 0, f: 0, b: 0, c: 0, d:0, e: 0, h: 0, l: 0,
            pc: 0, sp: 0,
            m: 0, t: 0
        },
    };

    /*
    loop {
        let op = cpu._mmu.rb(Z80._r.pc);
        cpu._map[op]()
        cpu._r.pc += 1;
    }
    */
}
