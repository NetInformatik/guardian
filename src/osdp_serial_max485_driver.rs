use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::{gpio::{AnyOutputPin}, uart::UartDriver};
use libosdp::{Channel, ChannelError};

use crate::osdp_serial_driver::OsdpSerialDriver;

pub struct OsdpSerialMax485Driver<'a>
{
    pub uart: UartDriver<'a>,
    pub rede_pin: AnyOutputPin,
}

impl OsdpSerialMax485Driver<'a>
{
    pub fn new(uart: UartDriver, rede_pin: AnyOutputPin) -> OsdpSerialMax485Driver {
        // Initialize Serial Port
        OsdpSerialMax485Driver {
            uart,
            rede_pin
        }
    }
}

impl OsdpSerialDriver for OsdpSerialMax485Driver<'a>
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