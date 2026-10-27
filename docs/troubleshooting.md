# Troubleshooting

## macOS says Logix can't be opened

Logix isn't signed with an Apple Developer ID yet. Before opening the
downloaded `.dmg`, run in Terminal (with your file's name):

```
xattr -c ~/Downloads/logix-darwin-arm64-<date>.dmg
```

Then open it and drag Logix to **Applications** again. See
[Installing](installing.md).

## The FPGA tools don't install

On its first start, Logix needs an internet connection to download the FPGA
tools. If it fails, a red bar shows the problem; check your connection and
click **Retry**.

## Upload can't find the board

- Check that the board is plugged in and powered.
- Try another USB cable: some cables only charge and don't carry data.
- Look next to the board list: **Connected** means Logix recognizes the
  selected board. If the board appears under **Connected** in the list but
  isn't the selected one, select it. (Boards with a common USB chip are never
  listed there; Upload still finds them.)
- Open **Tools > Connected Devices…** to see what the computer finds. If the
  board is listed, check that the board selected in the toolbar matches it.

## "The project has no pin file"

A design needs a pin file (e.g. `pinout.pcf`) that tells which FPGA pins
connect to what on the board. Start a new project with the board selected
(**File > New**), or open the board's **BareMinimum** or another of its
examples (**File > Examples…**), and use its pin file.

A few boards have no examples to take the pins from. For those, add a pin
file yourself (**File > New File…**, e.g. `pinout.pcf` for iCE40 boards),
with the pins from the board's documentation.

## A mistake isn't underlined while typing

Live checking catches mistakes in the grammar, such as a missing `;`. Other
mistakes, such as a name that isn't declared or a wrong type, are found when
you click **Verify**. If nothing is underlined at all, check that **Check
code while typing** is on in **Settings**.

## An example isn't in the list

A few examples use features Logix doesn't support yet (such as files in
subfolders, or several build setups), so they're not listed.

## A build or simulation doesn't finish

Click **Stop** (++cmd+period++). A simulation that never ends usually means
the testbench doesn't reach `$finish`.

## Starting over

To reinstall the FPGA tools and reset the settings, quit Logix and delete the
`~/.logix` folder. Your projects aren't affected. The next start downloads
the tools again.

## Still stuck?

Please report a problem with **Help > Report a Problem**, or
[on GitHub](https://github.com/zapta/logix/issues/new). Include what you did
and what Logix showed (the output area has the details), and attach the
Logix log: **Help > Show Log Folder** opens the folder in Finder; drag
`logix.log` into the report. Logix keeps the logs of its last 5 runs
(`logix.log` is the current or last one, `logix.1.log` the one before). The
log has what the output area showed and what Logix did, including the paths
of your projects; it stays on your computer unless you attach it.
