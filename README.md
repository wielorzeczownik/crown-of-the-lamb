<h1 align="center">Crown of the Lamb</h1>

<p align="center">
  <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/actions/workflows/release.yml"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&labelColor=2d333b&color=3fb950"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&color=2ea043"/><img src="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&labelColor=2d333b&color=3fb950" alt="Release"/></picture></a> <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&labelColor=2d333b&color=3fb950"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&color=2ea043"/><img src="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&labelColor=2d333b&color=3fb950" alt="Latest Release"/></picture></a> <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/blob/main/LICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/License-MIT-3fb950?style=flat-square&labelColor=2d333b"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/License-MIT-2ea043?style=flat-square"/><img src="https://img.shields.io/badge/License-MIT-3fb950?style=flat-square&labelColor=2d333b" alt="License: MIT"/></picture></a>
  <br/>
  <img src="https://img.shields.io/badge/Rust-B7410E?style=flat-square&logo=rust&logoColor=white" alt="Rust"/> <img src="https://img.shields.io/badge/ESP32-000000?style=flat-square&logo=espressif&logoColor=white" alt="ESP32"/> <img src="https://img.shields.io/badge/no__std-embassy-6f42c1?style=flat-square" alt="no_std / embassy"/>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/logo.png" alt="Crown of the Lamb" width="300" />
</p>

<p align="center">🇬🇧 English | 🇵🇱 <a href="README.pl.md">Polski</a></p>

