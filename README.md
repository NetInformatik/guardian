# Guardian Door Controller
This an ESP32 PoE-based system for controlling the doors.

## Pins
### ESP32 <-> MAX485
- GPIO-33 <-> DI (Driver Input)
- GPIO-34 <-> RO (Receiver Output)
- GPIO-14 <-> RE (Receiver Enable - Active LOW) + DE (Driver Enable)
- 3V3 <-> VCC
- GND <-> GND
### ESP32 <-> Door Unlock Relay
- GPIO-13 <-> IN (Relay Input)
### MAX485 <-> Card Reader
- A <-> OSDP RS-485 A(-)
- B <-> OSDP RS-485 B((+)