// Debug: watch and set a value inside your design on the real board, over
// the board's serial port, with a terminal such as Logix IDE's Serial
// Monitor (Tools > Serial Monitor). Copy this file and uart.v, with their
// testbenches debug_tb.v and uart_tb.v, into your project.
//
// Add one debug module to your design and connect it to the serial pins:
//   - Watch: pulse print, and it sends value as a line of text, such as
//     "count: 0x002A", the FPGA version of a print statement.
//   - Set: type a value in hexadecimal and press Enter, such as "4d", and
//     your design gets it on typed_value (0x4D), with typed at 1 for one
//     clock cycle.
//
// What you type to set a value:
//   - hexadecimal digits: 0 to 9, a to f or A to F, such as "4d";
//   - then Enter, to send the value to the design;
//   - any other character drops what was typed, so a mistake doesn't
//     half-apply; type the value again;
//   - more digits than DIGITS: the last DIGITS count, as on a calculator.
//
// How to use it:
//
//   debug #(.CLK_HZ(12_000_000), .LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4)) dbg (
//     .clk(clk),
//     .uart_tx(uart_tx), .uart_rx(uart_rx),    // The serial port's pins.
//     .print(tick),                            // 1 for one cycle: send a line now.
//     .value(count),                           // The value to show (DIGITS * 4 bits).
//     .typed_value(new_value),                 // The last value typed (DIGITS * 4 bits).
//     .typed(got_value)                        // 1 for one cycle: a value was typed.
//   );
//
// Leave out what you don't need: tie print to 0 to only set values, or
// leave typed unconnected to only watch.
//
// One debug module per serial port, since the port has one line each way.
// To watch several values, show them together as one wider value, such as
// .value({a, b}) with DIGITS 4 for two 8-bit values ("0xAABB").
//
// A print while a line is still being sent is ignored: print less often
// than a line takes to send (at 115200 baud, about 1 ms for a short line).
// The value is taken when print is 1, so the line shows that moment's
// value even if it changes while the line is sent.

module debug #(
  parameter CLK_HZ = 12_000_000, // Your clock's frequency, in Hz.
  parameter BAUD = 115_200,      // Serial speed, in bits per second.
  parameter LABEL = "value",     // The label, a short text.
  parameter LABEL_LENGTH = 5,    // How many characters LABEL has.
  parameter DIGITS = 4,          // Hexadecimal digits: the values have DIGITS * 4 bits.
  parameter OVERWRITE = 0        // 1: each line overwrites the last (no line feed).
) (
  input clk,                               // Your clock.
  output uart_tx,                          // The serial line to the computer.
  input uart_rx,                           // The serial line from the computer.
  input print,                             // 1 for one cycle: send a line with value.
  input [DIGITS*4-1:0] value,              // The value to show.
  output [DIGITS*4-1:0] typed_value,       // The last value typed.
  output typed                             // 1 for one cycle when a value is typed.
);

  // ---- Watch: value, as a line of text, to the computer ----

  wire [7:0] tx_data;
  wire tx_valid;
  wire tx_ready;

  debug_print #(
    .LABEL(LABEL), .LABEL_LENGTH(LABEL_LENGTH), .DIGITS(DIGITS), .OVERWRITE(OVERWRITE)
  ) printer (
    .clk(clk), .print(print), .value(value), .busy(),
    .tx_data(tx_data), .tx_valid(tx_valid), .tx_ready(tx_ready)
  );

  uart_tx #(.CLK_HZ(CLK_HZ), .BAUD(BAUD)) sender (
    .clk(clk), .data(tx_data), .valid(tx_valid), .ready(tx_ready), .tx(uart_tx)
  );

  // ---- Set: a value typed on the computer, into the design ----

  wire [7:0] rx_data;
  wire rx_valid;

  uart_rx #(.CLK_HZ(CLK_HZ), .BAUD(BAUD)) receiver (
    .clk(clk), .rx(uart_rx), .data(rx_data), .valid(rx_valid), .error()
  );

  debug_input #(.DIGITS(DIGITS)) reader (
    .clk(clk), .rx_data(rx_data), .rx_valid(rx_valid),
    .value(typed_value), .valid(typed)
  );

endmodule

// ---------------------------------------------------------------------------
// debug_print: the watch part of debug, above. It sends a value as a line
// of text through uart_tx (in uart.v): the label, a colon and a space, the
// value in hexadecimal (0x and DIGITS digits), then a carriage return and a
// line feed, such as "count: 0x002A". With OVERWRITE at 1, it ends with a
// carriage return only: the Serial Monitor (like any terminal) then goes
// back to the line's start, so each line overwrites the last and the value
// updates in place.

