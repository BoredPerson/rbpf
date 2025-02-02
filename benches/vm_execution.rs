// Copyright 2020 Solana Maintainers <maintainers@solana.com>
//
// Licensed under the Apache License, Version 2.0 <http://www.apache.org/licenses/LICENSE-2.0> or
// the MIT license <http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#![feature(test)]

extern crate solana_rbpf;
extern crate test;

use solana_rbpf::{
    assembler::assemble,
    user_error::UserError,
    vm::{Config, DefaultInstructionMeter, EbpfVm, Executable},
};
use std::{fs::File, io::Read};
use test::Bencher;
use test_utils::TestInstructionMeter;

#[bench]
fn bench_init_interpreter_execution(bencher: &mut Bencher) {
    let mut file = File::open("tests/elfs/pass_stack_reference.so").unwrap();
    let mut elf = Vec::new();
    file.read_to_end(&mut elf).unwrap();
    let executable = <dyn Executable<UserError, DefaultInstructionMeter>>::from_elf(
        &elf,
        None,
        Config::default(),
    )
    .unwrap();
    let mut vm =
        EbpfVm::<UserError, DefaultInstructionMeter>::new(executable.as_ref(), &mut [], &[])
            .unwrap();
    bencher.iter(|| {
        vm.execute_program_interpreted(&mut DefaultInstructionMeter {})
            .unwrap()
    });
}

#[cfg(not(windows))]
#[bench]
fn bench_init_jit_execution(bencher: &mut Bencher) {
    let mut file = File::open("tests/elfs/pass_stack_reference.so").unwrap();
    let mut elf = Vec::new();
    file.read_to_end(&mut elf).unwrap();
    let mut executable = <dyn Executable<UserError, DefaultInstructionMeter>>::from_elf(
        &elf,
        None,
        Config::default(),
    )
    .unwrap();
    executable.jit_compile().unwrap();
    let mut vm =
        EbpfVm::<UserError, DefaultInstructionMeter>::new(executable.as_ref(), &mut [], &[])
            .unwrap();
    bencher.iter(|| {
        vm.execute_program_jit(&mut DefaultInstructionMeter {})
            .unwrap()
    });
}

fn bench_jit_vs_interpreter(
    bencher: &mut Bencher,
    assembly: &str,
    instruction_meter: u64,
    mem: &mut [u8],
) {
    let program = assemble(assembly).unwrap();
    let mut executable = <dyn Executable<UserError, TestInstructionMeter>>::from_text_bytes(
        &program,
        None,
        Config::default(),
    )
    .unwrap();
    executable.jit_compile().unwrap();
    let mut vm = EbpfVm::new(executable.as_ref(), mem, &[]).unwrap();
    let interpreter_summary = bencher
        .bench(|bencher| {
            bencher.iter(|| {
                let result = vm.execute_program_interpreted(&mut TestInstructionMeter {
                    remaining: instruction_meter,
                });
                assert!(result.is_ok());
                assert_eq!(vm.get_total_instruction_count(), instruction_meter);
            });
        })
        .unwrap();
    let jit_summary = bencher
        .bench(|bencher| {
            bencher.iter(|| {
                let result = vm.execute_program_jit(&mut TestInstructionMeter {
                    remaining: instruction_meter,
                });
                assert!(result.is_ok());
                assert_eq!(vm.get_total_instruction_count(), instruction_meter);
            });
        })
        .unwrap();
    println!(
        "jit_vs_interpreter_ratio={}",
        interpreter_summary.mean / jit_summary.mean
    );
}

#[cfg(not(windows))]
#[bench]
fn bench_jit_vs_interpreter_address_translation(bencher: &mut Bencher) {
    bench_jit_vs_interpreter(
        bencher,
        "
    mov r1, r2
    and r1, 1023
    ldindb r1, 0
    add r2, 1
    jlt r2, 0x10000, -5
    exit",
        327681,
        &mut [0; 1024],
    );
}

#[cfg(not(windows))]
#[bench]
fn bench_jit_vs_interpreter_empty_for_loop(bencher: &mut Bencher) {
    bench_jit_vs_interpreter(
        bencher,
        "
    mov r1, r2
    and r1, 1023
    add r2, 1
    jlt r2, 0x10000, -4
    exit",
        262145,
        &mut [0; 0],
    );
}
