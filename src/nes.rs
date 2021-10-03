use crate::cpu::{Cpu, CpuBus};
use crate::mapper::{Empty, Mapper};

#[derive(Debug)]
pub(crate) struct Nes {
    pub(crate) cpu: Cpu,
    pub(crate) wram: [u8; 0x07FF],
    pub(crate) cpu_cycles: u128,

    pub(crate) mapper: Box<dyn Mapper>,
}

impl Nes {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            cpu: Default::default(),
            wram: [0; 0x07FF],
            cpu_cycles: 0,
            mapper: Box::new(Empty {}),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Mirroring {
    Horizontal,
    Vertical,
}

pub(crate) struct Bus {}

impl CpuBus for Bus {
    fn read(nes: &mut Nes, addr: u16) -> u8 {
        match addr {
            0x0000..=0x07FF => nes.wram[addr as usize],
            0x0800..=0x1FFF => nes.mapper.read(addr - 0x0800),
            //TODO ppu, apu, controllers
            _ => 0,
        }
    }

    fn write(nes: &mut Nes, addr: u16, value: u8) {
        match addr {
            0x0000..=0x07FF => nes.wram[addr as usize] = value,
            0x0800..=0x1FFF => nes.mapper.write(addr - 0x0800, value),
            //TODO ppu, apu, controllers
            _ => {}
        }
    }
}
