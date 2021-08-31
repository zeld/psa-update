# psa-update
CLI alternative to PSA (Peugeot / Citroën / Opel) NAC / RCC firmware update, hopefully more robust

Features:
- Download of RCC / NAC firmware updates
- Resume of download in case of failure
- Preparation of USB device for car system update

Not yet implemented:
- Map update for NAC
- Parallel download

# Usage

Command line can be invoked using vehicle VIN as a parameter:
```
psa-update <VIN>
```
This will check for available RCC / NAC updates, and interactively ask for download and extraction of the firmware update to a USB device.

# Binaries

Binaries are available for Windows in the release section.

# Building from source

To build and run from source using stable rust compiler toolchain:
```
git clone https://github.com/zeld/psa-update.git
cargo run
```
