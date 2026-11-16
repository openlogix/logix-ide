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

Choose your system: the steps on every page of these docs follow your choice.

=== "macOS"

    1. From the [**Latest** release](https://github.com/openlogix/logix-ide/releases/latest), download
       `logix-ide-darwin-arm64-<date>.dmg` to your **Downloads** folder.
    2. Open **Terminal** and run this command. Logix IDE isn't signed with an
       Apple Developer ID yet, and this tells macOS to allow it:

        ```
        xattr -c ~/Downloads/logix-ide-darwin-arm64-*.dmg
        ```

    3. Double-click the `.dmg` file and drag **Logix IDE** into the
       **Applications** folder. Dragging only copies it: nothing opens, and
       that's expected.
    4. Start Logix IDE: open **Finder**, click **Applications** in the sidebar
       and double-click **Logix IDE** (or press Command-Space, type
       `Logix IDE` and press Return). You can then eject the **Logix IDE**
       disk in Finder's sidebar.

=== "Windows"

    1. From the [**Latest** release](https://github.com/openlogix/logix-ide/releases/latest), download
       `logix-ide-windows-x86-64-<date>.msi`.
    2. Double-click it. If Windows says **Windows protected your PC**, click
       **More info**, then **Run anyway**: Logix IDE isn't signed yet.
    3. Follow the installer. When Windows asks whether the installer may make
       changes to your device, choose **Yes**. If your PC doesn't have
       Microsoft Edge WebView2 yet, the installer downloads it.
    4. Start Logix IDE from the **Start** menu.

    Many boards need the WinUSB driver before Logix IDE can program them on
    Windows. You don't need to install it beforehand: **Upload** offers it
    the first time. See [Drivers on Windows](using-logix.md#drivers-on-windows).

=== "Linux"

    Logix IDE comes as a `.deb` package, for Ubuntu, Debian and the systems
    based on them.

    1. From the [**Latest** release](https://github.com/openlogix/logix-ide/releases/latest), download
       `logix-ide-linux-x86-64-<date>.deb` to your **Downloads** folder.
    2. In a terminal, install it. apt also installs what Logix IDE needs, and
       asks for your password:

        ```
        sudo apt install ~/Downloads/logix-ide-linux-x86-64-*.deb
        ```

    3. Start Logix IDE from your applications: press the Super key, type
       `Logix IDE` and press Enter.

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

=== "macOS"

    Drag Logix IDE from **Applications** to the Trash. To also remove the FPGA
    tools, settings and logs, delete the `~/.logix/ide` folder.

=== "Windows"

    If Logix IDE installed the WinUSB driver for a board and you want the
    board maker's tools to use the board again, first remove the driver in
    **Tools > Connected Devices…** (**Remove Driver…**).

    Then open **Settings**, then **Apps** > **Installed apps**, and uninstall
    **Logix IDE**. To also remove the FPGA tools, settings and logs, delete the
    `.logix\ide` folder in your user folder.

=== "Linux"

    In a terminal, run `sudo apt remove logix-ide`. To also remove the FPGA
    tools, settings and logs, delete the `~/.logix/ide` folder.

Your projects stay where they are.
