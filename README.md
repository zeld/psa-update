# psa-update
CLI alternative to PSA (Peugeot / Citroën / Opel) NAC / RCC firmware update, hopefully more robust

Features:
- Download of RCC / NAC firmware updates
- Resume of download in case of failure
- Preparation of USB device for car system update
- Multiplatform: Window, Linux, MacOS

Not yet implemented:
- Map update for NAC
- Parallel download
- Automatic retry on failure

# Usage

Command line can be invoked using vehicle VIN as a parameter:
```
$ psa-update <VIN>
```
This will check for available RCC / NAC updates, and interactively ask for download and extraction of the firmware update to a USB device.

# Installation

Binaries are available for Windows (x86-64), Linux (x86-64) and MacOS (x86-64) in the release section. For other platforms the project from source code (see below).

# Building

To build and run from source using stable rust compiler toolchain:
```
$ git clone https://github.com/zeld/psa-update.git
$ cargo run
```
