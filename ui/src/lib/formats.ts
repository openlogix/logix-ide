// Which files are Verilog sources, includes and pin files, and each pin
// file format's line comment, as the backend gives them (`file_formats`):
// Logix Engine's (`get_engine_info`'s `hdl_extensions`, `get_catalog`'s
// families' `constraint_extension` and `constraint_comment`), or the
// backend's built-in copy until the engine has answered (Logix IDE reopens
// the last project before the engine is up). The app sets them at startup
// and again once the engine's catalog is loaded (`setFileFormats`).

import type { FileFormats } from "./backend";

let formats: FileFormats = { source: [], include: [], pins: [] };

/** Uses these formats from now on. */
export function setFileFormats(f: FileFormats) {
  formats = f;
}

/** A file name's extension, lowercase, without the dot (null without one). */
function extension(name: string): string | null {
  return /\.([^./\\]+)$/.exec(name)?.[1].toLowerCase() ?? null;
}

const has = (list: string[], name: string) => {
  const e = extension(name);
  return e !== null && list.some((x) => x.toLowerCase() === e);
};

/** Verilog or SystemVerilog, a source or an include (what Auto Format takes). */
export function isHdlSource(name: string): boolean {
  return has(formats.source, name) || has(formats.include, name);
}

/** A file other Verilog files include, such as `defs.vh`. */
export function isInclude(name: string): boolean {
  return has(formats.include, name);
}

/** A pin constraint file of any FPGA family's format. */
export function isPinFile(name: string): boolean {
  return pinComment(name) !== null;
}

/**
 * What starts a comment line in a pin file: `#`, or `//` for Gowin's
 * `.cst`, by Logix Engine. Null for other files.
 */
export function pinComment(name: string): string | null {
  const e = extension(name);
  return formats.pins.find((p) => p.extension.toLowerCase() === e)?.comment ?? null;
}
