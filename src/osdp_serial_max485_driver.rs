use std::sync::{Arc, Mutex};

use embedded_hal_0_2::{digital::v2::OutputPin, serial::{Read, Write}};
use libosdp::{Channel, ChannelError};
use max485::Max485;

use crate::osdp_serial_driver::OsdpSerialDriver;

pub struct OsdpSerialMax485Driver<RIDO, REDE>
where
    RIDO: Read<u8> + Write<u8>,
    REDE: OutputPin,
{
    pub max485: Arc<Mutex<Max485<RIDO, REDE>>>,
}

impl<RIDO, REDE> OsdpSerialMax485Driver<RIDO, REDE>
where
    RIDO: Read<u8> + Write<u8>,
    REDE: OutputPin,
{
    pub fn new(max485: Max485<RIDO, REDE>) -> OsdpSerialMax485Driver<RIDO, REDE> {
        let max485 = Arc::new(Mutex::new(max485));
        // Initialize Serial Port
        OsdpSerialMax485Driver {
            max485,
        }
    }
}

impl<RIDO, REDE> OsdpSerialDriver for OsdpSerialMax485Driver<RIDO, REDE>
where
    RIDO: Read<u8> + Write<u8>,
    REDE: OutputPin,
{
    fn read(&mut self) -> Result<u8, libosdp::OsdpError> {
        todo!()
    }

    fn write(&mut self, word: u8) -> Result<(), libosdp::OsdpError> {
        todo!()
    }

    fn flush(&mut self) -> Result<(), libosdp::OsdpError> {
        todo!()
    }
}