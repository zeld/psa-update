# psa-update

`psa-update` is a CLI alternative to the official Stellantis (Peugeot / Citroën / DS / Opel) update applications for car infotainment systems (NAC / RCC firmware and navigation maps), hopefully more robust.

![Screenshot](screenshot.png)

`psa-update` offers mostly the same features as the official firmware/map update applications provided by the vehicle manufacturer, except that it does not format the USB flash drive used to transfer the firmware/map update to the car.

Features:

- Download RCC / NAC firmware updates and navigation map updates (NAC only)
- Resume downloads in case of failure
- Prepare a USB flash drive for updating the car infotainment system
- Lightweight, self-contained executable for Windows, Linux, and macOS

> [!NOTE]
> Updates are exclusively downloaded from the official Stellantis and TomTom websites.

## Installation

Prebuilt binaries are available for Windows (x86-64), Linux (x86-64), and macOS (x86-64 and AArch64/ARM64) on the [releases](https://github.com/zeld/psa-update/releases) page.

To install, download and extract the release archive, then run the `psa-update` executable.

For other platforms, the project can be built from source code (see below).

## Usage

The command line executable can be invoked in a terminal:

```shell
$ psa-update
```

This will interactively ask for a VIN, check for available NAC/RCC/map updates, and extract updates onto a USB flash drive.

Once copied to the USB drive, the update can be applied to the car infotainment system by following the official Stellantis instructions that are similar for all brand.

For example, for Peugeot:

- [RCC instructions](https://web.archive.org/web/20220719220945/https://media-ct-ndp.peugeot.com/file/38/2/map-software-rcc-en.632382.pdf)
- [NAC instructions](https://web.archive.org/web/20230602131011/https://media-ct-ndp.peugeot.com/file/38/0/map-software-nac-en.632380.pdf)

### Advanced usage

The command line executable supports multiple options. Refer to the help message for more information.

```console
$ psa-update --help
CLI alternative to Peugeot/Citroën/Opel/DS update applications for car infotainment system (NAC/RCC firmware and navigation maps), hopefully more robust. Supports for resume of downloads.

Usage: psa-update [OPTIONS] [VIN]

Arguments:
  [VIN]  Vehicle Identification Number (VIN) to check for update

Options:
      --map <map>            Sets the map to check for update. Supported maps:
                              - afr: Africa
                              - alg: Algeria
                              - asia: Asia
                              - eur: Europe
                              - isr: Israel
                              - latam: Latin America
                              - latam-chile: Latin America Chile
                              - mea: Middle East
                              - oce: Oceania
                              - russia: Russia
                              - taiwan: Taiwan
      --silent               Sets silent (non-interactive) mode
      --download             Automatically proceed with download of updates. Previous downloads will be resumed.
      --extract <extract>    Location where to extract update files. Should be the root of an empty FAT32 USB drive.
      --sequential-download  Forces sequential download of updates. By default updates are downloaded concurrently.
  -h, --help                 Print help
  -V, --version              Print version
```

A silent (non-interactive) mode can be activated using the `--silent` flag. It allows to fully automate the download and extraction.

For example, to check for updates and automatically download and extract them onto a USB drive, you can use the following command:

```shell
$ psa-update --silent --download --extract /path/to/usb/drive
```

## Requirements

To transfer updates to the car, a USB flash drive is required:

- Recommended size is **32 GB**. Although most updates are smaller than 16 GB, some navigation maps can be larger than 16 GB.
- It must be formatted as **FAT32** and **empty**.

> [!NOTE]
> When using Windows, if the USB flash drive is larger than 32 GB, it is not possible to format it using FAT32. Alternatives are:
>
> - Create a 32 GB partition and format if as FAT32 and leave the rest unformatted.
> - Use a third-party tool to format the USB flash drive as FAT32. The official application presumably uses [fat32format from Ridgecrop Consultants Ltd](http://ridgecrop.co.uk/index.htm?guiformat.htm).

On Linux, OpenSSL is required. On Windows and MacOS, nothing is required, the operating system TLS framework is used.

## Build from source

To build and run from source code using the stable Rust toolchain (Rust 1.81 or newer):

```shell
$ git clone https://github.com/zeld/psa-update.git
$ cd psa-update
$ cargo build --release
$ ./target/release/psa-update --version
```

## Other tools

The table below quickly compares `psa-update` with other available tools I'm aware of.

Open source:

| Tool                                                                                        | Type        | Platform                | Language | Download updates | Format USB drive | Extract to USB drive |
| ------------------------------------------------------------------------------------------- | ----------- | ----------------------- | -------- | ---------------- | ---------------- | -------------------- |
| [psa-update](https://github.com/zeld/psa-update)                                            | Terminal    | Windows / Linux / MacOS | English  | ✅ (with resume) | ❌              | ✅                   |
| [peugeot-tools](https://github.com/sbz/peugeot-tools)                                       | Terminal    | ? (build from source)   | English  | ✅ (with resume) | ❌              | ❌                   |

Others:

| Tool                                                                                        | Type        | Platform                | Language | Download updates | Format USB drive | Extract to USB drive |
| ------------------------------------------------------------------------------------------- | ----------- | ----------------------- | -------- | ---------------- | ---------------- | -------------------- |
| Official (Peugeot Update, Citroën Update, Opel Update, DS Update)                           | Desktop app | Windows / MacOS         | Multi    | ✅ (with resume) | ✅              | ✅                   |
| [Peugeot Update alternative](https://github.com/bagou9/Peugeot-Update-logiciel-alternatif/) | Desktop app | Windows                 | French   | ✅ (with resume) | ❌              | ❌                   |
| [Mittns Toolbox](https://www.mittns.de/thread/428-mittns-toolbox-2-5-0-4-download)          | Desktop app | Windows                 | Multi    | ✅               | ✅              | ✅                   |

## Proxy

Downloading updates works behind a proxy as long as the `http_proxy` and `https_proxy` environment variables are correctly set.

Using a Linux or MacOS terminal:

```shell
export http_proxy=<host>:<port>
export https_proxy=<host>:<port>
```

Using a Windows CMD prompt:

```cmd
SET http_proxy=<host>:<port>
SET https_proxy=<host>:<port>
```

## Logging

Logging can be enabled using the `RUST_LOG` variable. For exemple to enable `debug` log level:

```shell
RUST_LOG="debug" ./psa-update
```

## Credits

- Inspired from the Linux script in this French [forum post](https://www.forum-peugeot.com/Forum/threads/app-peugeot-update-logiciel-alternatif-multi-os-v1-5-26-08-2021.119707/)
- List of firmware and map updates on [this site](https://sites.google.com/view/nac-rcc/)
