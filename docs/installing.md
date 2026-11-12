# Installing Logix IDE

Logix IDE needs a Mac with **Apple Silicon** (M1 or newer) and macOS 11 or newer.

## Download and install

1. Download the latest `logix-darwin-arm64-<date>.dmg` from the
   [releases page](https://github.com/openlogix/logix-ide/releases) to your
   **Downloads** folder.
2. Open **Terminal** and run this command.
   Logix IDE isn't signed with an Apple Developer ID yet, and this tells macOS to
   allow it:

    ```
    xattr -c ~/Downloads/logix-darwin-arm64-*.dmg
    ```

3. Double-click the `.dmg` file and drag **Logix IDE** into the **Applications**
   folder.
4. Open Logix IDE from **Applications**.

## The first start

On its first start, Logix IDE downloads the FPGA tools it needs (Logix Engine
and its packages). This may take a few minutes and needs an internet connection; the
status bar at the bottom of the window shows the progress, and the editor area says it's getting
the FPGA tools ready. When the output area shows **Ready.**, you can start:
the start screen offers a new project, an example, or your recent
projects.

Later starts are quick and also work without internet. Once a day, when you
use it, Logix IDE checks for newer releases of Logix Engine and its FPGA tools
and downloads them by itself (**Help > Check for Updates** checks now).

## Where Logix IDE keeps its files

| What | Where |
| :--- | :--- |
| Your projects (by default) | `~/Documents/OpenLogix/IDE` |
| The FPGA tools and settings | `~/.logix/ide` |

You can choose another projects folder in **Settings**.

## Uninstalling

Drag Logix IDE from **Applications** to the Trash. To also remove the FPGA tools
and settings, delete the `~/.logix/ide` folder. Your projects stay where they are.
