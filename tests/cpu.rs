#![feature(plugin,const_fn)]
#![plugin(stainless)]

extern crate gb;
use gb::{cpu, gpu, mmu};

#[test]
fn test_the_cpu() {
    cpu::Z80::new();
}


// describe! the_CPU {
//     before_each {
//         pub use gb::{cpu, gpu, mmu};
//         let mut cpu = gb::cpu::Z80::new();
//     }
//
//     describe! the_clock {
//         it "moves forward in time" {
//             cpu.clock.tick(1);
//             assert_eq!(cpu.clock.m, 1);
//             assert_eq!(cpu.clock.t, 4);
//             cpu.clock.tick(4);
//             assert_eq!(cpu.clock.m, 1+4);
//             assert_eq!(cpu.clock.t, 4+16);
//         }
//     }
// }
//
//     describe! the_ALU {
//         it "adds 8-bit numbers" {
//         }
//
//         it "adds 16-bit numbers" {
//         }
//     }
//     describe! the_instruction_set {
//         it "can ADCHLss" {
//         }
//     }
// }
