use libosdp::{ControlPanel, OsdpCommand, OsdpCommandBuzzer};

pub fn send_access_granted_beep(cp: &mut ControlPanel) -> Result<(), libosdp::OsdpError> {
    // Send Beep Command
    let beep_cmd = OsdpCommandBuzzer{
        reader: 0,
        control_code: 2,
        on_count: 1,
        off_count: 1,
        rep_count: 2,
    };
    let cmd = OsdpCommand::Buzzer(beep_cmd);
    return cp.send_command(0, cmd);
}