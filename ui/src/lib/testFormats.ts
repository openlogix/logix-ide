// The file formats as Logix Engine gives them (`get_engine_info`'s
// `hdl_extensions`, the families' pin file extension and comment), for unit
// tests of code that reads them (formats.ts).
import type { FileFormats } from "./backend";

export const ENGINE_FORMATS: FileFormats = {
  source: ["v", "sv"],
  include: ["vh", "svh"],
  pins: [
    { extension: "pcf", comment: "#" },
    { extension: "lpf", comment: "#" },
    { extension: "cst", comment: "//" },
    { extension: "xdc", comment: "#" },
  ],
};
