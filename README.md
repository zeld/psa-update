# psa-update

CLI alternative to PSA (Peugeot / Citroën / DS / Opel) infotainment system update (NAC / RCC firmware and navigation maps),
hopefully more robust.

![Screenshot](screenshot.png)

psa-update offers mostly the same features as the official firmware/map update application proposed by the car vendor,
except that it does not format the USB device that has to be used to upload the firmware/map update to the car. Updates
are exclusively downloaded from the official PSA site.

Features:
- Download of RCC / NAC firmware updates, and GPS navigation map updates (NAC only)
- Resume of download in case of failure
- Preparation of USB storage device for car system update
- Lightweight self-contained executable that can run on multiple platforms: Windows, Linux, MacOS

## Usage

The command line executable can be invoked in a terminal using vehicle VIN as a parameter:
```shell
$ psa-update <VIN>
```
This will check for available RCC or NAC updates, and interactively ask for download and extraction of the firmware update to a USB device.

To check for updates of both firmware and GPS navigation map (NAC only):
```shell
$ psa-update <VIN> --map eur
```

The list of available maps identifiers can be obtained using the help:
```shell
$ psa-update --help
```

Once copied to the USB drive, the update can be applied on the infotainement system following PSA instructions.
For example for Peugeot:
- [RCC instructions](https://media-ct-ndp.peugeot.com/file/38/2/map-software-rcc-en.632382.pdf)
- [NAC instructions](https://media-ct-ndp.peugeot.com/file/38/0/map-software-nac-en.632380.pdf).

## Requirements

On Linux, OpenSSL is required. On Windows and MacOS, nothing is required, the operating system TLS framework is used.

## Install

Binaries are available for Windows (x86-64), Linux (x86-64) and MacOS (x86-64) in the [releases](https://github.com/zeld/psa-update/releases) section.
To install, simply download and extract the `psa-update` executable.
For other platforms the project can be built from source code (see below).

## Build from source

To build and run from source code using stable rust compiler toolchain (version 1.54+):
```shell
$ git clone https://github.com/zeld/psa-update.git
$ cargo run
```

## Proxy

Download is possible behind a proxy provided the `http_proxy` and `https_proxy` environment variables are correctly set.
In a Linux or MacOS shell:
```shell
export http_proxy=<host>:<port>
export https_proxy=<host>:<port>
```
In a Windows CMD prompt:
```cmd
SET http_proxy=<host>:<port>
SET https_proxy=<host>:<port>
```

## Logging

Logging can be enabled using the `RUST_LOG` variable. For example to enable `debug` log level:
```shell
RUST_LOG="debug" ./psa-update
```

## Credits

- Inspired from the Linux script in this french [forum post](https://www.forum-peugeot.com/Forum/threads/app-peugeot-update-logiciel-alternatif-multi-os-v1-5-26-08-2021.119707/)
- For the list of navigation maps, and associated content, this french [forum post](https://forum-auto.caradisiac.com/topic/129967-le-nac-du-3008-ii-et-de-tous-les-v%C3%A9hicules-psa-lisez-en-premier-la-page-n%C2%B012/)
- List of firmware and map updates on [this site](https://sites.google.com/view/nac-rcc/)
