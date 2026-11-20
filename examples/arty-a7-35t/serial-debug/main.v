// Serial debug demo for the Arty A7-35T (Xilinx, Artix7), using debug.v and
// uart.v.
//
// What it does:
//   - A counter goes up every half second, and the 4 LEDs show its lowest 4
//     bits in binary.
//   - Each time it changes, the board prints its value over the USB serial
//     port, such as "count: 0x002A", so you can watch it in the Serial
//     Monitor. Each line overwrites the last, so the value updates in
//     place (debug's OVERWRITE; set it to 0 to keep every line).
//   - Type a value in hexadecimal and press Enter, such as "4d", and the
//     counter is set to it (and printed at once), then counts on from there.
//
// That's how to see inside your own design on the real board, and control
// it: connect a debug module to the value you want to watch and set.
//
// To try it: upload it, then open Tools > Serial Monitor (115200 baud).
//
// Which port: the board's USB chip (an FTDI) has two channels, so computers
// list two serial ports for it. Interface 0 programs the board; use the one
// for interface 1, usually the second of the two (on Linux usually
// /dev/ttyUSB1). If you see nothing, try the other.

module main #(
  parameter BAUD = 115_200,          // Serial speed, in bits per second.
  parameter TICK_CYCLES = 50_000_000 // Clock cycles between counts: 0.5 s at 100 MHz.
) (
  input sys_clk,      // 100 MHz clock.
  output uart_tx,     // Serial line to the computer.
  input uart_rx,      // Serial line from the computer.
  output [3:0] leds   // 4 LEDs, lit when 1.
);

  // ---------------------------------------------------------------------
  // The design being watched: a counter
  // ---------------------------------------------------------------------

  // tick is 1 for one clock cycle every TICK_CYCLES cycles.
  // 26 bits count up to 67 million, enough for 0.5 s at 100 MHz.
  reg [25:0] tick_cycles = 26'd0;
  wire tick = (tick_cycles == TICK_CYCLES - 1);
  always @(posedge sys_clk) begin
    tick_cycles <= tick ? 26'd0 : tick_cycles + 26'd1;
  end

  // The counter: one more at each tick, or the value typed (below).
  reg [15:0] count = 16'd0;
  wire [15:0] new_value;
  wire got_value;
  always @(posedge sys_clk) begin
    if (got_value) count <= new_value;
    else if (tick) count <= count + 16'd1;
  end

  // The LEDs: the counter's lowest 4 bits.
  assign leds = count[3:0];

  // ---------------------------------------------------------------------
  // The debugger: shows the counter, and sets it to a value typed
  // ---------------------------------------------------------------------

  // Printed one cycle after a change, when count has its new value.
  reg print = 1'b0;
  always @(posedge sys_clk) print <= tick || got_value;

  debug #(
    .CLK_HZ(100_000_000), .BAUD(BAUD),
    .LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4), .OVERWRITE(1)
  ) dbg (
    .clk(sys_clk),
    .uart_tx(uart_tx), .uart_rx(uart_rx),
    .print(print), .value(count),
    .typed_value(new_value), .typed(got_value)
  );

endmodule
