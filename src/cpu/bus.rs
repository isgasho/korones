use super::*;

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

pub(super) fn read<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u8 {
    CpuBusInternal::<B, T>::read(nes, addr)
}

pub(super) fn write<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16, value: u8) {
    CpuBusInternal::<B, T>::write(nes, addr, value)
}

pub(super) fn read_word<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    CpuBusInternal::<B, T>::read(nes, addr) as u16
        | (CpuBusInternal::<B, T>::read(nes, addr + 1) as u16) << 8
}

pub(super) fn read_on_indirect<B: CpuBus, T: CpuTick>(nes: &mut Nes, addr: u16) -> u16 {
    let low = CpuBusInternal::<B, T>::read(nes, addr) as u16;
    // Reproduce 6502 bug - http://nesdev.com/6502bugs.txt
    let high = CpuBusInternal::<B, T>::read(nes, (addr & 0xFF00) | ((addr + 1) & 0x00FF)) as u16;
    low | (high << 8)
}
