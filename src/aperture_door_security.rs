use std::time::Instant;

use esp_idf_svc::hal::gpio::{Output, Pin, PinDriver};

use super::manage_command::{MANAGECommand, MANAGECommandType};

pub enum DoorSecurityDoorType {
    Motorized,
    LockFailSecure,
}

pub struct DoorSecurity<'d, OP, CP, SP, UP>
where
    OP: Pin,
    CP: Pin,
    SP: Pin,
    UP: Pin,
{
    door_type: DoorSecurityDoorType,
    door_open_pin: PinDriver<'d, OP, Output>,
    door_close_pin: PinDriver<'d, CP, Output>,
    door_stop_pin: PinDriver<'d, SP, Output>,
    door_unlock_pin: PinDriver<'d, UP, Output>,
    last_action_time: Instant,
    lock_timer: Instant,
}

impl<'d, OP, CP, SP, UP> DoorSecurity<'d, OP, CP, SP, UP>
where
    OP: Pin,
    CP: Pin,
    SP: Pin,
    UP: Pin,
{
    pub fn new(
        door_type: DoorSecurityDoorType,
        door_open_pin: PinDriver<'d, OP, Output>,
        door_close_pin: PinDriver<'d, CP, Output>,
        door_stop_pin: PinDriver<'d, SP, Output>,
        door_unlock_pin: PinDriver<'d, UP, Output>,
    ) -> Self {
        Self {
            door_type,
            door_open_pin,
            door_close_pin,
            door_stop_pin,
            door_unlock_pin,
            last_action_time: Instant::now(),
            lock_timer: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        match self.door_type {
            DoorSecurityDoorType::Motorized => {
                self.tick_motorized();
            }
            DoorSecurityDoorType::LockFailSecure => {
                self.tick_lock_fail_secure();
            }
        }
    }

    fn tick_motorized(&mut self) {
        // Check if the last action was more than 500ms ago
        if self.last_action_time.elapsed().as_millis() > 500 {
            // Ensure all pins are low
            self.door_open_pin.set_low().unwrap();
            self.door_close_pin.set_low().unwrap();
            self.door_stop_pin.set_low().unwrap();
        }
    }

    fn tick_lock_fail_secure(&mut self) {
        // Check if the lock should be released
    }

    pub fn handle_command(&mut self, command: MANAGECommand) {
        match command.command {
            MANAGECommandType::DoorOpen => {
                log::info!("DOOR ACTION - Opening the door!");

                // Update the last action time
                self.last_action_time = Instant::now();

                // Ensure all other pins are low
                self.door_close_pin.set_low().unwrap();
                self.door_stop_pin.set_low().unwrap();

                // Set the door open pin high
                self.door_open_pin.set_high().unwrap();
            }
            MANAGECommandType::DoorClose => {
                log::info!("DOOR ACTION - Closing the door!");

                // Update the last action time
                self.last_action_time = Instant::now();

                // Ensure all other pins are low
                self.door_open_pin.set_low().unwrap();
                self.door_stop_pin.set_low().unwrap();

                // Set the door close pin high
                self.door_close_pin.set_high().unwrap();
            }
            MANAGECommandType::DoorStop => {
                log::info!("DOOR ACTION - ***STOPPING*** the door!");

                // Update the last action time
                self.last_action_time = Instant::now();

                // Ensure all other pins are low
                self.door_open_pin.set_low().unwrap();
                self.door_close_pin.set_low().unwrap();

                // Set the door stop pin high
                self.door_stop_pin.set_high().unwrap();
            }
        }
    }
}
