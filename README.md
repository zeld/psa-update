# psa-update

CLI alternative to PSA (Peugeot / Citroën / Opel) NAC / RCC firmware update, hopefully more robust.

Features:
- Download of RCC / NAC firmware updates, and GPS navigation map updates (NAC only)
- Resume of download in case of failure
- Preparation of USB storage device for car system update
- Lightweight self-contained executable that can run on multiple platforms: Windows, Linux, MacOS

# Usage

The command line executable can be invoked in a terminal using vehicle VIN as a parameter:
```
$ psa-update <VIN>
```
This will check for available RCC or NAC updates, and interactively ask for download and extraction of the firmware update to a USB device.

To check for updates of both firmware and GPS navigation map (NAC only):
```
$ psa-update <VIN> --map eur
```

The list of available maps identifiers can be obtained using the help:
```
$ psa-update --help
```

# Setup

Binaries are available for Windows (x86-64), Linux (x86-64) and MacOS (x86-64) in the release section: simply download and extract the `psa-update`executable.
For other platforms the project can be built from source code (see below).

# Building

To build and run from source code using stable rust compiler toolchain (version 1.54+):
```
$ git clone https://github.com/zeld/psa-update.git
$ cargo run
```

# Proxy

Download is possible behind a proxy provided the `http_proxy` and `https_proxy` environment variables are correctly configured.
In a Linux or MacOS shell:
```
export http_proxy=<host>:<port>
export https_proxy=<host>:<port>
```
In a Windows CMD prompt:
```
SET http_proxy=<host>:<port>
SET https_proxy=<host>:<port>
```

# Credits

- Inspired from the Linux script in this french [forum post](https://www.forum-peugeot.com/Forum/threads/app-peugeot-update-logiciel-alternatif-multi-os-v1-5-26-08-2021.119707/)
- For the list of navigation maps, and associated content, this french [forum post](https://forum-auto.caradisiac.com/topic/129967-le-nac-du-3008-ii-et-de-tous-les-v%C3%A9hicules-psa-lisez-en-premier-la-page-n%C2%B012/)
- Mirror of firmware and map updates on [this site](https://sites.google.com/view/nac-rcc/)
