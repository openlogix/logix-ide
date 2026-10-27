# Installing Logix

Logix needs a Mac with **Apple Silicon** (M1 or newer) and macOS 11 or newer.

## Download and install

1. Download the latest `logix-darwin-arm64-<date>.dmg` from the
   [releases page](https://github.com/zapta/logix/releases) to your
   **Downloads** folder.
2. Open **Terminal** and run this command, with the file name you downloaded.
   Logix isn't signed with an Apple Developer ID yet, and this tells macOS to
   allow it:

    ```
    xattr -c ~/Downloads/logix-darwin-arm64-<date>.dmg
    ```

3. Double-click the `.dmg` file and drag **Logix** into the **Applications**
   folder.
4. Open Logix from **Applications**.

## The first start

On its first start, Logix downloads the FPGA tools it needs (Apio and its
packages). This may take several minutes and needs an internet connection; a
banner at the top shows the progress, and the editor area says it's getting
the FPGA tools ready. When the output area shows **Ready.**, Logix opens a
first project for the Alhambra II board, and you can start.

Later starts are quick and also work without internet.

## Where Logix keeps its files

| What | Where |
| :--- | :--- |
| Your projects (by default) | `~/Documents/Logix` |
| The FPGA tools and settings | `~/.logix` (about 3 GB) |

You can choose another projects folder in **Settings** (++cmd+comma++).

## Uninstalling

Drag Logix from **Applications** to the Trash. To also remove the FPGA tools
and settings, delete the `~/.logix` folder. Your projects stay where they are.
