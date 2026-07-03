<h1 align="center">Crown of the Lamb</h1>

<p align="center">
  <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/actions/workflows/release.yml"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&labelColor=2d333b&color=3fb950"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&color=2ea043"/><img src="https://img.shields.io/github/actions/workflow/status/wielorzeczownik/crown-of-the-lamb/release.yml?branch=main&style=flat-square&labelColor=2d333b&color=3fb950" alt="Release"/></picture></a> <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&labelColor=2d333b&color=3fb950"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&color=2ea043"/><img src="https://img.shields.io/github/v/release/wielorzeczownik/crown-of-the-lamb?style=flat-square&labelColor=2d333b&color=3fb950" alt="Latest Release"/></picture></a> <a href="https://github.com/wielorzeczownik/crown-of-the-lamb/blob/main/LICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/License-MIT-3fb950?style=flat-square&labelColor=2d333b"/><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/License-MIT-2ea043?style=flat-square"/><img src="https://img.shields.io/badge/License-MIT-3fb950?style=flat-square&labelColor=2d333b" alt="License: MIT"/></picture></a>
  <br/>
  <img src="https://img.shields.io/badge/Rust-B7410E?style=flat-square&logo=rust&logoColor=white" alt="Rust"/> <img src="https://img.shields.io/badge/ESP32-000000?style=flat-square&logo=espressif&logoColor=white" alt="ESP32"/> <img src="https://img.shields.io/badge/no__std-embassy-6f42c1?style=flat-square" alt="no_std / embassy"/>
</p>

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/logo.png" alt="Crown of the Lamb" width="300" />
</p>

<p align="center">🇬🇧 <a href="README.md">English</a> | 🇵🇱 Polski</p>

