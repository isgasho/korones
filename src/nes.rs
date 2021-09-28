use crate::cpu::Cpu;

#[derive(Debug)]
pub(crate) struct Nes {
    pub(crate) cpu: Cpu,
    pub(crate) wram: [u8; 0x07FF],
    pub(crate) cpu_cycles: u128,
}

impl Nes {
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self {
            cpu: Default::default(),
            wram: [0; 0x07FF],
            cpu_cycles: 0,
        }
    }
}
