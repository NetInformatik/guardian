use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::gpio::{Gpio0, Gpio1, InputPin, OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::uart::{config, UartDriver};
use esp_idf_svc::hal::units::Hertz;
use libosdp::{ControlPanel, OsdpEvent, PdInfoBuilder};
use max485::Max485;
use osdp_serial_max485_driver::OsdpSerialMax485Driver;

mod osdp_serial_driver;
mod osdp_serial_max485_driver;
mod osdp_serial_channel;
mod osdp_utils;

// GPIO Pin for Lock
const GPIO_LOCK: u8 = 17;

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
        peripherals.uart0,
        osdp_uart_tx,
        osdp_uart_rx,
        Option::<Gpio0>::None,
        Option::<Gpio1>::None,
        &osdp_uart_config,
    ).unwrap();

    // Setup MAX485 Driver
    let osdp_max485_rede = peripherals.pins.gpio4.downgrade_output();
    let osdp_max485 = Max485::new(osdp_uart, osdp_max485_rede.into());

    // Initialize Unlock Pin
    let unlock_pin_output = peripherals.pins.gpio15.downgrade_output();
    let mut unlock_pin = PinDriver::output(unlock_pin_output).unwrap();

    // Prepare Settings
    let allowed_card_id = vec![192, 77, 43, 64];

    // Setup Serial Port
    let serial_max485_driver = OsdpSerialMax485Driver::new(osdp_max485);
    let serial_channel = Box::new(osdp_serial_channel::SerialChannel::new(0, serial_max485_driver));

    // Prepare Peripheral Device(s) Info
    let mut initial_pd_builder = PdInfoBuilder::new();
    initial_pd_builder = initial_pd_builder.channel(serial_channel);
    let pd_info = initial_pd_builder.build();
    let pd_infos = vec! [pd_info];

    // Initialize OSDP Control Panel
    let mut cp = ControlPanel::new(pd_infos).expect("Failed to initialize Control Panel"); 

    // Initialize a channel for processing events
    let (event_tx, event_rx) = channel::<OsdpEvent>();
    
    // Setup Event Handler
    cp.set_event_callback(move |_pd, event| {
        // Send Event to Event Handler
        event_tx.send(event).expect("Failed to send event");

        // Report Back Successful Event Handling
        return 0;
    });

    // Report Ready
    println!("Guardian Ready!");

    // Initialize Loop Timer
    let mut next_refresh = Instant::now() + Duration::from_millis(50);

    // Initialize Lock Timer
    let mut lock_timer = Instant::now();

    // Initialize Pin
    unlock_pin.set_low();

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
                        unlock_pin.set_high();
                        lock_timer = Instant::now() + Duration::from_secs(5);
                        osdp_utils::send_access_granted_beep(&mut cp).expect("Failed to send beep");
                    }
                },
                _ => {
                    println!("Event: {:?}", event);
                }
            }
        }

        // Check if lock should be released
        if lock_timer < Instant::now() {
            unlock_pin.set_low();
        }

        // Sleep for ~50ms
        thread::sleep(next_refresh.saturating_duration_since(Instant::now()));

        // Update next refresh time
        next_refresh += Duration::from_millis(50);
    }
}
