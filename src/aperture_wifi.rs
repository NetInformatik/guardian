use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use heapless::String;

// WiFi Connect Failed Error (with message)
#[derive(Debug)]
pub struct WifiConnectFailedError;

pub fn wifi_setup<'a>(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    wifi_ssid: String<32>,
    wifi_password: String<64>,
) -> BlockingWifi<EspWifi<'a>> {
    // Configure the WiFi
    log::info!("Configuring WiFi...");

    // Initialize the WiFi driver
    let mut wifi_driver: EspWifi<'a> = EspWifi::new(modem, sys_loop.clone(), Some(nvs)).unwrap();

    // Configure the WiFi driver
    wifi_driver
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: wifi_ssid,
            password: wifi_password,
            ..Default::default()
        }))
        .unwrap();

    // Wrap in Blocking
    let mut blocking_wifi_driver = BlockingWifi::wrap(wifi_driver, sys_loop).unwrap();

    // Start the WiFi driver
    blocking_wifi_driver.start().unwrap();

    return blocking_wifi_driver;
}

pub fn wifi_connect(
    wifi_driver: &mut BlockingWifi<EspWifi>,
    wifi_poll_interval: Duration,
    wifi_timeout: Duration,
) -> Result<(), WifiConnectFailedError> {
    // Connect to the WiFi network
    log::info!("Connecting to WiFi...");
    match wifi_driver.connect() {
        Ok(_) => {
            log::info!("WiFi association established!");
        }
        Err(_) => {
            log::error!("Failed to establish WiFi association!");
            return Err(WifiConnectFailedError);
        }
    }

    // Set the WiFi timeout
    let start_time = Instant::now();

    // Poll the WiFi connection status
    loop {
        let is_connected = wifi_driver.is_connected().unwrap();
        let is_up = wifi_driver.is_up().unwrap();
        log::info!(
            "WiFi connection status: connected={}, up={}",
            is_connected,
            is_up
        );

        if is_connected && is_up {
            break;
        }

        if start_time.elapsed() > wifi_timeout {
            log::error!("WiFi connection timed out!");
            return Err(WifiConnectFailedError);
        }

        sleep(wifi_poll_interval);
    }

    log::info!(
        "Connected to WiFi network: {:?}",
        wifi_driver.wifi().sta_netif().get_ip_info().unwrap()
    );
    return Ok(());
}

pub fn wifi_check(
    wifi_driver: &mut BlockingWifi<EspWifi>,
    wifi_poll_interval: Duration,
    wifi_timeout: Duration,
) {
    // Check WiFi connection status
    let is_up = wifi_driver.is_up().unwrap();
    log::info!("WiFi Health Check: up={}", is_up);

    // If WiFi is down, try to reconnect
    if !is_up {
        let wifi_connect_result = wifi_connect(wifi_driver, wifi_poll_interval, wifi_timeout);
        match wifi_connect_result {
            Ok(_) => {
                log::info!("WiFi reconnected successfully!");
            }
            Err(_) => {
                log::error!("WiFi reconnection failed!");
            }
        }
    }
}
