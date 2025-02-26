use esp_idf_svc::hal::io::EspIOError;
use esp_idf_svc::handle::RawHandle;
use esp_idf_svc::ws::client::{
    EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
};
use hex::encode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::esp_hw::get_mac_address;
use super::manage_command::MANAGECommand;

// Shared flag to indicate connection status
pub static WS_OPEN: AtomicBool = AtomicBool::new(false);

pub fn ws_client_setup(
    ws_base_uri: &str,
    ws_timeout: Duration,
    tx: Sender<MANAGECommand>,
) -> EspWebSocketClient {
    // Combine the WebSocket base URI with the MAC address
    let mac_address = get_mac_address().unwrap();
    let ws_uri = format!("{}{}/", ws_base_uri, encode(mac_address));

    // Configure the WebSocket client
    let config = EspWebSocketClientConfig {
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    // Create the WebSocket client
    let ws_client = EspWebSocketClient::new(&ws_uri, &config, ws_timeout, move |event| {
        on_websocket_event(&tx, event)
    });

    // Assume WS is open until something goes wrong
    WS_OPEN.store(true, Ordering::SeqCst);
    ws_client.unwrap()
}

fn on_websocket_event(tx: &Sender<MANAGECommand>, event: &Result<WebSocketEvent, EspIOError>) {
    match event {
        Ok(event) => {
            match event.event_type {
                WebSocketEventType::Connected => {
                    log::info!("Connected to MANAGE!");
                }
                WebSocketEventType::Disconnected => {
                    log::warn!("Disconnected from MANAGE!");
                }
                WebSocketEventType::Text(data) => {
                    log::debug!("WebSocket event: Text: {:?}", data);
                    match serde_json::from_str(data) {
                        Ok(command) => {
                            tx.send(command).unwrap();
                        }
                        Err(_) => {
                            log::error!("Failed to decode incoming command!");
                        }
                    }
                }
                WebSocketEventType::Closed => {
                    log::warn!("Connection to MANAGE closed! Marking for retry...");
                    WS_OPEN.store(false, Ordering::SeqCst);
                }
                // Any other event type
                _ => {
                    log::debug!("WebSocket event: Event: {:?}", event.event_type);
                }
            }
        }
        Err(err) => {
            log::error!("Error: {:?}", err);
        }
    }
}

pub fn nuke_ws_client(ws_client: &EspWebSocketClient) {
    // Retrieve the WebSocket client handle
    let ws_client_handle = ws_client.handle();

    // Destroy the WebSocket client
    unsafe {
        esp_idf_svc::sys::esp_websocket_client_destroy(ws_client_handle);
    }
}