Otwartoźródłowy **firmware na ESP32** napisany w **Rust** (`no_std`, [Embassy](https://embassy.dev)) do animatronicznego rekwizytu – korony z gry **Cult of the Lamb**. Okrągły wyświetlacz **GC9A01** renderuje żywe, mrugające oko, które rozgląda się, reaguje na kierunek dźwięku i wpada w mroczniejsze grymasy – a wszystko ustawisz bezprzewodowo z telefonu przez własny **captive portal WiFi** rekwizytu.

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/demo.jpg" alt="Gotowa korona z zapalonym okiem" width="70%" />
</p>

## Funkcje

- **Animowane oko** na okrągłym wyświetlaczu GC9A01 240×240: rozglądanie się, naturalne mruganie, drganie źrenicy i refleks świetlny.
- **Miny reagujące na dźwięk**: dwa mikrofony zasilają FFT, które szacuje kierunek dźwięku; oko podąża w jego stronę i może zareagować minami **Strach**, **Podejrzliwość**, **Złość** lub **Zażenowanie**.
- **Grzech (Sin)** – mina spoczynkowa pojawiająca się losowo w chwilach ciszy.
- **Punkt dostępowy WiFi + captive portal**: po połączeniu z siecią rekwizytu automatycznie otwiera się panel konfiguracji (DHCP + DNS + HTTP na urządzeniu).
- **Konfiguracja na żywo**: kolor oka, kształt źrenicy, tempo mrugania, próg dźwięku i szanse na poszczególne miny – stosowane natychmiast i zapisywane we flashu.

## Sprzęt

| Element     | Szczegóły                                |
| ----------- | ---------------------------------------- |
| MCU         | ESP32 (np. ESP32-WROOM-32)               |
| Wyświetlacz | GC9A01 240×240, okrągły LCD (SPI)        |
| Mikrofony   | 2× analogowe moduły (np. MAX9814), L + P |
| Zasilanie   | 5V USB lub bateria, zależnie od obudowy  |

### Podłączenie

Dwie grupy: **sygnał** (wyświetlacz + mikrofony do ESP32) i **zasilanie** (bateria → ładowarka → boost → ESP32).

#### Sygnał – wyświetlacz i mikrofony → ESP32

| Moduł         | Pin        | ESP32                            |
| ------------- | ---------- | -------------------------------- |
| GC9A01        | VCC        | 3V3                              |
| GC9A01        | GND        | GND                              |
| GC9A01        | SCL / SCK  | GPIO18 (SPI CLK)                 |
| GC9A01        | SDA / MOSI | GPIO23 (SPI MOSI)                |
| GC9A01        | RES        | GPIO4                            |
| GC9A01        | DC         | GPIO2                            |
| GC9A01        | CS         | GPIO5                            |
| GC9A01        | BL         | GPIO15 (podświetlenie)           |
| MAX9814 lewy  | VDD        | 3V3                              |
| MAX9814 lewy  | GND        | GND                              |
| MAX9814 lewy  | OUT        | GPIO34 (ADC1_CH6, tylko wejście) |
| MAX9814 lewy  | GAIN       | GND (60 dB)                      |
| MAX9814 prawy | VDD        | 3V3                              |
| MAX9814 prawy | GND        | GND                              |
| MAX9814 prawy | OUT        | GPIO35 (ADC1_CH7, tylko wejście) |
| MAX9814 prawy | GAIN       | GND (60 dB)                      |

Umieść mikrofony po przeciwnych stronach korony, żeby różnica głośności L/P dawała użyteczny kierunek dźwięku. Po stronie ESP32 wejścia ADC używają atenuacji 11 dB (ustawione w firmware).

#### Zasilanie – 18650 → TP4056 → włącznik → MT3608 → ESP32

| Skąd        | Dokąd                    | Uwaga                               |
| ----------- | ------------------------ | ----------------------------------- |
| 18650 + / − | TP4056 B+ / B−           | ogniwo do chronionej ładowarki      |
| TP4056 OUT+ | włącznik → MT3608 IN+    | włącznik odcina całe urządzenie     |
| TP4056 OUT− | MT3608 IN− / GND         | wspólna masa                        |
| MT3608 OUT+ | ESP32 5V (VIN)           | **najpierw ustaw MT3608 na 5,0 V!** |
| ESP32 3V3   | GC9A01 VCC + MAX9814 VDD | z wewnętrznego LDO 3,3 V ESP32      |

- Użyj modułu TP4056 **z ochroną** (6 padów: IN±, B±, OUT±).
- Przy podłączonym USB-C TP4056 ładuje ogniwo **i** jednocześnie zasila urządzenie; po odłączeniu włącznik całkowicie odcina pobór z baterii.

> [!TIP]
> Dla stabilnego obrazu dodaj kondensatory przy pinach zasilania:
> ~10 µF + 100 nF na VCC/GND wyświetlacza i na 3V3 ESP32 oraz większy 220-470 µF
> na wyjściu MT3608. Opcjonalne.

## Lista części

### Obudowa (druk 3D)

Korona bazuje na modelu [Red Crown (Cult of the Lamb)](https://www.printables.com/model/326078-red-crown-cult-of-the-lamb/files) autorstwa [ShinpiMakes](https://www.printables.com/@ShinpiMakes_277232) z Printables – wariant **Full Shelled Cross Mount** – przerobionym pod elektronikę:

- wycięty otwór na **oko** (okno wyświetlacza),
- **otwory na mikrofony** po obu bokach (dla kierunku dźwięku L/P),
- wycięcia na **2× port USB-C** (ładowarka + moduł boost) oraz **włącznik**.

Wydruk w skali **120%**, żeby wszystko się zmieściło w środku. Skalowanie może wyjść różnie w zależności od drukarki i slicera, więc jako punkt odniesienia warto przyjąć fizyczny rozmiar: gotowy wydruk powinien mieć **około 11,5 cm** wysokości, od najniższego do najwyższego punktu.

### Części

| Część                                           | Rola                                                                 | Źródło                                                                                                                      |
| ----------------------------------------------- | -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| ESP32 DevKitC-32D                               | główny MCU                                                           | [AliExpress](https://pl.aliexpress.com/item/1005007963451549.html)                                                          |
| Wyświetlacz GC9A01 240×240                      | animowane oko                                                        | [AliExpress](https://pl.aliexpress.com/item/1005004482028005.html)                                                          |
| 2× moduł mikrofonu MAX9814                      | wykrywanie kierunku dźwięku                                          | [AliExpress](https://pl.aliexpress.com/item/1005006109706636.html)                                                          |
| Moduł ładowania USB-C                           | ładowanie ogniw Li-ion                                               | [AliExpress](https://pl.aliexpress.com/item/1005009438702687.html)                                                          |
| Przetwornica MT3608 (USB-C)                     | podbija napięcie baterii do 5 V                                      | [AliExpress](https://pl.aliexpress.com/item/1005011548818999.html)                                                          |
| Ogniwo 18650                                    | bateria                                                              | [AliExpress](https://pl.aliexpress.com/item/1005011665515612.html)                                                          |
| Koszyk 18650                                    | mocowanie ogniw                                                      | [AliExpress](https://pl.aliexpress.com/item/1005005084346241.html)                                                          |
| Włącznik                                        | odcina obieg z baterii                                               | [AliExpress](https://pl.aliexpress.com/item/4000681145062.html)                                                             |
| Przewody jumper / Dupont                        | okablowanie                                                          | [AliExpress](https://pl.aliexpress.com/item/1005006205169394.html)                                                          |
| 2× przedłużacz USB-C (krótki, np. UGREEN 0,5 m) | wyprowadza port ESP32 i ładowania na obudowę – dowolny krótki pasuje | [UGREEN](https://www.ugreen.pl/pl/products/kable/usb-c/przedluzajacy-kabel-ugreen-usb-c-0-5m-3-1-4k-60w-czarny-ed008-60664) |
| Zestaw małych radiatorów                        | chłodzi ESP32 i dystansuje ciepły chip od plastiku wydruku           | [AliExpress](https://pl.aliexpress.com/item/4000266052801.html)                                                             |
| Zestaw dystansów                                | montaż wyświetlacza w odpowiedniej odległości                        | [AliExpress](https://pl.aliexpress.com/item/1005006270641755.html)                                                          |

> [!NOTE]
> Złożone za około **150 zł** w częściach w tamtym czasie – ceny podzespołów od
> tego czasu wzrosły. Oferty AliExpress bywają nietrwałe; gdy link wygaśnie,
> szukaj po nazwie części, każdy równoważny moduł zadziała.

## Montaż

Wszystko wchodzi na styk. Kilka uwag z mojego składania:

- Korona została wydrukowana w całości, a **dół rozciąłem i trochę wyrzeźbiłem** po wydruku, żeby zmieścić elektronikę. Jeśli masz drukarkę, łatwiej **rozdzielić dolną część w slicerze**, żeby wygodnie włożyć większe elementy.
- **Upychaj wszystko w rogi** korony: najpierw **baterie i ESP32**, a dopiero potem wciśnij **przedłużacze USB-C i wyświetlacz**.

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-1.jpg" alt="Złożona korona 1" width="24%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-2.jpg" alt="Złożona korona 2" width="24%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-3.jpg" alt="Złożona korona 3" width="24%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/assembly-4.jpg" alt="Złożona korona 4" width="24%" />
</p>

## Wgrywanie

Pobierz gotowy firmware ze [GitHub Releases](https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest):

- **[crown-of-the-lamb-merged.bin](https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest/download/crown-of-the-lamb-merged.bin)** – obraz scalony (bootloader + tablica partycji + aplikacja) do wgrania czystego ESP32 na offset `0x0`.
- **[crown-of-the-lamb.bin](https://github.com/wielorzeczownik/crown-of-the-lamb/releases/latest/download/crown-of-the-lamb.bin)** – obraz aplikacji do aktualizacji partycji app (`0x10000`).

Wgranie czystej płytki obrazem scalonym:

```bash
espflash write-bin 0x0 crown-of-the-lamb-merged.bin
# albo esptool:
esptool.py --chip esp32 write_flash 0x0 crown-of-the-lamb-merged.bin
```

## Konfiguracja

Po starcie rekwizyt uruchamia punkt dostępowy WiFi. Połącz się telefonem lub laptopem, a captive portal otworzy się automatycznie (albo wejdź na adres bramy `192.168.4.1`).

Z portalu ustawisz na żywo:

- **Kolor oka** (RGB) i wysokość/kształt **źrenicy**
- **Interwał mrugania** i **amplitudę drgania**
- **Próg dźwięku** dla czułości reakcji
- **Szanse na miny** (`Grzech`, `Zażenowanie`, `Strach`, `Podejrzliwość`, `Złość`)

Miny reaktywne losują się niezależnie przy każdej wykrytej zmianie kierunku dźwięku; szansa na **Grzech** losuje się przy każdym mrugnięciu. Wszystkie wartości zapisują się we flashu i wczytują po kolejnym włączeniu.

<p align="center">
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-1.jpg" alt="Interfejs captive portalu 1" width="19%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-2.jpg" alt="Interfejs captive portalu 2" width="19%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-3.jpg" alt="Interfejs captive portalu 3" width="19%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-4.jpg" alt="Interfejs captive portalu 4" width="19%" />
  <img src="https://raw.githubusercontent.com/wielorzeczownik/crown-of-the-lamb/main/assets/portal-5.jpg" alt="Interfejs captive portalu 5" width="19%" />
</p>

## Budowanie ze źródeł

Projekt buduje się toolchainem Xtensa `esp`.

```bash
# Jednorazowa konfiguracja toolchaina
cargo install espup espflash
espup install --targets esp32
source "$HOME/export-esp.sh"   # dodaj do profilu powłoki

# Zbuduj, wgraj i monitoruj podłączoną płytkę
cargo run --release
```

> [!IMPORTANT]
> Zawsze buduj z `--release`

## Zastrzeżenie

To hobbystyczny projekt rekwizytu, nieoficjalny i niezwiązany z Cult of the Lamb ani Massive Monster.
