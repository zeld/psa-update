# psa-update
CLI alternative to PSA (Peugeot / Citroën / Opel) NAC / RCC firmware update, hopefully more robust

Features:
- Download of RCC / NAC firmware updates
- Download of navigation map updates for NAC
- Resume of download in case of failure
- Preparation of USB device for car system update
- Multiplatform: Window, Linux, MacOS
- Concurrent download of firmware and map (using a single thread)

Not yet implemented:
- Concurrent download for a single file
- Automatic retry on failure

# Usage

Command line can be invoked in a terminal using vehicle VIN as a parameter:
```
$ psa-update <VIN>
```
This will check for available RCC / NAC updates, and interactively ask for download and extraction of the firmware update to a USB device.

To check for updates of both firmware and navigation map (NAC only):
```
$ psa-update <VIN> --map eur
```

The list of available maps identifiers can be obtained using the help:
```
$ psa-update --help
```

# Installation

Binaries are available for Windows (x86-64), Linux (x86-64) and MacOS (x86-64) in the release section. For other platforms the project can be built from source code (see below).

# Building

To build and run from source using stable rust compiler toolchain:
```
$ git clone https://github.com/zeld/psa-update.git
$ cargo run
```

# Proxy

Download is possible behind a proxy provided the `https_proxy` environment variable is correctly configured.
In a Linux or MacOS shell:
```
export https_proxy=https://<host>:<port>
```
In a Windows command line:
```
SET https_proxy=https://<host>:<port>
```