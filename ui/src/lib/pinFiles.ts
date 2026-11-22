// Pin constraint files in the editor: each format's line comment (Logix
// Engine's `constraint_comment`, see formats.ts), for Comment/Uncomment (⌘/)
// and for coloring comments. These formats have no block comments the tools
// all accept, so only line comments are used.

import { StreamLanguage } from "@codemirror/language";
import { pinComment } from "./formats";

const languages = new Map<string, StreamLanguage<unknown>>();

/**
 * A minimal editor language for a pin file: comments and quoted names are
 * colored, and Comment/Uncomment uses the format's line comment. Null for
 * files that aren't pin files.
 */
export function pinLanguage(file: string): StreamLanguage<unknown> | null {
  const token = pinComment(file);
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
