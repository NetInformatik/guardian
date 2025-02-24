use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::gpio::{Gpio0, Gpio1, InputPin, OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::uart::{config, UartDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::ESP_ERR_TIMEOUT;
use libosdp::{ControlPanel, OsdpEvent, PdInfoBuilder};
use osdp_serial_channel::SerialChannel;

mod osdp_serial_channel;
mod osdp_utils;

// Pipes for OSDP Serial TX & RX
static mut OSDP_SERIAL_TX_PIPE: Pipe<CriticalSectionRawMutex, 256> = Pipe::new();
static mut OSDP_SERIAL_RX_PIPE: Pipe<CriticalSectionRawMutex, 256> = Pipe::new();

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Report Start
    println!("Initializing Guardian...");

    // Retrieve periherals
    let peripherals = Peripherals::take().unwrap();

    // Initialize UART for OSDP
    let osdp_uart_tx = peripherals.pins.gpio32.downgrade_output();
    let osdp_uart_rx = peripherals.pins.gpio12.downgrade_input();
    let osdp_uart_config = config::Config::new().baudrate(Hertz(9_600));
    let osdp_uart = UartDriver::new(
        peripherals.uart1,
        osdp_uart_tx,
        osdp_uart_rx,
        Option::<Gpio0>::None,
        Option::<Gpio1>::None,
        &osdp_uart_config,
    )
    .unwrap();
    println!("OSDP UART Initialized");

    // Setup MAX485 REDE Pin
    let osdp_max485_rede_output = peripherals.pins.gpio4.downgrade_output();
    let mut osdp_max485_rede = PinDriver::output(osdp_max485_rede_output).unwrap();
    print!("OSDP MAX485 REDE Pin Initialized");

    // Initialize Unlock Pin
    let unlock_pin_output = peripherals.pins.gpio15.downgrade_output();
    let mut unlock_pin = PinDriver::output(unlock_pin_output).unwrap();
    println!("Door unlock Pin Initialized");

    // Prepare Settings
    let allowed_card_id = vec![192, 77, 43, 64];

    // Split OSDP Serial TX & RX Pipes
    let (osdp_serial_tx_reader, osdp_serial_tx_writer) = unsafe { OSDP_SERIAL_TX_PIPE.split() };
    let (osdp_serial_rx_reader, osdp_serial_rx_writer) = unsafe { OSDP_SERIAL_RX_PIPE.split() };
    println!("OSDP Serial Pipes Initialized");

    // Setup Serial Port
    let serial_channel = Box::new(SerialChannel::new(
        1,
        osdp_serial_tx_writer,
        osdp_serial_rx_reader,
    ));
    println!("OSDP Serial Channel Initialized");

    // Create thread to handle serial communication
    thread::spawn(move || {
        loop {
            // Read from UART
            let mut read_buf = [0u8; 256];
            match osdp_uart.read(&mut read_buf, delay::NON_BLOCK) {
                Ok(_) => {
                    // Write to OSDP Serial RX Pipe
                    match osdp_serial_rx_writer.try_write(&read_buf) {
                        Ok(_) => {}
                        Err(error) => {
                            println!(
                                "Error sending received serial data to OSDP Serial RX Pipe: {:?}",
                                error
                            );
                        }
                    }
                }
                // If the EspError is not a timeout, log it
                Err(error) => {
                    if error.code() != ESP_ERR_TIMEOUT {
                        println!("Error Reading ESP UART Serial: {:?}", error);
                    }
                }
            }

            // Read from OSDP Serial TX Pipe
            let mut write_buf = [0u8; 256];
            match osdp_serial_tx_reader.try_read(&mut write_buf) {
                Ok(_) => {
                    // Set MAX485 REDE Pin to High
                    osdp_max485_rede.set_high().unwrap();

                    // Write to UART
                    osdp_uart.write(&write_buf).unwrap();

                    // Wait for UART to finish transmitting
                    osdp_uart.wait_tx_done(delay::BLOCK).unwrap();

                    // Set MAX485 REDE Pin to Low
                    osdp_max485_rede.set_low().unwrap();
                }
                Err(_) => {}
            }

            // Sleep for 1ms
            thread::sleep(Duration::from_millis(1));
        }
    });
    println!("OSDP Serial Thread Initialized");

    // Prepare Peripheral Device(s) Info
    let mut initial_pd_builder = PdInfoBuilder::new();
    initial_pd_builder = initial_pd_builder.channel(serial_channel);
    let pd_info = initial_pd_builder.build();
    let pd_infos = vec![pd_info];

    // Initialize OSDP Control Panel
    let mut cp = ControlPanel::new(pd_infos).expect("Failed to initialize Control Panel");
    println!("OSDP Control Panel Initialized");

    // Initialize a channel for processing events
    let (event_tx, event_rx) = channel::<OsdpEvent>();

    // Setup Event Handler
    cp.set_event_callback(move |_pd, event| {
        // Send Event to Event Handler
        event_tx.send(event).expect("Failed to send event");

        // Report Back Successful Event Handling
        return 0;
    });
    println!("OSDP Event Handler Initialized");

    // Initialize Loop Timer
    let mut next_refresh = Instant::now() + Duration::from_millis(50);

    // Initialize Lock Timer
    let mut lock_timer = Instant::now();

    // Initialize Pin
    unlock_pin.set_low().unwrap();

    // Report Ready
    println!("Guardian Ready!");

    // Loop and wait for events
    loop {
        // Refresh Control Panel state
        cp.refresh();

        // Check for events
        while let Ok(event) = event_rx.try_recv() {
            // Process Event
            match event {
                libosdp::OsdpEvent::CardRead(card_read_event) => {
                    let card_data = card_read_event.data;
                    if card_data != allowed_card_id {
                        println!("Access Denied!");
                    } else {
                        println!("Access Granted!");
                        unlock_pin.set_high().unwrap();
                        lock_timer = Instant::now() + Duration::from_secs(5);
                        osdp_utils::send_access_granted_beep(&mut cp).expect("Failed to send beep");
                    }
                }
                _ => {
                    println!("Event: {:?}", event);
                }
            }
        }

        // Check if lock should be released
        if lock_timer < Instant::now() {
            unlock_pin.set_low().unwrap();
        }

        // Sleep for ~50ms
        thread::sleep(next_refresh.saturating_duration_since(Instant::now()));

        // Update next refresh time
        next_refresh += Duration::from_millis(50);
    }
}
