use esp_idf_svc::sys::{esp_efuse_mac_get_default, ESP_OK};

pub fn get_mac_address() -> Result<[u8; 6], &'static str> {
    let mut mac: [u8; 6] = [0; 6];
    let result = unsafe { esp_efuse_mac_get_default(mac.as_mut_ptr()) };

    if result == ESP_OK {
        Ok(mac)
    } else {
        Err("Failed to get MAC address")
    }
}
