// Pin constraint files in the editor: each format's line comment, for
// Comment/Uncomment (⌘/) and for coloring comments.

import { StreamLanguage } from "@codemirror/language";

/**
 * The line comment of a pin file format: `#` for PCF (iCE40), LPF (ECP5)
 * and XDC (Xilinx, Tcl), `//` for CST (Gowin). Null for other files.
 * These formats have no block comments the tools all accept, so only line
 * comments are used.
 */
export function pinCommentToken(file: string): "#" | "//" | null {
  const ext = /\.([^./\\]+)$/.exec(file)?.[1].toLowerCase();
  switch (ext) {
    case "pcf":
    case "lpf":
    case "xdc":
      return "#";
    case "cst":
      return "//";
    default:
      return null;
  }
}

const languages = new Map<string, StreamLanguage<unknown>>();

/**
 * A minimal editor language for a pin file: comments and quoted names are
 * colored, and Comment/Uncomment uses the format's line comment. Null for
 * files that aren't pin files.
 */
export function pinLanguage(file: string): StreamLanguage<unknown> | null {
  const token = pinCommentToken(file);
  if (!token) return null;
  let language = languages.get(token);
  if (!language) {
    language = StreamLanguage.define({
      name: `pins${token}`,
      token(stream) {
        if (stream.match(token)) {
          stream.skipToEnd();
          return "comment";
        }
        if (stream.match(/^"[^"]*"?/)) return "string";
        if (stream.match(/^-?\d+(\.\d+)?\b/)) return "number";
        stream.next();
        return null;
      },
      languageData: { commentTokens: { line: token } },
    });
    languages.set(token, language);
  }
  return language;
}
