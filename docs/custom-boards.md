# Custom boards

Logix IDE knows the
[supported boards](https://www.openlogix.org/logix-catalog/boards/). When
yours isn't on the list, or you want to program a supported board another
way, describe it yourself in a small file: a **custom board**. Logix IDE
then lists it, builds for it and uploads to it like any other board.

A custom board can use any of the
[supported FPGAs](https://www.openlogix.org/logix-catalog/fpgas/), also
those no supported board has yet. Custom boards add boards,
never FPGAs.

## How it works

- A project's custom boards are in a folder named **boards** in the project
  folder, one file per board.
- Each file is named after the board's **ID**, such as
  **boards/my-upduino.json** for the board `my-upduino`. An ID has only
  lowercase letters, digits and dashes.
- The files are in the same format as the supported boards, Logix Catalog's
  [board format](https://www.openlogix.org/logix-catalog/model/). So you can
  start from a supported board's file, and a board you describe can be added
  to the supported boards for everyone, as it is.
- You write the files with any text editor, such as VS Code. Logix IDE reads
  them but never edits them.
- They're part of the project: anyone who opens the project, on any
  computer, gets the same boards.

## Add a custom board, step by step

1. **Open a project for the board.** Custom boards belong to a project, and
   new projects can't start from one. Start a new project from an example of
   a supported board with the same FPGA family as yours, such as an iCE40
   board for an iCE40 FPGA, so its pin file has the right kind
   (`.pcf`, `.lpf`, `.cst` or `.xdc`), and save it. Or open a project you
   already have.

2. **Find a similar supported board.** Open **Tools > Board Explorer…**, pick
   **FPGAs**, and find your board's FPGA: its details list the boards that
   use it. A board with the same FPGA, or the same programming chip (most
   often an FTDI chip), is the best start. Note its ID, shown under its name
   in the [supported boards](https://www.openlogix.org/logix-catalog/boards/)
   list.

3. **Copy its file.** Each supported board's file is in Logix Catalog's
   [boards folder](https://github.com/openlogix/logix-catalog/tree/main/boards)
   on GitHub, named after its ID. Open it, and copy its text (the **Copy raw
   file** button).

4. **Make the boards folder.** **Project > Show Project Folder** opens the
   project folder in Finder (File Explorer on Windows). Make a folder named
   **boards** there, and in it a new file named after your board's ID, such
   as **my-upduino.json**. Paste the text into it.

5. **Describe your board.** Change the fields as the next sections say: its
   `name`, its FPGA (`fpga_id`), how to program it (`variants`) and,
   optionally, what's on it (`resources`). Leave out what you don't know.

6. **Save the file.** Logix IDE notices it within a second. Your board is
   listed first in the board list, under **Custom boards**, and in its FPGA
   family's group too, marked with a small circuit board. Pick it.

7. **Edit the pin file.** The project's pin file says which FPGA pin each of
   the design's signals is on. Change it to your board's pins: its clock,
   LEDs and buttons, from your board's schematic or its maker's docs.

8. **Verify and upload.** Click **Verify**, then **Upload**. The output says
   when a command uses a custom board. When the upload works, set `tested`
   to `true` (see below).

The example **custom-board** for the Alhambra II (in **File > Examples…**,
under the Alhambra II) has a boards folder with one board, the
Alhambra II programmed with `iceprog` instead of the usual `openFPGALoader`.

!!! tip "Let an AI agent write it"
    An AI agent working beside Logix IDE (see
    [Using an AI agent](ai-agents.md)) can write the file for you: give it
    your board's name, its FPGA, its USB IDs and a link to its schematic or
    docs, and ask it for a custom board in the project's **boards** folder.
    Check what it wrote before you upload.

## A board's file

A small but complete file, for an UPduino programmed over its own USB port:

```json
{
  "$schema": "https://www.openlogix.org/schemas/logix-board-v1.json",
  "name": "My UPduino",
  "fpga_id": "ice40up5k-sg48",
  "variants": [
    {
      "selector": {"connection": "usb", "memory": "flash"},
      "programmer": "iceprog",
      "args": "-d d:${BUS}/${DEV} ${BITSTREAM}",
      "usb": {"vid": "0403", "pid": "6014"},
      "tested": false
    }
  ],
  "resources": {
    "clk": {"type": "clock", "pin": "35", "mhz": 12},
    "rgb": {"type": "rgb_led", "red": "41", "green": "39", "blue": "40", "active": "low"}
  }
}
```

| Field | Required | What it holds |
| --- | --- | --- |
| `$schema` | no | The format's address, always the one above. With it, editors such as VS Code check the file and complete its fields as you type |
| `name` | yes | The board's name, as the board list shows it |
| `fpga_id` | yes | The board's FPGA, by its FPGA ID: in the Board Explorer, pick **FPGAs** and select it. The exact chip and package, such as `ice40up5k-sg48` |
| `variants` | yes | The ways to program the board, the first being the default (see Variants) |
| `requires` | no | What every design for the board must do, such as a pin to hold high |
| `resources` | no | What the board wires to FPGA pins: clocks, LEDs, buttons, UARTs, PMODs and headers (see Resources) |
| `ai_hints` | no | Short lines for AI agents writing designs for the board, about what the other fields can't say. Always the last field |

Logix Catalog's [board format](https://www.openlogix.org/logix-catalog/model/)
describes every field in full.

### Variants

Each variant is one way to program the board. Most boards have one or two,
such as writing the flash (kept at power-off) or the FPGA's RAM (lost at
power-off). The [board options](using-logix.md#board-options) next to the
board choose between them.

| Field | What it holds |
| --- | --- |
| `selector` | What tells this variant apart: `connection`, how the computer reaches the board (`usb`, its own USB port; `bootloader`, its USB port in bootloader mode; `ft2232h`, `ft232h`, `cmsis-dap` or `usb-blaster`, an external adapter; or `unspecified`), and `memory`, `flash` or `ram`. No two variants share one |
| `programmer` | The program that uploads: `openfpgaloader`, `iceprog`, `dfu-util` or `icesprog`. Logix IDE gets them with the FPGA tools |
| `args` | The programmer's arguments, with these variables: `${BITSTREAM}` (the file to upload), `${BUS}` and `${DEV}` (the board's USB bus and device numbers), `${VID}` and `${PID}` (its USB IDs), `${SERIAL_NUM}` (its serial number) and `${USB_PATH}` (its USB port, as `dfu-util` takes it) |
| `usb` | How Logix IDE finds the board among connected USB devices: `vid` and `pid`, its USB IDs as four hex digits each, and optionally `product_regex` (its product name), `serial_regex` (its serial number) and `release` (its chip's release number), for boards that share USB IDs |
| `tested` | `true` once uploading this way is known to work |
| `user_notes` | Optional, a note for people using the board |

The arguments depend on the programmer and the board's USB chip, so copy
them from a supported board that's programmed the same way. For
`openfpgaloader`, the board may also be one it knows by name (`-b` and the
name), as many supported boards are.

### Your board's USB IDs

The `usb` filter needs your board's USB IDs. Plug the board in, then:

=== "macOS"

    Open **System Information** (hold Option and choose **Apple menu >
    System Information**), then **Hardware > USB**, and select your board:
    **Vendor ID** and **Product ID** are its USB IDs, such as `0x0403` and
    `0x6014` (write them without `0x`).

=== "Windows"

    Open **Device Manager**, find your board (often under **Universal
    Serial Bus controllers** or **Ports**), right-click it and choose
    **Properties**, then **Details**, and pick **Hardware Ids**: the line
    `USB\VID_0403&PID_6014` gives the USB IDs `0403` and `6014`.

=== "Linux"

    Run `lsusb` in a terminal. Your board's line has its USB IDs, such as
    `ID 0403:6014`.

On Windows and Linux, Upload offers the driver or udev rule your board
needs, as for supported boards (see
[Drivers on Windows](using-logix.md#drivers-on-windows) and
[udev rules on Linux](using-logix.md#udev-rules-on-linux)).

### Resources

`resources` lists what the board wires to FPGA pins, each by the name a
design would give its port, such as `clk` or `leds`. The Board Explorer
shows them, and AI agents use them to write designs for the board. They're
optional: the build uses the project's pin file, not them.

| Type | Fields | Example |
| --- | --- | --- |
| `clock` | `pin`, `mhz` | `"clk": {"type": "clock", "pin": "35", "mhz": 12}` |
| `leds` | `pins` (bit 0 first), `active` | `"leds": {"type": "leds", "pins": ["11", "37"], "active": "low"}` |
| `rgb_led` | `red`, `green`, `blue`, `active` | `"rgb": {"type": "rgb_led", "red": "41", "green": "39", "blue": "40", "active": "low"}` |
| `buttons`, `switches` | `pins`, `active` | `"btn": {"type": "buttons", "pins": ["10"], "active": "low"}` |
| `uart` | `tx` (the FPGA's output), `rx` | `"uart": {"type": "uart", "tx": "9", "rx": "6"}` |
| `pmod` | `pins`: the 8 signal pins, 1 to 4 then 7 to 10 | `"pmod1": {"type": "pmod", "pins": ["4", "2", "47", "45", "3", "48", "46", "44"]}` |
| `header` | `positions`: what each position is wired to | other connectors |

`active` is `high` or `low`: whether the LED lights, or the button reads,
at a high or a low level. Leave it out if you don't know. Pins are named as
the FPGA's pin file names them, such as `35` for iCE40 boards or `A10` for
others.

## When something's wrong

If a file has a mistake, the output lists each one with the file's full
path, line and column, such as
`/Users/you/Documents/OpenLogix/IDE/blinky/boards/my-upduino.json:4:3: error: …`.
The board list's **Custom boards** group says the boards folder has
problems, and its boards aren't offered until it's fixed. Verify and Upload
stop, and the status line says the boards folder has problems. Fix the file
and save it: the output says when the folder has no problems.

Common mistakes:

- The file name isn't the board's ID with `.json`, or the ID has capital
  letters, spaces or underscores.
- `fpga_id` isn't an FPGA ID. Copy it from the Board Explorer.
- A missing comma or quote. With the `$schema` line, your editor marks it
  as you type.
- Upload doesn't find the board: check its USB IDs, and the variant's
  `connection`.

## Replace a supported board

A custom board with the same ID as a supported board replaces it in this
project, such as to program it another way. Copy the supported board's
file to **boards/** with its name unchanged, and edit it.

## Share it with everyone

A custom board is already in the supported boards' format. To add it to
them, so every Open Logix tool knows it, send its file to Logix Catalog as
a pull request: see
[Adding or fixing a board](https://www.openlogix.org/logix-catalog/#adding-or-fixing-a-board).
