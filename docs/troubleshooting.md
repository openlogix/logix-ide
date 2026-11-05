# Troubleshooting

## macOS says Logix can't be opened

Logix isn't signed with an Apple Developer ID yet. Before opening the
downloaded `.dmg`, run in Terminal (with your file's name):

```
xattr -c ~/Downloads/logix-darwin-arm64-<date>.dmg
```

Then open it and drag Logix IDE to **Applications** again. See
[Installing](installing.md).

## The FPGA tools don't install

On its first start, Logix needs an internet connection to download the FPGA
tools. If it fails, a red bar shows the problem; check your connection and
click **Retry**.

## Upload can't find the board

- Check that the board is plugged in and powered.
- Try another USB cable: some cables only charge and don't carry data.
- Look next to the board list: **Connected** means a connected
  device fits the selected board. If the board appears under **Connected**
  or **Maybe connected** in the list but isn't the selected one, select it. (Boards with a common USB chip are never
  listed there; Upload still finds them.)
- Open **Tools > Connected Devices…** to see what the computer finds, and
  which boards each device looks like. If the board is listed, check that
  the board selected in the toolbar matches it. A device listed under
  **Found but can't be opened** needs permissions or a driver first; the
  dialog says what to do.

## My design is gone after unplugging the board

The board uploads to RAM, which loses the design when the board is powered
off; when it powers up, it runs whatever design is in its flash. The output
says "Programmed the RAM of the …" after such an upload. If the board can also upload to
flash, choose **Flash** next to the board (see
[Board options](using-logix.md#board-options)); if not, upload again after
powering it up.

## "The project has no pin file"

A design needs a pin file (e.g. `pinout.pcf`) that tells which FPGA pins
connect to what on the board. Start a new project with the board selected
(**File > New Project > Empty Project**), or open one of the board's
examples (**File > New Project > From Example…**), and use its pin file.

A few boards have no examples to take the pins from. For those, add a pin
file yourself (**File > New File…**, e.g. `pinout.pcf` for iCE40 boards),
with the pins from the board's documentation.

## A mistake isn't underlined while typing

Live checking catches mistakes in the grammar, such as a missing `;`. Other
mistakes, such as a name that isn't declared or a wrong type, are found when
you click **Verify**. If nothing is underlined at all, check that **Check
code while typing** is on in **Settings**.

## My board isn't in the list

Logix's build engine, Logix Engine, supports 84 boards so far: iCE40,
ECP5, Gowin and Xilinx boards that program with openFPGALoader, iceprog,
dfu-util or icesprog. More boards are added as their programmers are
supported. A project for a board that isn't supported yet opens, but
Verify and Upload ask you to choose another board.

A board that can be programmed through different USB adapters, or into
flash or RAM, is listed once: choose the adapter or memory next to the
board (see
[Board options](using-logix.md#board-options)).

## An example isn't in the list

Logix comes with examples for the boards it supports; boards it doesn't
support yet have none (see above). Some examples aren't in Logix
yet, such as ones with files in subfolders or several build setups.

## A build or simulation doesn't finish

Click the **Stop** button ![Stop button](images/icon-stop.png){ .toolbar-icon }. A simulation that never ends usually means
the testbench doesn't reach `$finish`.

## Starting over

To reinstall the FPGA tools and reset the settings, quit Logix and delete the
`~/.logix` folder. Your projects aren't affected. The next start downloads
the tools again.

## Still stuck?

Please report a problem with **Help > Report a Problem**, or
[on GitHub](https://github.com/openlogix/logix-ide/issues/new). Include what you did
and what Logix showed (the output area has the details), and attach the
Logix log: **Help > Show Log Folder** opens the folder in Finder; drag
`logix.log` into the report. Logix keeps the logs of its last 5 runs
(`logix.log` is the current or last one, `logix.1.log` the one before). The
log has what the output area showed and what Logix did, including the paths
of your projects; it stays on your computer unless you attach it.
