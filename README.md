# psa-update

CLI alternative to Stellantis (Peugeot / Citroën / DS / Opel) update applications for car infotainment system (NAC / RCC firmware and navigation maps), hopefully more robust.

![Screenshot](screenshot.png)

`psa-update` offers mostly the same features as the official firmware/map update application proposed by the car vendor, except that it does not format the USB flash drice that has to be used to upload the firmware/map update to the car.

Updates are exclusively downloaded from the official Stellantis website.

Features:

- Download updates of RCC / NAC firmwares, and navigation maps (NAC only)
- Resume downloads in case of failure
- Prepares USB flash drive for car infotainment system update
- Lightweight self-contained executable that can run on multiple platforms: Windows, Linux, MacOS

## Usage

The command line executable can be invoked in a terminal:

```shell
$ psa-update
```

This will interactively ask for vehicle VIN, check for available NAC/RCC/map updates, and extract updates to a USB flash drive.

Once copied to the USB drive, the update can be applied on the car infotainment system following stellantis instructions.

For exemple for Peugeot:

- [RCC instructions](https://web.archive.org/web/20220719220945/https://media-ct-ndp.peugeot.com/file/38/2/map-software-rcc-en.632382.pdf)
- [NAC instructions](https://web.archive.org/web/20230602131011/https://media-ct-ndp.peugeot.com/file/38/0/map-software-nac-en.632380.pdf)

## Requirements

For the transfer of updates to the car, a USB flash drive is required:

- Recommended size is **32 GB**. Although most updates are smaller than 16 GB, some navigations maps can be larger than 16 GB.
- It must be formatted as **FAT32** and **empty**.

> Note: When using Windows, if the USB flash drive is larger than 32 GB, it is not possible to format it using FAT32. Alternatives are:
>
> - Create a 32 GB partition and format if as FAT32 and leave the rest unformatted.
> - Use a third-party tool to format the USB flash drive using FAT32. The official vendor application presumably uses [fat32format from Ridgecrop Consultants Ltd](http://ridgecrop.co.uk/index.htm?guiformat.htm) to achieve this.

On Linux, OpenSSL is required. On Windows and MacOS, nothing is required, the operating system TLS framework is used.

## Install

Binaries are available for Windows (x86-64), Linux (x86-64) and MacOS (x86-64 and AArch64/ARM64) in the [releases](https://github.com/zeld/psa-update/releases) section.

To install, simply download and extract the `psa-update` executable.

For other platforms the project can be built from source code (see below).

## Build from source

To build and run from source code using stable rust compiler toolchain (version 1.81+):

```shell
$ git clone https://github.com/zeld/psa-update.git
$ cd psa-update
$ cargo build --release
$ ./target/release/psa-update --version
```

## Proxy

Download of updates is possible behind a proxy provided the `http_proxy` and `https_proxy` environment variables are correctly set.

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
- For the list of navigation maps, and associated content, this French [forum post](https://forum-auto.caradisiac.com/topic/129967-le-nac-du-3008-ii-et-de-tous-les-v%C3%A9hicules-psa-lisez-en-premier-la-page-n%C2%B012/)
- List of firmware and map updates on [this site](https://sites.google.com/view/nac-rcc/)
