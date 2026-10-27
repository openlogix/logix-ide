<!--
  The code editor (CodeMirror 6). Holds one document per project file, so
  each tab keeps its own undo history and cursor position.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { basicSetup } from "codemirror";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { indentUnit, StreamLanguage } from "@codemirror/language";
  import { verilog } from "@codemirror/legacy-modes/mode/verilog";
  import { oneDark } from "@codemirror/theme-one-dark";
  import {
    lintGutter,
    setDiagnostics as setLintDiagnostics,
    type Diagnostic as LintDiagnostic,
  } from "@codemirror/lint";
  import type { Diagnostic } from "./backend";
  import {
    selectAll as cmSelectAll,
    toggleComment as cmToggleComment,
  } from "@codemirror/commands";
  import { jumpToDefinition, type LSPClient } from "@codemirror/lsp-client";
  import { fileUri } from "./lsp";

  let {
    onChange,
    onContextMenu,
    dark = false,
    lsp = null,
    projectDir = null,
  }: {
    onChange: (file: string, text: string) => void;
    /**
     * Right-click in the text, after the cursor moved there. `canGoToDefinition`
     * is whether the live checker can look up the name there.
     */
    onContextMenu?: (canGoToDefinition: boolean) => void;
    /** Dark colors (from the theme in Settings). */
    dark?: boolean;
    /** The language server client (live errors), once it's running. */
    lsp?: LSPClient | null;
    /** The project folder, for the files' addresses in the language server. */
    projectDir?: string | null;
  } = $props();

  let host: HTMLElement;
  let view: EditorView | null = null;
  const docs = new Map<string, EditorState>();
  let shown: string | null = null;

  const themeSlot = new Compartment();
  /** The language server connection of each Verilog document. */
  const lspSlot = new Compartment();

  function lspExtension(file: string) {
    if (!lsp || !isVerilog(file)) return [];
    const language = /\.svh?$/i.test(file) ? "systemverilog" : "verilog";
    return lsp.plugin(fileUri(projectDir, file), language);
  }
  const verilogLanguage = StreamLanguage.define(verilog);

  function currentTheme() {
    return dark ? oneDark : [];
  }

  function isVerilog(file: string): boolean {
    return /\.(s?v|s?vh)$/i.test(file);
  }

  function makeState(file: string, text: string): EditorState {
    return EditorState.create({
      doc: text,
      extensions: [
        basicSetup,
        lintGutter(),
        isVerilog(file) ? verilogLanguage : [],
        indentUnit.of("  "),
        themeSlot.of(currentTheme()),
        lspSlot.of(lspExtension(file)),
        EditorView.theme({
          "&": { height: "100%" },
          ".cm-scroller": {
            fontFamily: "ui-monospace, Menlo, Consolas, monospace",
            // Settings > Editor font size (see app.css).
            fontSize: "var(--editor-font-size, 13px)",
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onChange(file, update.state.doc.toString());
        }),
      ],
    });
  }

  /** Keeps the shown document's latest state (edits, cursor, history). */
  function stashShown() {
    if (view && shown !== null && docs.has(shown)) docs.set(shown, view.state);
  }

  /** Replaces all documents (e.g. when opening a project) and shows one. */
  export function reset(
    files: { name: string; text: string }[],
    showFile: string,
  ) {
    docs.clear();
    shown = null;
    for (const f of files) docs.set(f.name, makeState(f.name, f.text));
    show(showFile);
  }

  /** Shows a file's document. */
  export function show(file: string) {
    if (!view) return;
    stashShown();
    const state = docs.get(file);
    if (!state) return;
    view.setState(state);
    // The document may have been created under the other color scheme.
    view.dispatch({ effects: themeSlot.reconfigure(currentTheme()) });
    shown = file;
  }

  export function add(file: string, text: string) {
    docs.set(file, makeState(file, text));
  }

  /** Renames a document. Its undo history is reset (the language may change). */
  export function rename(oldName: string, newName: string) {
    stashShown();
    const state = docs.get(oldName);
    if (!state) return;
    docs.delete(oldName);
    docs.set(newName, makeState(newName, state.doc.toString()));
    if (shown === oldName) {
      shown = null;
      show(newName);
    }
  }

  export function remove(file: string) {
    docs.delete(file);
    if (shown === file) shown = null;
  }

  /** Converts build diagnostics (1-based lines) to editor positions. */
  function toLint(state: EditorState, diags: Diagnostic[]): LintDiagnostic[] {
    const doc = state.doc;
    return diags
      .filter((d) => d.line !== null)
      .map((d) => {
        const line = doc.line(Math.min(Math.max(d.line!, 1), doc.lines));
        const from = d.column
          ? Math.min(line.from + d.column - 1, line.to)
          : line.from;
        return {
          from,
          to: Math.max(from, line.to),
          severity: d.severity,
          message: d.message,
        };
      });
  }

  /**
   * Shows build errors and warnings. `byFile` maps file names to their
   * diagnostics; files not in it are cleared.
   */
  export function setDiagnostics(byFile: Map<string, Diagnostic[]>) {
    stashShown();
    for (const [name, state] of docs) {
      const diags = toLint(state, byFile.get(name) ?? []);
      const next = state.update(setLintDiagnostics(state, diags)).state;
      docs.set(name, next);
    }
    if (view && shown !== null) {
      const state = docs.get(shown);
      if (state) view.setState(state);
    }
  }

  /**
   * Replaces a file's whole text as one edit (so Undo restores the old
   * text), keeping the cursor on the same line number where possible.
   */
  export function replaceText(file: string, text: string) {
    stashShown();
    const state = docs.get(file);
    if (!state || state.doc.toString() === text) return;
    const cursorLine = state.doc.lineAt(state.selection.main.head).number;
    const spec = (s: EditorState) => {
      const newLines = text.split("\n").length;
      const line = Math.min(cursorLine, newLines);
      // Position of the start of `line` in the new text.
      let pos = 0;
      for (let i = 1; i < line; i++) pos = text.indexOf("\n", pos) + 1;
      return {
        changes: { from: 0, to: s.doc.length, insert: text },
        selection: { anchor: pos },
      };
    };
    if (view && shown === file) {
      view.dispatch({ ...spec(view.state), scrollIntoView: true });
    } else {
      docs.set(file, state.update(spec(state)).state);
    }
  }

  /** Shows a file and puts the cursor at the start of a 1-based line. */
  export function reveal(file: string, line: number) {
    show(file);
    if (!view) return;
    const doc = view.state.doc;
    const pos = doc.line(Math.min(Math.max(line, 1), doc.lines)).from;
    view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: "center" }),
    });
    view.focus();
  }

  export function focus() {
    view?.focus();
  }

  /** Jumps to where the name at the cursor is defined (live checking). */
  export function goToDefinition() {
    if (!view) return;
    view.focus();
    jumpToDefinition(view);
  }

  /** Whether the editor has the keyboard focus. */
  export function hasFocus(): boolean {
    return view?.hasFocus ?? false;
  }

  /** Selects the whole open file. */
  export function selectAll() {
    if (!view) return;
    view.focus();
    cmSelectAll(view);
  }

  /** Comments or uncomments the selected lines (`//`). */
  export function toggleComment() {
    if (!view) return;
    view.focus();
    cmToggleComment(view);
  }

  function contextMenu(e: MouseEvent) {
    if (!view || !onContextMenu) return;
    e.preventDefault();
    // As in other editors: the menu acts where you clicked, unless that's
    // inside the selection (e.g. to cut or comment it).
    const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
    const inSelection = view.state.selection.ranges.some(
      (r) => !r.empty && pos !== null && pos >= r.from && pos <= r.to,
    );
    if (pos !== null && !inSelection) view.dispatch({ selection: { anchor: pos } });
    view.focus();
    onContextMenu(lsp !== null && shown !== null && isVerilog(shown));
  }

  // Connect the documents to the language server when it starts, or when
  // the project folder (part of the files' addresses) changes.
  $effect(() => {
    void lsp;
    void projectDir;
    stashShown();
    for (const [name, state] of docs) {
      docs.set(
        name,
        state.update({ effects: lspSlot.reconfigure(lspExtension(name)) }).state,
      );
    }
    const state = shown !== null ? docs.get(shown) : undefined;
    if (view && state) view.setState(state);
  });

  // Follow theme changes (other documents get it when shown).
  $effect(() => {
    const theme = currentTheme();
    view?.dispatch({ effects: themeSlot.reconfigure(theme) });
  });

  onMount(() => {
    view = new EditorView({ parent: host });
    return () => {
      view?.destroy();
      view = null;
    };
  });
</script>

<!-- The editor inside has its own roles; this only catches right-clicks. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="host" bind:this={host} oncontextmenu={contextMenu}></div>

<style>
  .host {
    height: 100%;
  }
</style>
