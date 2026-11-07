/**
 * Lines a tool prints over (the engine's `redraw` flag, such as each state
 * of a progress bar): the console shows each at once and replaces it with
 * the same tool's next line, as a terminal does. Any other line in between
 * ends the redraw, so the line stays. The log file gets only the lines
 * that stay.
 */
export class Redraws {
  /** The console's last line, when it's a redraw: its tool and text. */
  private last: { tool: string; text: string } | null = null;

  /**
   * A tool's line. `replace`: it replaces the console's last line.
   * `final`: the lines that now stay, for the log file.
   */
  toolLine(
    text: string,
    tool: string,
    redraw: boolean,
  ): { replace: boolean; final: string[] } {
    const replace = this.last !== null && this.last.tool === tool;
    const final = this.last && !replace ? [this.last.text] : [];
    this.last = redraw ? { tool, text } : null;
    if (!redraw) final.push(text);
    return { replace, final };
  }

  /** Any other line, or the console cleared: a redraw shown stays. */
  end(): string[] {
    const final = this.last ? [this.last.text] : [];
    this.last = null;
    return final;
  }
}
