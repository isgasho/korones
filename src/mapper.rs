pub(crate) trait Mapper: std::fmt::Debug {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}

#[derive(Debug)]
pub(crate) struct Empty {}

impl Mapper for Empty {
    fn read(&mut self, _addr: u16) -> u8 {
        0
    }
    fn write(&mut self, _addr: u16, _value: u8) {}
}
