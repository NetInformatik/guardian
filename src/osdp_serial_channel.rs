use embassy_sync::pipe::Writer;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pipe::Reader};
use libosdp::{Channel, ChannelError};

pub struct SerialChannel<'a> {
    pub uart_number: u8,
    pub channel_writer: Writer<'a, CriticalSectionRawMutex, 256>,
    pub channel_reader: Reader<'a, CriticalSectionRawMutex, 256>,
}

impl<'a> SerialChannel<'a> {
    pub fn new(
        uart_number: u8,
        channel_writer: Writer<'a, CriticalSectionRawMutex, 256>,
        channel_reader: Reader<'a, CriticalSectionRawMutex, 256>,
    ) -> Self {
        Self {
            uart_number,
            channel_writer,
            channel_reader,
        }
    }
}

impl<'a> Channel for SerialChannel<'a> {
    fn get_id(&self) -> i32 {
        self.uart_number as i32
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ChannelError> {
        // Attempt to read from the pipe
        match self.channel_reader.try_read(buf) {
            Ok(size) => Ok(size),
            Err(_) => Err(ChannelError::WouldBlock),
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ChannelError> {
        // Attempt to write to the pipe
        match self.channel_writer.try_write(buf) {
            Ok(size) => Ok(size),
            Err(_) => Err(ChannelError::WouldBlock),
        }
    }

    fn flush(&mut self) -> Result<(), ChannelError> {
        return Ok(());
    }
}
