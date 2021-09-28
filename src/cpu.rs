use crate::nes::Nes;
use crate::Emu;

mod addressing_mode;
mod decoder;
mod instruction;

#[cfg(test)]
mod instruction_test;

use decoder::{AddressingMode, Instruction, Mnemonic};

#[derive(Debug, Default)]
pub struct Cpu {
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    p: Status,
    pc: u16,
}

impl Emu {
    #[allow(dead_code)]
    fn cpu_step<B: CpuBus, T: CpuTick>(nes: &mut Nes) {
        use decoder::decode;

        let opcode = read::<B, T>(nes, nes.cpu.pc);
        nes.cpu.pc = nes.cpu.pc.wrapping_add(1);

        let inst = decode(opcode);

        let (_, addressing_mode) = &inst;
        let operand = addressing_mode::get_operand::<B, T>(nes, *addressing_mode);

        instruction::execute::<B, T>(nes, inst, operand);
    }
}

bitflags! {
    #[derive(Default)]
    struct Status: u8 {
        // Carry
        const C = 1;
        // Zero
        const Z = 1 << 1;
        // Interrupt Disable
        const I = 1 << 2;
        // Decimal
        const D = 1 << 3;
        // Overflow
        const V = 1 << 6;
        // Negative
        const N = 1 << 7;
        // B flags
        const INTERRUPT_B = 0b00100000;
        const INSTRUCTION_B = 0b00110000;
    }
}

impl Status {
    fn set_zn(&mut self, v: u8) {
        self.set(Self::Z, v == 0);
        self.set(Self::N, v & 0x80 == 0x80);
    }
}

pub(crate) trait CpuBus {
    fn read(nes: &mut Nes, addr: u16) -> u8;
    fn write(nes: &mut Nes, addr: u16, value: u8);
}

pub(crate) trait CpuTick {
    fn tick(nes: &mut Nes);
    fn tick_n(nes: &mut Nes, n: u128);
}

struct CpuBusInternal<B: CpuBus, T: CpuTick> {
    _bus: std::marker::PhantomData<B>,
    _tick: std::marker::PhantomData<T>,
}

impl<B: CpuBus, T: CpuTick> CpuBus for CpuBusInternal<B, T> {
    fn read(nes: &mut Nes, addr: u16) -> u8 {
        let v = B::read(nes, addr);
        T::tick(nes);
        v
    }

    fn write(nes: &mut Nes, addr: u16, value: u8) {
        //TODO OAMDMA
        B::write(nes, addr, value);
        T::tick(nes);
    }
}

fn read<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u8 {
    CpuBusInternal::<B, T>::read(nes, addr)
}

fn write<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16, value: u8) {
    CpuBusInternal::<B, T>::write(nes, addr, value)
}

fn read_word<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    CpuBusInternal::<B, T>::read(nes, addr) as u16
        | (CpuBusInternal::<B, T>::read(nes, addr + 1) as u16) << 8
}

fn read_on_indirect<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    let low = CpuBusInternal::<B, T>::read(nes, addr) as u16;
    // Reproduce 6502 bug - http://nesdev.com/6502bugs.txt
    let high = CpuBusInternal::<B, T>::read(nes, (addr & 0xFF00) | ((addr + 1) & 0x00FF)) as u16;
    low | (high << 8)
}

fn push_stack<B: CpuBus, T: CpuTick>(nes: &mut Nes, v: u8) {
    write::<B, T>(nes, nes.cpu.s as u16, v);
    nes.cpu.s = nes.cpu.s.wrapping_sub(1);
}

fn pull_stack<B: CpuBus, T: CpuTick>(nes: &mut Nes) -> u8 {
    nes.cpu.s = nes.cpu.s.wrapping_add(1);
    read::<B, T>(nes, nes.cpu.s as u16)
}

fn push_stack_word<B: CpuBus, T: CpuTick>(nes: &mut Nes, v: u16) {
    push_stack::<B, T>(nes, (v >> 8) as u8);
    push_stack::<B, T>(nes, (v & 0xFF) as u8);
}

fn pull_stack_word<B: CpuBus, T: CpuTick>(nes: &mut Nes) -> u16 {
    let low = pull_stack::<B, T>(nes) as u16;
    let high = pull_stack::<B, T>(nes) as u16;
    low | (high << 8)
}

fn page_crossed(a: u16, b: u16) -> bool {
    a.wrapping_add(b) & 0xFF00 != (b & 0xFF00)
}

#[cfg(test)]
mod test_mock {
    use super::*;

    pub(super) struct CpuTickMock {}
    impl CpuTick for CpuTickMock {
        fn tick(nes: &mut Nes) {
            nes.cpu_cycles = nes.cpu_cycles.wrapping_add(1);
        }
        fn tick_n(nes: &mut Nes, n: u128) {
            nes.cpu_cycles = nes.cpu_cycles.wrapping_add(n);
        }
    }

    pub(super) struct CpuBusMock {}
    impl CpuBus for CpuBusMock {
        fn read(nes: &mut Nes, addr: u16) -> u8 {
            nes.wram[addr as usize]
        }
        fn write(nes: &mut Nes, addr: u16, value: u8) {
            nes.wram[addr as usize] = value
        }
    }
}