Open-source **ESP32 firmware** written in **Rust** (`no_std`, [Embassy](https://embassy.dev)) for an animatronic **Cult of the Lamb** crown prop. A round **GC9A01** LCD renders a living, blinking eye that looks around, reacts to the direction of sound, and slips into darker expressions – all tunable wirelessly from your phone through the prop's own **WiFi captive portal**.

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/demo.jpg" alt="The finished crown with the eye lit up" width="70%" />
</p>

## Features

- **Animated eye** on a GC9A01 240×240 round display: idle look-around, natural blinking, pupil wiggle, and a specular highlight.
- **Sound-reactive expressions**: two microphones feed an FFT that estimates which direction a sound came from; the eye tracks toward it and can react with **Startled**, **Suspicious**, **Angry**, or an **EyeRoll**.
- **The Sin** resting expression that surfaces at random during quiet moments.
- **WiFi access point + captive portal**: connect to the prop's network and a configuration UI opens automatically (DHCP + DNS + HTTP served on-device).
- **Live configuration**: eye colour, pupil shape, blink cadence, sound threshold, and per-expression probabilities – applied instantly and persisted to flash.

## Hardware

| Component   | Details                                 |
| ----------- | --------------------------------------- |
| MCU         | ESP32 (e.g. ESP32-WROOM-32)             |
| Display     | GC9A01 240×240 round LCD (SPI)          |
| Microphones | 2× analog modules (e.g. MAX9814), L + R |
| Power       | 5V USB or battery, per your enclosure   |

### Wiring

Two groups: **signal** (display + microphones to the ESP32) and **power** (battery → charger → boost → ESP32).

#### Signal – display & microphones → ESP32

| Module        | Pin        | ESP32                         |
| ------------- | ---------- | ----------------------------- |
| GC9A01        | VCC        | 3V3                           |
| GC9A01        | GND        | GND                           |
| GC9A01        | SCL / SCK  | GPIO18 (SPI CLK)              |
| GC9A01        | SDA / MOSI | GPIO23 (SPI MOSI)             |
| GC9A01        | RES        | GPIO4                         |
| GC9A01        | DC         | GPIO2                         |
| GC9A01        | CS         | GPIO5                         |
| GC9A01        | BL         | GPIO15 (backlight)            |
| Left MAX9814  | VDD        | 3V3                           |
| Left MAX9814  | GND        | GND                           |
| Left MAX9814  | OUT        | GPIO34 (ADC1_CH6, input-only) |
| Left MAX9814  | GAIN       | GND (60 dB)                   |
| Right MAX9814 | VDD        | 3V3                           |
| Right MAX9814 | GND        | GND                           |
| Right MAX9814 | OUT        | GPIO35 (ADC1_CH7, input-only) |
| Right MAX9814 | GAIN       | GND (60 dB)                   |

Mount the microphones on opposite sides of the crown so the left/right level difference gives usable sound direction. On the ESP32 side the ADC inputs use 11 dB attenuation (set in firmware).

#### Power – 18650 → TP4056 → switch → MT3608 → ESP32

| From        | To                       | Note                               |
| ----------- | ------------------------ | ---------------------------------- |
| 18650 + / − | TP4056 B+ / B−           | cell into the protected charger    |
| TP4056 OUT+ | switch → MT3608 IN+      | the switch cuts the whole device   |
| TP4056 OUT− | MT3608 IN− / GND         | common ground                      |
| MT3608 OUT+ | ESP32 5V (VIN)           | **set MT3608 to 5.0 V first!**     |
| ESP32 3V3   | GC9A01 VCC + MAX9814 VDD | from the ESP32's onboard 3.3 V LDO |

- Use a TP4056 module **with protection** (6 pads: IN±, B±, OUT±).
- With USB-C connected the TP4056 charges the cell **and** runs the device at the same time; unplugged, the switch fully cuts battery draw.

> [!TIP]
> For a rock-steady image, add near the power pins: ~10 µF + 100 nF
> across the display VCC/GND and the ESP32 3V3, plus a larger 220-470 µF on the
> MT3608 output.

## Bill of materials

### 3D-printed enclosure

The crown is based on [Red Crown (Cult of the Lamb)](https://www.printables.com/model/326078-red-crown-cult-of-the-lamb/files) by [ShinpiMakes](https://www.printables.com/@ShinpiMakes_277232) on Printables – the **Full Shelled Cross Mount** variant – then modified to fit the electronics:

- an opening cut out for the **eye** (display window),
- **microphone holes** on both sides (for left/right sound direction),
- cut-outs for **2× USB-C ports** (charger + boost module) and a **power switch**.

It's printed at **120% scale** so all the electronics fit inside. Scaling can come out differently depending on your printer and slicer, so use the physical size as a reference: the finished print should measure **around 11.5 cm** in height, from its lowest to its highest point.

### Parts

| Part                                                | Role                                                                        | Source                                                                                                                      |
| --------------------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| ESP32 DevKitC-32D                                   | main MCU                                                                    | [AliExpress](https://pl.aliexpress.com/item/1005007963451549.html)                                                          |
| GC9A01 240×240 round display                        | the animated eye                                                            | [AliExpress](https://pl.aliexpress.com/item/1005004482028005.html)                                                          |
| 2× MAX9814 microphone module                        | stereo sound-direction sensing                                              | [AliExpress](https://pl.aliexpress.com/item/1005006109706636.html)                                                          |
| USB-C charging module                               | Li-ion charging                                                             | [AliExpress](https://pl.aliexpress.com/item/1005009438702687.html)                                                          |
| MT3608 boost converter (USB-C)                      | steps the battery up to 5 V                                                 | [AliExpress](https://pl.aliexpress.com/item/1005011548818999.html)                                                          |
| 18650 cell                                          | battery                                                                     | [AliExpress](https://pl.aliexpress.com/item/1005011665515612.html)                                                          |
| 18650 holder                                        | battery mount                                                               | [AliExpress](https://pl.aliexpress.com/item/1005005084346241.html)                                                          |
| Power switch                                        | cuts the battery circuit                                                    | [AliExpress](https://pl.aliexpress.com/item/4000681145062.html)                                                             |
| Jumper / Dupont wires                               | wiring                                                                      | [AliExpress](https://pl.aliexpress.com/item/1005006205169394.html)                                                          |
| 2× USB-C extension cable (short, e.g. UGREEN 0.5 m) | routes the ESP32 and charging ports out to the shell – any short cable fits | [UGREEN](https://www.ugreen.pl/pl/products/kable/usb-c/przedluzajacy-kabel-ugreen-usb-c-0-5m-3-1-4k-60w-czarny-ed008-60664) |
| Small heatsink set                                  | cools the ESP32 and spaces the warm chip off the printed plastic            | [AliExpress](https://pl.aliexpress.com/item/4000266052801.html)                                                             |
| Standoff / spacer set                               | mounts the display at the correct depth                                     | [AliExpress](https://pl.aliexpress.com/item/1005006270641755.html)                                                          |

> [!NOTE]
> Assembled for roughly **150 PLN (~€35)** in parts at the time – component
> prices have since risen. AliExpress listings are volatile; if a link dies,
> search by the part name, any equivalent module works.

## Assembly

It's a tight fit. A few notes from putting mine together:

- The crown was printed in one piece and its **bottom cut open and carved** afterwards to get the electronics in. If you have a printer, it's easier to **split the bottom in the slicer** so you can drop the larger parts in cleanly.
- **Pack everything into the corners** of the crown: seat the **battery and ESP32 first**, then feed in the **USB-C extension cables and the display** last.

<div align="center">
<table>
<tr>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-1.jpg" alt="Wiring coiled inside the open base" width="200"/><br/><sub><b>Base wiring</b> · cables & connectors packed in</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-2.jpg" alt="Sealed back of the crown with mic port" width="200"/><br/><sub><b>Rear</b> · sealed shell, mic port</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-3.jpg" alt="Twin USB-C ports and power button at the base" width="200"/><br/><sub><b>Ports</b> · dual USB-C & power button</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-4.jpg" alt="Front of the crown with the eye cutout" width="200"/><br/><sub><b>Front</b> · eye-socket cutout</sub></td>
</tr>
</table>
</div>

## Flashing

Grab the latest prebuilt firmware from the [GitHub Releases](https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest):

- **[crown-of-the-lamb.bin](https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest/download/crown-of-the-lamb.bin)** – full image (bootloader + partition table + app)

Flash it at `0x0`:

```bash
espflash write-bin 0x0 crown-of-the-lamb.bin
# or with esptool:
esptool.py --chip esp32 write_flash 0x0 crown-of-the-lamb.bin
```

## Configuration

On boot the prop starts a WiFi access point. Join it with a phone or laptop and the captive portal opens automatically (or browse to the gateway IP `192.168.4.1`).

From the portal you can adjust, live:

- **Eye colour** (RGB) and **pupil** height/shape
- **Blink interval** and **wiggle amplitude**
- **Sound threshold** for reaction sensitivity
- **Expression chances** (`Sin`, `EyeRoll`, `Startled`, `Suspicious`, `Angry`)

Reaction expressions roll independently on each detected change of sound direction; the **Sin** chance rolls on each blink. All values are saved to flash and reload on the next power-up.

<div align="center">
<table>
<tr>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-1.jpg" alt="Eye tab with colour, pupil and blink controls" width="200"/><br/><sub><b>Eye</b> · colour, pupil & blink</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-2.jpg" alt="Mimicry tab with expression chance sliders" width="200"/><br/><sub><b>Mimicry</b> · expression chances</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-3.jpg" alt="Sound tab with microphone sensitivity threshold" width="200"/><br/><sub><b>Sound</b> · mic sensitivity</sub></td>
<td align="center" width="25%"><img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-4.jpg" alt="WiFi tab with SSID configuration" width="200"/><br/><sub><b>WiFi</b> · SSID setup</sub></td>
</tr>
</table>
</div>

## Building from source

This project targets the Xtensa `esp` Rust toolchain.

```bash
# One-time toolchain setup
cargo install espup espflash
espup install --targets esp32
source "$HOME/export-esp.sh"   # add this to your shell profile

# Build, flash, and monitor a connected board
cargo run --release
```

> [!IMPORTANT]
> Always build with `--release`

## Disclaimer

This is a hobby prop project, unofficial and not affiliated with Cult of the Lamb or Massive Monster.
