#![feature(macro_rules)]

extern crate log;
mod cpu;
mod mmu;
mod gpu;

fn main() {
    // Construct a CPU
    let mut CPU = cpu::Z80 {
        _clock: cpu::Clock {
            m: 0, t: 0
        },

        _r: cpu::RegisterSet {
            a: 0, f: 0, b: 0, c: 0, d:0, e: 0, h: 0, l: 0,
            pc: 0, sp: 0,
            m: 0, t: 0
        },
        MMU: mmu::MMU,
    };

    loop {
        // Read an instruction from memory
        let op = CPU._mmu.rb(CPU._r.pc);
        // Execute
        CPU.call(op);
        CPU._r.pc += 1;
        // Keep track of how much time the instruction took
        CPU._clock.m += CPU._r.m;
        CPU._clock.t += CPU._r.t;
        // Keep track of how long it took to redraw the screen 
        GPU.step();
    }
}
