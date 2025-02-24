use std::{
    sync::mpmc::{Receiver, Sender},
    thread,
    time::Duration,
};

use libosdp::{Channel, ChannelError};

pub struct SerialChannel {
    pub uart_number: u8,
    pub channel_sender: Sender<u8>,
    pub channel_receiver: Receiver<u8>,
}

impl SerialChannel {
    pub fn new(uart_number: u8, channel_writer: Sender<u8>, channel_reader: Receiver<u8>) -> Self {
        Self {
            uart_number,
            channel_sender: channel_writer,
            channel_receiver: channel_reader,
        }
    }
}

impl Channel for SerialChannel {
    fn get_id(&self) -> i32 {
        self.uart_number as i32
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ChannelError> {
        // While there is data in the pipe, read it
        let mut i = 0;
        while i < buf.len() {
            match self.channel_receiver.try_recv() {
                Ok(byte) => {
                    buf[i] = byte;
                    i += 1;
                }
                Err(error) => match error {
                    std::sync::mpmc::TryRecvError::Empty => {
                        break;
                    }
                    std::sync::mpmc::TryRecvError::Disconnected => {
                        println!("ERROR: OSDP Serial RX Queue Disconnected!");
                        return Err(ChannelError::TransportError);
                    }
                },
            }
        }
        Ok(i)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, ChannelError> {
        // For each byte in the buffer, write it to the pipe
        let mut i = 0;
        for byte in buf.iter() {
            match self.channel_sender.try_send(*byte) {
                Ok(_) => {
                    i += 1;
                }
                Err(error) => match error {
                    std::sync::mpmc::TrySendError::Full(_) => {
                        break;
                    }
                    std::sync::mpmc::TrySendError::Disconnected(_) => {
                        println!("ERROR: OSDP Serial TX Queue Disconnected!");
                        return Err(ChannelError::TransportError);
                    }
                },
            }
        }
        if i == 0 {
            return Err(ChannelError::WouldBlock);
        }
        Ok(i)
    }

    fn flush(&mut self) -> Result<(), ChannelError> {
        loop {
            if self.channel_sender.is_empty() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(1));
        }
    }
}
