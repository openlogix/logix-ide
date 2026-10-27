// Live errors and other help while typing, from Verible's Verilog language
// server (started by the backend; see src-tauri/src/apio/language_server.rs).
// The CodeMirror LSP client talks the protocol; the backend only relays
// messages.
import {
  LSPClient,
  LSPPlugin,
  jumpToDefinitionKeymap,
  serverDiagnostics,
  type Transport,
} from "@codemirror/lsp-client";
import { hoverTooltip, keymap, type EditorView } from "@codemirror/view";
import { backend } from "./backend";

/** Relays LSP messages through the backend. */
function backendTransport(): Transport {
  const handlers = new Set<(message: string) => void>();
  backend.onLspMessage((message) => handlers.forEach((h) => h(message)));
  return {
    send(message) {
      backend.lspSend(message).catch(() => {
        // The server stopped; the app is told through onLspClosed.
      });
    },
    subscribe: (handler) => handlers.add(handler),
    unsubscribe: (handler) => handlers.delete(handler),
  };
}

/**
 * Starts the language server and returns a connected client, ready to be
 * attached to editor documents (`client.plugin(uri)`).
 */
export async function startLanguageServer(): Promise<LSPClient> {
  await backend.lspStart();
  const client = new LSPClient({
    extensions: [
      // Errors as you type (shown like Verify's).
      serverDiagnostics(),
      // Information about the symbol under the mouse.
      hoverTooltip(hoverSource, { hideOn: (tr) => tr.docChanged }),
      // F12: jump to where a signal or module is defined.
      keymap.of(jumpToDefinitionKeymap),
    ],
  });
  client.connect(backendTransport());
  await client.initializing;
  return client;
}

// LSP hover contents: a string or {language, value} ("marked strings", one
// or a list), or {kind, value} (markup).
type MarkedString = string | { language: string; value: string };
type HoverContents =
  | MarkedString
  | MarkedString[]
  | { kind: string; value: string };

/**
 * Tidies Verible's hover Markdown. It starts with a title naming Verible's
 * internal category, e.g. "### data/net/var/instance counter", followed by
 * the useful part ("Type: reg [23:0]") between "---" lines. Keeps the name
 * (in bold) and the useful part; returns "" when there's nothing beyond the
 * title (e.g. for ports and module names). Other Markdown is kept as is.
 */
export function tidyVeribleHover(markdown: string): string {
  const lines = markdown.split("\n");
  const title = /^###\s+(.*\S)\s+(\S+)\s*$/.exec(lines[0] ?? "");
  if (!title) return markdown;
  const body = lines
    .slice(1)
    .filter((line) => line.trim() !== "" && line.trim() !== "---");
  if (body.length === 0) return "";
  return [`**${title[2]}**`, ...body].join("\n\n");
}

/**
 * The hover contents as Markdown snippets, without empty ones. (Verible
 * answers with empty contents for most of the code.)
 */
export function hoverMarkdown(contents: HoverContents | null | undefined): string[] {
  if (!contents) return [];
  const list = Array.isArray(contents) ? contents : [contents];
  return list
    .map((item) => {
      if (typeof item === "string") return tidyVeribleHover(item);
      if ("language" in item) {
        // An empty code block would still draw a box.
        if (item.value.trim() === "") return "";
        return "```" + item.language + "\n" + item.value + "\n```";
      }
      return tidyVeribleHover(item.value);
    })
    .filter((text) => text.trim() !== "");
}

/** Settings > "Show info when hovering" (off by default). */
let hoverEnabled = false;

export function setHoverEnabled(enabled: boolean) {
  hoverEnabled = enabled;
}

/**
 * Hover tooltips, like the LSP client's own (`hoverTooltips`), except that
 * they can be turned off and empty answers show nothing instead of an
 * empty box.
 */
async function hoverSource(view: EditorView, pos: number) {
  if (!hoverEnabled) return null;
  const plugin = LSPPlugin.get(view);
  if (!plugin || !plugin.client.serverCapabilities?.hoverProvider) return null;
  plugin.client.sync();
  const result = await plugin.client
    .request<
      object,
      { contents: HoverContents; range?: { start: { line: number; character: number } } } | null
    >("textDocument/hover", {
      position: plugin.toPosition(pos),
      textDocument: { uri: plugin.uri },
    })
    .catch(() => null);
  const parts = hoverMarkdown(result?.contents);
  if (!result || parts.length === 0) return null;
  return {
    pos: result.range ? plugin.fromPosition(result.range.start, view.state.doc) : pos,
    above: true,
    create() {
      const dom = document.createElement("div");
      dom.className = "cm-lsp-hover-tooltip cm-lsp-documentation";
      dom.innerHTML = parts.map((p) => plugin.docToHTML(p, "markdown")).join("<br>");
      return { dom };
    },
  };
}

/**
 * The LSP address of a project file: its real path for a saved project
 * (`dir`), else a placeholder (the server only needs a unique address).
 */
export function fileUri(dir: string | null, file: string): string {
  if (dir) return `file://${encodeURI(`${dir}/${file}`)}`;
  return `file:///logix-unsaved/${encodeURIComponent(file)}`;
}
