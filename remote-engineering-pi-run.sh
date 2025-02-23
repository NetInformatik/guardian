#!/bin/bash
set -euo pipefail

# Check for the binary argument
if [ -z "$1" ]
  then
    echo "No binary argument supplied"
fi

# Copy the binary to the remote device
scp "$1" remote-engineering:/tmp/aperture2
#ssh remote-engineering "/home/pi/.cargo/bin/espflash flash --monitor /tmp/aperture2"
ssh remote-engineering  -t 'bash -l -c "espflash flash --monitor /tmp/aperture2"'