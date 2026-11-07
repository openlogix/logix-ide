// Live errors and other help while typing, from Verible's Verilog language
// server (started by the backend; see src-tauri/src/language_server.rs).
// The CodeMirror LSP client talks the protocol; the backend only relays
// messages.
import {
  LSPClient,
  LSPPlugin,
  Workspace,
  jumpToDefinitionKeymap,
  type LSPClientExtension,
  type Transport,
  type WorkspaceFile,
} from "@codemirror/lsp-client";
import { ChangeSet, Text } from "@codemirror/state";
import { hoverTooltip, keymap, ViewPlugin, type EditorView } from "@codemirror/view";
import { backend, type Diagnostic } from "./backend";

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

/** A change to an open file, as `Workspace.syncFiles` returns it. */
type WorkspaceFileUpdate = ReturnType<Workspace["syncFiles"]>[number];

/** One project file open in the language server, shown or not. */
class ProjectFile implements WorkspaceFile {
  constructor(
    public uri: string,
    public languageId: string,
    public version: number,
    public doc: Text,
    /** The editor, while the file is the shown tab. */
    public view: EditorView | null,
  ) {}
  getView() {
    return this.view;
  }
}

/** A project file's text, for `ProjectWorkspace.setFiles`. */
export interface OpenFile {
  uri: string;
  languageId: string;
  doc: Text;
}

/**
 * Keeps every Verilog file of the project open in the language server, not
 * only the shown tab (as the client's default workspace does), so each
 * file is checked while typing, shown or not. The shown file follows its
 * editor; the others follow `setFiles`.
 */
export class ProjectWorkspace extends Workspace {
  files: ProjectFile[] = [];
  private versions: Record<string, number> = Object.create(null);
  private pending: WorkspaceFileUpdate[] = [];

  private nextVersion(uri: string): number {
    return (this.versions[uri] = (this.versions[uri] ?? -1) + 1);
  }

  /** Makes `doc` the file's text, to be sent with the next sync. */
  private replaceDoc(file: ProjectFile, doc: Text) {
    const changes = ChangeSet.of({ from: 0, to: file.doc.length, insert: doc }, file.doc.length);
    this.pending.push({ file, prevDoc: file.doc, changes });
    file.doc = doc;
    file.version = this.nextVersion(file.uri);
  }

  syncFiles(): readonly WorkspaceFileUpdate[] {
    const result = this.pending;
    this.pending = [];
    for (const file of this.files) {
      if (!file.view) continue;
      const plugin = LSPPlugin.get(file.view);
      if (!plugin) continue;
      const changes = plugin.unsyncedChanges;
      if (!changes.empty) {
        result.push({ changes, file, prevDoc: file.doc });
        file.doc = file.view.state.doc;
        file.version = this.nextVersion(file.uri);
        plugin.clear();
      }
    }
    return result;
  }

  /**
   * The project's Verilog files: opens new ones, sends changed text of
   * the files not shown, and closes the ones that are gone.
   */
  setFiles(open: OpenFile[]) {
    const uris = new Set(open.map((f) => f.uri));
    for (const file of this.files.filter((f) => !uris.has(f.uri))) {
      this.files = this.files.filter((f) => f !== file);
      this.client.didClose(file.uri);
    }
    for (const f of open) {
      const file = this.getFile(f.uri) as ProjectFile | null;
      if (!file) {
        const added = new ProjectFile(f.uri, f.languageId, this.nextVersion(f.uri), f.doc, null);
        this.files.push(added);
        this.client.didOpen(added);
      } else if (!file.view && !file.doc.eq(f.doc)) {
        this.replaceDoc(file, f.doc);
      }
    }
    if (this.pending.length > 0) this.client.sync();
  }

  openFile(uri: string, languageId: string, view: EditorView) {
    const file = this.getFile(uri) as ProjectFile | null;
    if (!file) {
      const added = new ProjectFile(uri, languageId, this.nextVersion(uri), view.state.doc, view);
      this.files.push(added);
      this.client.didOpen(added);
      return;
    }
    file.view = view;
    if (!file.doc.eq(view.state.doc)) {
      this.replaceDoc(file, view.state.doc);
      queueMicrotask(() => this.client.sync());
    }
  }

  /** The file stays open: it's only no longer the shown tab. */
  closeFile(uri: string, view: EditorView) {
    const file = this.getFile(uri) as ProjectFile | null;
    if (file?.view === view) file.view = null;
  }
}

/** Live problems of one file, by its address, as the server sends them. */
export type LiveProblemsHandler = (uri: string, problems: Diagnostic[]) => void;

/** Sends the shown file's edits half a second after typing stops. */
const autoSync = ViewPlugin.fromClass(
  class {
    pending: ReturnType<typeof setTimeout> | null = null;
    update(update: { docChanged: boolean; view: EditorView }) {
      if (!update.docChanged) return;
      if (this.pending) clearTimeout(this.pending);
      this.pending = setTimeout(() => {
        this.pending = null;
        LSPPlugin.get(update.view)?.client.sync();
      }, 500);
    }
    destroy() {
      if (this.pending) clearTimeout(this.pending);
    }
  },
);

/** LSP severities: 1 error, 2 warning, 3 information, 4 hint. */
export function lspProblems(
  diagnostics: { range: { start: { line: number; character: number } }; severity?: number; message: string }[],
): Diagnostic[] {
  return diagnostics.map((d) => ({
    severity: (d.severity ?? 1) === 1 ? "error" : "warning",
    file: null,
    line: d.range.start.line + 1,
    column: d.range.start.character + 1,
    message: d.message,
  }));
}

/**
 * Errors as you type, for every open file (shown or not): passed to
 * `onProblems`, which shows them with the others (see problems.ts).
 */
function liveProblems(onProblems: LiveProblemsHandler): LSPClientExtension {
  return {
    clientCapabilities: { textDocument: { publishDiagnostics: { versionSupport: true } } },
    notificationHandlers: {
      "textDocument/publishDiagnostics": (client, params) => {
        const file = client.workspace.getFile(params.uri);
        // An answer for an older version of the text: a newer one follows.
        if (!file || (params.version != null && params.version !== file.version)) return true;
        onProblems(params.uri, lspProblems(params.diagnostics));
        return true;
      },
    },
    editorExtension: autoSync,
  };
}

/**
 * Starts the language server and returns a connected client, ready to be
 * attached to editor documents (`client.plugin(uri)`). Every project file
 * is checked (`ProjectWorkspace.setFiles`), and its problems go to
 * `onProblems`.
 */
export async function startLanguageServer(onProblems: LiveProblemsHandler): Promise<LSPClient> {
  await backend.lspStart();
  const client = new LSPClient({
    workspace: (c) => new ProjectWorkspace(c),
    extensions: [
      // Errors as you type, in every file.
      liveProblems(onProblems),
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
