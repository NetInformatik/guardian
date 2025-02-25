use esp_idf_svc::{
    eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, nvs::EspDefaultNvsPartition,
};

pub fn system_setup() -> (Peripherals, EspSystemEventLoop, EspDefaultNvsPartition) {
    // Fetch the peripherals, event loop, and NVS partition
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    // Return the peripherals, event loop, and NVS partition
    return (peripherals, sys_loop, nvs);
}
