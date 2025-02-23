use libosdp::{Channel, ChannelError};

use crate::osdp_serial_driver::OsdpSerialDriver;

pub struct SerialChannel<S>
where
    S: OsdpSerialDriver + Send + Sync,
{
    pub uart_number: u8,
    pub serial_driver: S,
}

impl<S> SerialChannel<S>
where
    S: OsdpSerialDriver + Send + Sync,
{
    pub fn new(uart_number: u8, serial_driver: S) -> SerialChannel<S> {
        // Initialize Serial Port
        SerialChannel {
            uart_number,
            serial_driver,
        }
    }
}

impl<S> Channel for SerialChannel<S>
where
    S: OsdpSerialDriver + Send + Sync,
{
    fn get_id(&self) -> i32 {
        todo!()
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ChannelError> {
        todo!()
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ChannelError> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), ChannelError> {
        todo!()
    }
}