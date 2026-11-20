// Serial debug demo for the Sipeed Tang Nano 9k, using debug_print and debug_input
// (debug.v), with uart_tx and uart_rx (uart.v).
//
// What it does:
//   - A counter goes up every half second, and the LED shows its lowest bit:
//     it blinks once a second.
//   - Each time it changes, the board prints its value over the USB serial
//     port, such as "count: 0x002A", so you can watch it in the Serial
//     Monitor. Each line overwrites the last, so the value updates in
//     place (debug_print's OVERWRITE; set it to 0 to keep every line).
//   - Type a value in hexadecimal and press Enter, such as "4d", and the
//     counter is set to it (and printed at once), then counts on from there.
//
// That's how to see inside your own design on the real board, and control
// it: add a debug_print for each value you want to watch, and a
// debug_input for each value you want to set.
//
// To try it: upload it, then open Tools > Serial Monitor (115200 baud).

module main #(
  parameter BAUD = 115_200,          // Serial speed, in bits per second.
  parameter TICK_CYCLES = 13_500_000 // Clock cycles between counts: 0.5 s at 27 MHz.
) (
  input sys_clk,      // 27 MHz clock.
  output uart_tx,     // Serial line to the computer.
  input uart_rx,      // Serial line from the computer.
  output led          // LED, lit when 0.
);

  // ---------------------------------------------------------------------
  // The debug input: a value typed in the Serial Monitor sets the counter
  // ---------------------------------------------------------------------

  wire [7:0] rx_data;
  wire rx_valid;
  wire [15:0] new_value;
  wire got_value;

  uart_rx #(.CLK_HZ(27_000_000), .BAUD(BAUD)) receiver (
    .clk(sys_clk),
    .rx(uart_rx), .data(rx_data), .valid(rx_valid), .error()
  );

  debug_input #(.DIGITS(4)) set_count (
    .clk(sys_clk),
    .rx_data(rx_data), .rx_valid(rx_valid),
    .value(new_value), .valid(got_value)
  );

  // ---------------------------------------------------------------------
  // The design being watched: a counter
  // ---------------------------------------------------------------------

  // tick is 1 for one clock cycle every TICK_CYCLES cycles.
  // 24 bits count up to 16 million, enough for 0.5 s at 27 MHz.
  reg [23:0] tick_cycles = 24'd0;
  wire tick = (tick_cycles == TICK_CYCLES - 1);
  always @(posedge sys_clk) begin
    tick_cycles <= tick ? 24'd0 : tick_cycles + 24'd1;
  end

  // The counter: one more at each tick, or the value typed (above).
  reg [15:0] count = 16'd0;
  always @(posedge sys_clk) begin
    if (got_value) count <= new_value;
    else if (tick) count <= count + 16'd1;
  end

  // The LED: the counter's lowest bit, inverted: the LED is lit by a 0.
  assign led = ~count[0];

  // ---------------------------------------------------------------------
  // The debug print: the counter's value, each time it changes
  // ---------------------------------------------------------------------

  // Printed one cycle after a change, when count has its new value.
  reg print = 1'b0;
  always @(posedge sys_clk) print <= tick || got_value;

  wire [7:0] tx_data;
  wire tx_valid;
  wire tx_ready;

  debug_print #(.LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4), .OVERWRITE(1)) show_count (
    .clk(sys_clk),
    .print(print),
    .value(count),
    .busy(),
    .tx_data(tx_data), .tx_valid(tx_valid), .tx_ready(tx_ready)
  );

  uart_tx #(.CLK_HZ(27_000_000), .BAUD(BAUD)) sender (
    .clk(sys_clk),
    .data(tx_data), .valid(tx_valid), .ready(tx_ready), .tx(uart_tx)
  );

endmodule
