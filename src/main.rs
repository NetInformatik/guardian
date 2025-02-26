#![feature(mpmc_channel)]
#![feature(deadline_api)]

use std::sync::atomic::Ordering;
use std::sync::mpmc::sync_channel;
use std::sync::mpsc::{self, channel};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{mem, thread};

use aperture_door_security::DoorSecurityDoorType;
use aperture_ws_client::nuke_ws_client;
use atomic_time::AtomicInstant;
use esp_idf_svc::eth::EthDriver;
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::gpio::{Gpio0, Gpio1, Gpio16, Gpio17, InputPin, OutputPin, PinDriver};
use esp_idf_svc::hal::uart::{config, UartDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::ESP_ERR_TIMEOUT;
use esp_idf_svc::ws::FrameType;
use guardian_global_status::PD_ONLINE;
use libosdp::{ControlPanel, OsdpEvent, PdInfoBuilder};
use manage_command::{MANAGECommand, MANAGEReport};
use osdp_serial_channel::SerialChannel;

mod aperture_core;
mod aperture_door_security;
mod aperture_ws_client;
mod esp_hw;
mod guardian_global_status;
mod manage_command;
mod osdp_serial_channel;
mod osdp_time_patch;

#[macro_use]
extern crate lazy_static;

// Core Parameters
const WS_BASE_URI: &str = "wss://manage.netinformatik.com/ws/office-security/door-commands/";
const WS_TIMEOUT: Duration = Duration::from_secs(10);
const SYSTEM_HEALTH_LOOP_INTERVAL: Duration = Duration::from_secs(5);
const DOOR_SECURITY_LOOP_INTERVAL: Duration = Duration::from_millis(100);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Log all OSDP messages
    esp_idf_svc::log::set_target_level("osdp::cp", log::LevelFilter::Trace).unwrap();
    esp_idf_svc::log::set_target_level("libosdp::cp", log::LevelFilter::Trace).unwrap();

    // Report Start
    log::info!("Initializing Guardian...");

    // Fetch the peripherals, event loop, and NVS partition
    let (peripherals, sys_loop, _nvs) = aperture_core::system_setup();

    // Initialize Ethernet Driver
    let eth_driver = EthDriver::new_rmii(
        peripherals.mac,
        peripherals.pins.gpio25,
        peripherals.pins.gpio26,
        peripherals.pins.gpio27,
        peripherals.pins.gpio23,
        peripherals.pins.gpio22,
        peripherals.pins.gpio21,
        peripherals.pins.gpio19,
        peripherals.pins.gpio18,
        esp_idf_svc::eth::RmiiClockConfig::<Gpio0, Gpio16, Gpio17>::OutputInvertedGpio17(
            peripherals.pins.gpio17,
        ),
        Some(peripherals.pins.gpio5),
        // Replace with IP101 if you have that variant, or with some of the others in the `RmiiEthChipset`` enum
        esp_idf_svc::eth::RmiiEthChipset::LAN87XX,
        Some(0),
        sys_loop.clone(),
    )
    .unwrap();
    let eth = esp_idf_svc::eth::EspEth::wrap(eth_driver).unwrap();
    let mut eth = esp_idf_svc::eth::BlockingEth::wrap(eth, sys_loop.clone()).unwrap();

    // Start Ethernet
    eth.start().unwrap();
    log::info!("Ethernet Driver Started");

    // Setup channel for command data
    let (command_channel_tx, command_channel_rx) = mpsc::channel::<MANAGECommand>();

    // Setup channel for report data
    let (report_channel_tx, report_channel_rx) = mpsc::channel::<MANAGEReport>();

    // Initialize UART for OSDP
    let osdp_uart_tx_pin = peripherals.pins.gpio33.downgrade_output();
    let osdp_uart_rx_pin = peripherals.pins.gpio34.downgrade_input();
    let osdp_uart_config = config::Config::new().baudrate(Hertz(9_600));
    let osdp_uart = UartDriver::new(
        peripherals.uart1,
        osdp_uart_tx_pin,
        osdp_uart_rx_pin,
        Option::<Gpio0>::None,
        Option::<Gpio1>::None,
        &osdp_uart_config,
    )
    .unwrap();
    log::info!("OSDP UART Initialized");

    // Setup MAX485 REDE Pin
    let osdp_max485_rede_output = peripherals.pins.gpio14.downgrade_output();
    let mut osdp_max485_rede = PinDriver::output(osdp_max485_rede_output).unwrap();
    log::info!("OSDP MAX485 REDE Pin Initialized");

    // Initialize Open, Close, Stop/Unlock Pins
    let stop_unlock_pin_output = peripherals.pins.gpio13.downgrade_output();
    let stop_unlock_pin = PinDriver::output(stop_unlock_pin_output).unwrap();
    let open_pin_output = peripherals.pins.gpio32.downgrade_output();
    let open_pin = PinDriver::output(open_pin_output).unwrap();
    let close_pin_output = peripherals.pins.gpio4.downgrade_output();
    let close_pin = PinDriver::output(close_pin_output).unwrap();

    // Initialize the door security handler
    let mut door_security = aperture_door_security::DoorSecurity::new(
        DoorSecurityDoorType::LockFailSecure,
        open_pin,
        close_pin,
        stop_unlock_pin,
    );
    log::info!("Door Security Pin Handler System Initialized");

    // Initialize OSDP Serial TX & RX MPMC Queues
    let (osdp_serial_tx_sender, osdp_serial_tx_receiver) = sync_channel::<u8>(256);
    let (osdp_serial_rx_sender, osdp_serial_rx_receiver) = sync_channel::<u8>(256);
    log::info!("OSDP Serial MPMC Queues Initialized");

    // Setup Serial Port
    let serial_channel = Box::new(SerialChannel::new(
        1,
        osdp_serial_tx_sender,
        osdp_serial_rx_receiver,
    ));
    log::info!("OSDP Serial Channel Initialized");

    // Split the UART into TX and RX
    let (mut osdp_uart_tx, osdp_uart_rx) = osdp_uart.into_split();

    // Create thread to handle serial communication
    thread::spawn(move || {
        loop {
            // Read from UART
            let mut read_buf = [0u8; 1];
            match osdp_uart_rx.read(&mut read_buf, delay::BLOCK) {
                Ok(_) => {
                    // Get the single byte read
                    let byte = read_buf[0];

                    // Write to OSDP Serial RX Queue Producer
                    match osdp_serial_rx_sender.try_send(byte) {
                        Ok(_) => {}
                        Err(error) => match error {
                            std::sync::mpmc::TrySendError::Full(_) => {
                                log::warn!("WARNING: OSDP Serial RX Queue Full!");
                            }
                            std::sync::mpmc::TrySendError::Disconnected(_) => {
                                log::error!("ERROR: OSDP Serial RX Queue Disconnected!");
                            }
                        },
                    }
                }
                // If the EspError is not a timeout, log it
                Err(error) => {
                    if error.code() != ESP_ERR_TIMEOUT {
                        log::info!("Error Reading ESP UART Serial: {:?}", error);
                    }
                }
            }
        }
    });

    // Create thread to handle serial communication
    thread::spawn(move || {
        loop {
            // Read from OSDP Serial TX Pipe
            match osdp_serial_tx_receiver.recv() {
                Ok(byte) => {
                    // Set MAX485 REDE Pin to High
                    osdp_max485_rede.set_high().unwrap();

                    // Write first byte to UART
                    osdp_uart_tx.write(&[byte]).unwrap();

                    // For each byte in the queue, write it to the UART
                    for byte in osdp_serial_tx_receiver.try_iter() {
                        osdp_uart_tx.write(&[byte]).unwrap();
                    }

                    // Wait for UART to finish transmitting
                    osdp_uart_tx.wait_done(delay::BLOCK).unwrap();

                    // Set MAX485 REDE Pin to Low
                    osdp_max485_rede.set_low().unwrap();
                    // Write to UART
                }
                Err(error) => {
                    log::info!("OSDP Serial TX Queue Disconnected: {:?}", error);
                }
            }
        }
    });

    log::info!("OSDP Serial Thread Initialized");

    // Prepare Peripheral Device(s) Info
    let mut initial_pd_builder = PdInfoBuilder::new();
    initial_pd_builder = initial_pd_builder.channel(serial_channel);
    let pd_info = initial_pd_builder.build();
    let pd_infos = vec![pd_info];

    // Initialize OSDP Control Panel
    let mut cp = ControlPanel::new(pd_infos).expect("Failed to initialize Control Panel");
    log::info!("OSDP Control Panel Initialized");

    // Initialize a channel for processing events
    let (event_tx, event_rx) = channel::<OsdpEvent>();

    // Setup Event Handler
    cp.set_event_callback(move |_pd, event| {
        // Send Event to Event Handler
        event_tx.send(event).expect("Failed to send event");

        // Report Back Successful Event Handling
        return 0;
    });
    log::info!("OSDP Event Handler Initialized");

    // Initialize Loop Timer
    let mut next_refresh = Instant::now() + Duration::from_millis(50);

    // Report Ready
    log::info!("Guardian Local System Initialization Complete!");

    // Create thread to handle OSDP CP events & other tasks
    let osdp_event_report_channel_tx = report_channel_tx.clone();
    thread::spawn(move || {
        // Loop and wait for events
        loop {
            // Refresh Control Panel state
            cp.refresh();

            // Check for events
            while let Ok(event) = event_rx.try_recv() {
                // Process Event
                match event {
                    libosdp::OsdpEvent::CardRead(card_read_event) => {
                        log::info!("Card Read: {:?}", card_read_event);
                        let report = MANAGEReport::OsdpCardRead {
                            event: card_read_event,
                        };
                        osdp_event_report_channel_tx.send(report).unwrap();
                    }
                    _ => {
                        log::info!("Event: {:?}", event);
                    }
                }
            }

            // Print Info
            PD_ONLINE.store(cp.is_online(0), Ordering::SeqCst);

            // Sleep for ~50ms
            thread::sleep(next_refresh.saturating_duration_since(Instant::now()));

            // Update next refresh time
            next_refresh += Duration::from_millis(50);
        }
    });

    // Initialize the door security last tick time
    let door_security_last_tick = Arc::new(AtomicInstant::now());
    let last_tick = Arc::clone(&door_security_last_tick);

    // Create thread to handle door system
    thread::spawn(move || {
        loop {
            match command_channel_rx.try_recv() {
                Ok(command) => {
                    // We received a command, handle it
                    door_security.handle_command(command);
                }
                Err(_) => {
                    // Tick the door security system and sleep for a while
                    door_security.tick();
                    door_security_last_tick.store(Instant::now(), Ordering::SeqCst);
                    thread::sleep(DOOR_SECURITY_LOOP_INTERVAL);
                }
            }
        }
    });

    // Initialize Heartbeat
    let mut next_heartbeat = Instant::now();

    // Create thread to handle system health
    thread::spawn(move || {
        loop {
            // Sleep for a while
            thread::sleep(SYSTEM_HEALTH_LOOP_INTERVAL);

            // Retrieve elapsed time
            let elapsed = last_tick.load(Ordering::SeqCst).elapsed();

            // Display System Status
            let status_ip_info: String;
            match eth.eth().netif().get_ip_info() {
                Ok(ip_info) => {
                    status_ip_info = format!("IP: {}\n", ip_info.ip);
                }
                Err(error) => {
                    status_ip_info = format!("IP: Not Available! ({:?})\n", error);
                }
            }
            let status = format!(
                "GUARDIAN SYSTEM STATUS\n---\nOSDP Online: {}\n{}Last Door Tick: {} seconds\n---",
                PD_ONLINE.load(Ordering::SeqCst),
                status_ip_info,
                elapsed.as_secs(),
            );
            log::info!("{}", status);

            // Prepare Heartbeat
            let now = Instant::now();
            if next_heartbeat < now {
                next_heartbeat = now + HEARTBEAT_INTERVAL;
                let is_healthy = elapsed.as_secs() < 2 && PD_ONLINE.load(Ordering::SeqCst);
                let heartbeat = MANAGEReport::Heartbeat {
                    is_healthy: is_healthy,
                };
                report_channel_tx.send(heartbeat).unwrap();
            }
        }
    });

    // Connect to MANAGE Door Security Websocket
    let mut ws_client =
        aperture_ws_client::ws_client_setup(WS_BASE_URI, WS_TIMEOUT, command_channel_tx.clone());

    // Handle WebSocket Connection
    loop {
        // Check if the WebSocket is closed
        if !aperture_ws_client::WS_OPEN.load(std::sync::atomic::Ordering::SeqCst) {
            log::warn!("WebSocket is closed! Reconnecting...");

            // Nuke the old WebSocket client (calls unsafe destroy method)
            nuke_ws_client(&ws_client);

            // Forget the old WebSocket client
            mem::forget(ws_client);

            // Create a new WebSocket client
            ws_client = aperture_ws_client::ws_client_setup(
                WS_BASE_URI,
                WS_TIMEOUT,
                command_channel_tx.clone(),
            );
        }

        // Set next check time
        let next_check = Instant::now() + Duration::from_secs(5);

        // Send any reports
        while let Ok(report) = report_channel_rx.recv_deadline(next_check) {
            match ws_client.send(
                FrameType::Text(false),
                serde_json::to_string(&report).unwrap().as_bytes(),
            ) {
                Ok(_) => {
                    log::info!("Sent report to MANAGE!: {:?}", report);
                }
                Err(_) => {
                    log::error!("Failed to send report to MANAGE!: {:?}", report);
                }
            }
        }
    }
}
