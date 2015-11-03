extern crate gb;

fn main() {
    let mut cpu = gb::cpu::Z80::new();
    /*
    // Construct a CPU
    let mut CPU = cpu::Z80::new();

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
        // GPU.step();
    }
    */
}