module debug_print #(
  parameter LABEL = "count",  // The label, a short text.
  parameter LABEL_LENGTH = 5, // How many characters LABEL has.
  parameter DIGITS = 4,       // Hexadecimal digits shown: the value has DIGITS * 4 bits.
  parameter OVERWRITE = 0     // 1: each line overwrites the last (no line feed).
) (
  input clk,                     // Your clock.
  input print,                   // 1 for one cycle: send a line with value.
  input [DIGITS*4-1:0] value,    // The value to show.
  output busy,                   // 1 while a line is being sent.

  // To uart_tx: the characters of the line, one at a time.
  output [7:0] tx_data,
  output tx_valid,
  input tx_ready
);

  // The line's length: the label, ": 0x", the digits, then "\r\n" (or "\r"
  // alone, with OVERWRITE).
  localparam LENGTH = LABEL_LENGTH + 4 + DIGITS + (OVERWRITE ? 1 : 2);

  // The value as it was when print came.
  reg [DIGITS*4-1:0] shown = 0;

  // Which character of the line is sent next, and whether a line is being
  // sent at all.
  reg sending = 1'b0;
  reg [7:0] position = 8'd0;

  assign busy = sending;
  assign tx_valid = sending;

  // A hexadecimal digit (0 to 15) as its character: "0" to "9" (codes 0x30
  // to 0x39), then "A" to "F" (codes 0x41 to 0x46, so 0x37 plus 10 to 15).
  function [7:0] hex_char(input [3:0] digit);
    hex_char = (digit < 4'd10) ? 8'h30 + {4'd0, digit} : 8'h37 + {4'd0, digit};
  endfunction

  // The character at each position of the line, for the value shown.
  function [7:0] line_char(input [7:0] pos, input [DIGITS*4-1:0] number);
    if (pos < LABEL_LENGTH)
      line_char = LABEL[8 * (LABEL_LENGTH - 1 - pos) +: 8];  // The label, first character first.
    else if (pos == LABEL_LENGTH)
      line_char = ":";
    else if (pos == LABEL_LENGTH + 1)
      line_char = " ";
    else if (pos == LABEL_LENGTH + 2)
      line_char = "0";
    else if (pos == LABEL_LENGTH + 3)
      line_char = "x";
    else if (pos < LABEL_LENGTH + 4 + DIGITS)
      // A digit, the leftmost first.
      line_char = hex_char(number[(DIGITS - 1 - (pos - LABEL_LENGTH - 4)) * 4 +: 4]);
    else if (pos == LABEL_LENGTH + 4 + DIGITS)
      line_char = 8'h0D;  // Carriage return.
    else
      line_char = 8'h0A;  // Line feed (not with OVERWRITE).
  endfunction

  assign tx_data = line_char(position, shown);

  always @(posedge clk) begin
    if (!sending) begin
      // Waiting: start a line when asked, with the value of that moment.
      if (print) begin
        shown <= value;
        position <= 8'd0;
        sending <= 1'b1;
      end
    end else if (tx_ready) begin
      // uart_tx takes this character now: on to the next, or done.
      if (position == LENGTH - 1) sending <= 1'b0;
      position <= position + 8'd1;
    end
  end

endmodule

// ---------------------------------------------------------------------------
// debug_input: the set part of debug, above. It reads the characters
// uart_rx (in uart.v) receives, collects hexadecimal digits, and when a line
// ends (Enter: a carriage return or a line feed) it gives the value with
// valid at 1 for one clock cycle.

module debug_input #(
  parameter DIGITS = 2  // Hexadecimal digits taken: the value has DIGITS * 4 bits.
) (
  input clk,                          // Your clock.

  // From uart_rx: each character received.
  input [7:0] rx_data,
  input rx_valid,

  output reg [DIGITS*4-1:0] value = 0, // The last value typed.
  output reg valid = 1'b0              // 1 for one cycle when a value arrives.
);

  // The digits typed so far on this line, and whether there's at least one.
  reg [DIGITS*4-1:0] typed = 0;
  reg any = 1'b0;

  // The character as a hexadecimal digit, and whether it is one.
  wire is_number = (rx_data >= "0") && (rx_data <= "9");
  wire is_upper = (rx_data >= "A") && (rx_data <= "F");
  wire is_lower = (rx_data >= "a") && (rx_data <= "f");
  wire is_digit = is_number || is_upper || is_lower;
  wire [7:0] digit_code = is_number ? rx_data - "0"
                        : is_upper ? rx_data - "A" + 8'd10
                        : rx_data - "a" + 8'd10;
  wire [3:0] digit = digit_code[3:0];

  // Enter: a carriage return or a line feed.
  wire is_enter = (rx_data == 8'h0D) || (rx_data == 8'h0A);

  always @(posedge clk) begin
    valid <= 1'b0;  // 1 for one cycle only.
    if (rx_valid) begin
      if (is_digit) begin
        // One more digit: the others move left, as on a calculator.
        typed <= {typed[DIGITS*4-5:0], digit};
        any <= 1'b1;
      end else if (is_enter) begin
        // The line is done: give the value, if a digit was typed (a
        // carriage return and a line feed together give it only once).
        if (any) begin
          value <= typed;
          valid <= 1'b1;
        end
        typed <= 0;
        any <= 1'b0;
      end else begin
        // Not a digit: drop what was typed.
        typed <= 0;
        any <= 1'b0;
      end
    end
  end

endmodule
