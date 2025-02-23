use libosdp::OsdpError;

pub trait OsdpSerialDriver {
    fn write(&mut self, word: u8) -> Result<(), OsdpError>;
    fn read(&mut self) -> Result<u8, OsdpError>;
    fn flush(&mut self) -> Result<(), OsdpError>;
}