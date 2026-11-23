// Serial port demo for the Icesugar Pro, using uart_tx and uart_rx (uart.v).
//
// What it does:
//   - Every character you type is sent back to you (an "echo").
//   - The LED shows the lowest bit of the code of the last character you
//     typed: lit for an odd code, such as "A" (65), off for an even one,
//     such as "B" (66).
//   - When you haven't typed anything for 5 seconds, the board sends
//     "Hello from your FPGA board", then again every 5 seconds. Each
//     character you type starts the 5 seconds over, so the greeting waits
//     while you type. After you've typed, the greeting starts on a new line.
//
// To try it: upload it, then open Tools > Serial Monitor (115200 baud).

module main #(
  parameter BAUD = 115_200,         // Serial speed, in bits per second.
  parameter QUIET_CYCLES = 125_000_000 // Clock cycles of quiet before a greeting: 5 s at 25 MHz.
) (
  input sys_clk,      // 25 MHz clock.
  output uart_tx,     // Serial line to the computer.
  input uart_rx,      // Serial line from the computer.
  output led          // LED, lit when 0.
);

  // ---------------------------------------------------------------------
  // The serial port: a transmitter and a receiver
  // ---------------------------------------------------------------------

  wire [7:0] tx_data;     // The character to send.
  wire tx_valid;          // 1: there's a character to send.
  wire tx_ready;          // 1: the transmitter can take a character.
  wire [7:0] rx_data;     // The last character received.
  wire rx_valid;          // 1 for one cycle when a character arrives.

  uart_tx #(.CLK_HZ(25_000_000), .BAUD(BAUD)) sender (
    .clk(sys_clk),
    .data(tx_data), .valid(tx_valid), .ready(tx_ready), .tx(uart_tx)
  );

  uart_rx #(.CLK_HZ(25_000_000), .BAUD(BAUD)) receiver (
    .clk(sys_clk),
    .rx(uart_rx), .data(rx_data), .valid(rx_valid), .error()
  );

  // A character is handed to the transmitter in a cycle where we have one
  // (tx_valid) and it can take it (tx_ready).
  wire char_sent = tx_valid && tx_ready;

  // ---------------------------------------------------------------------
  // The LED: the last character received
  // ---------------------------------------------------------------------

  reg [7:0] last_char = 8'd0;
  always @(posedge sys_clk) begin
    if (rx_valid) last_char <= rx_data;
  end
  assign led = ~last_char[0];  // Its lowest bit, inverted: the LED is lit by a 0.

  // ---------------------------------------------------------------------
  // The echo
  // ---------------------------------------------------------------------

  // A received character waits here until the transmitter can send it back.
  // If you type faster than it's sent, the newer character replaces the
  // waiting one.
  reg echo_waiting = 1'b0;
  reg [7:0] echo_char = 8'd0;

  // ---------------------------------------------------------------------
  // The greeting
  // ---------------------------------------------------------------------

  // The text, with a carriage return and a line feed (written in octal as
  // \015 and \012) before and after it. The ones before are sent only after
  // you've typed, to end the line of echoed characters first; the ones after
  // end the greeting's own line. The first character is in the highest 8
  // bits.
  localparam LENGTH = 30;
  localparam [8*LENGTH-1:0] GREETING = "\015\012Hello from your FPGA board\015\012";
  localparam [4:0] FIRST = 5'd0;   // Where the greeting starts after typing: the new line.
  localparam [4:0] HELLO = 5'd2;   // Where it starts otherwise: "Hello".
  localparam [4:0] LAST = LENGTH - 1;

  // Counts the quiet time. Every character received starts it over.
  // 27 bits count up to 134 million, enough for 5 seconds at 25 MHz.
  reg [26:0] quiet_cycles = 27'd0;

  reg sending_greeting = 1'b0;   // 1 while the greeting is being sent.
  reg [4:0] char_number = 5'd0;  // Which character of the greeting is next (0 first).
  reg typed = 1'b0;              // 1 if a character was received since the last greeting.

  // The greeting's next character: 8 bits, counted from the top.
  wire [7:0] greeting_char = GREETING[8*(LENGTH-1-char_number) +: 8];

  // ---------------------------------------------------------------------
  // What to send next: an echo first, else the greeting's next character
  // ---------------------------------------------------------------------

  assign tx_valid = echo_waiting || sending_greeting;
  assign tx_data = echo_waiting ? echo_char : greeting_char;

  always @(posedge sys_clk) begin
    // The echo: keep each received character until it's sent.
    if (rx_valid) begin
      echo_waiting <= 1'b1;
      echo_char <= rx_data;
    end else if (char_sent && echo_waiting) begin
      echo_waiting <= 1'b0;
    end

    // The quiet time: start over on each character received; when it
    // reaches QUIET_CYCLES, start a greeting (unless one is being sent).
    if (rx_valid) begin
      quiet_cycles <= 27'd0;
      typed <= 1'b1;
    end else if (quiet_cycles == QUIET_CYCLES - 1) begin
      quiet_cycles <= 27'd0;
      if (!sending_greeting) begin
        // Start a greeting: on a new line if something was typed since the
        // last one.
        sending_greeting <= 1'b1;
        char_number <= typed ? FIRST : HELLO;
        typed <= 1'b0;
      end
    end else begin
      quiet_cycles <= quiet_cycles + 27'd1;
    end

    // The greeting: move to the next character each time one is sent. After
    // the last, stop, and go back to the start, so char_number never points
    // past the end of the text (greeting_char would read nothing there).
    if (char_sent && !echo_waiting) begin
      if (char_number == LAST) begin
        sending_greeting <= 1'b0;
        char_number <= FIRST;
      end else begin
        char_number <= char_number + 5'd1;
      end
    end
  end

endmodule
