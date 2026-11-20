# Installing Logix IDE

Logix IDE runs on:

- **macOS** on a Mac with **Apple Silicon** (M1 or newer), macOS 11 or newer.
- **Windows** on a 64-bit (x64) PC, Windows 10 or 11.
- **Linux** on a 64-bit (x64) PC: Ubuntu 22.04 or newer, Debian, or a system
  based on them.

Always install the [**Latest** release](https://github.com/openlogix/logix-ide/releases/latest). Newer pre-releases on the
[releases page](https://github.com/openlogix/logix-ide/releases) are
development builds.

## Download and install

1. Open the [**Latest** release](https://github.com/openlogix/logix-ide/releases/latest).
2. Follow the steps for your system on that page: macOS, Windows or Linux.
   They say exactly what that release needs, such as a step for files that
   aren't signed yet, and how to uninstall.

Then, by system (the tabs on every page of these docs follow your choice):

=== "macOS"

    Nothing else: no drivers are needed.

=== "Windows"

    Many boards need the WinUSB driver before Logix IDE can program them on
    Windows. You don't need to install it beforehand: **Upload** offers it
    the first time. See [Drivers on Windows](using-logix.md#drivers-on-windows).

=== "Linux"

    Most boards need a udev rule before Logix IDE can program them on Linux.
    You don't need to add it beforehand: **Upload** offers it the first
    time. See [udev rules on Linux](using-logix.md#udev-rules-on-linux).

## The first start

On its first start, Logix IDE downloads the FPGA tools it needs (Logix Engine
and its packages). This may take a few minutes and needs an internet connection; the
status bar at the bottom of the window shows the progress, and the editor area says it's getting
the FPGA tools ready. When the output area shows **Ready.**, you can start:
the start screen offers a new project, an example, or your recent
projects.

Later starts are quick and also work without internet. Once a day, when you
use it, Logix IDE checks for newer releases of Logix Engine, its FPGA tools and
the examples, and downloads them by itself (**Help > Check for Updates** checks now).
**Help > Check for Updates** also tells you when a newer Logix IDE is
available, with the page to download it from; Logix IDE never updates
itself.

## Where Logix IDE keeps its files

=== "macOS"

    | What | Where |
    | :--- | :--- |
    | Your projects (by default) | `~/Documents/OpenLogix/IDE` |
    | The FPGA tools, settings and logs | `~/.logix/ide` |

=== "Windows"

    | What | Where |
    | :--- | :--- |
    | Your projects (by default) | `Documents\OpenLogix\IDE` in your user folder |
    | The FPGA tools, settings and logs | `.logix\ide` in your user folder |

=== "Linux"

    | What | Where |
    | :--- | :--- |
    | Your projects (by default) | `~/Documents/OpenLogix/IDE` |
    | The FPGA tools, settings and logs | `~/.logix/ide` |

You can choose another projects folder in **Settings**.

## Uninstalling

The steps for your system on the
[**Latest** release](https://github.com/openlogix/logix-ide/releases/latest)
say how to uninstall Logix IDE, and how to also remove its FPGA tools,
settings and logs.

=== "macOS"

    Nothing to do first.

=== "Windows"

    If Logix IDE installed the WinUSB driver for a board and you want the
    board maker's tools to use the board again, first remove the driver in
    **Tools > Connected Devices…** (**Remove Driver…**).

=== "Linux"

    If Logix IDE added a udev rule for a board, uninstalling leaves it in
    place. To remove it, first use **Tools > Connected Devices…**
    (**Remove udev Rule…**).

Your projects stay where they are.
