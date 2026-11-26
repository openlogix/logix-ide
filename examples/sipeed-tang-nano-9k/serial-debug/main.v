// Serial debug demo for the Sipeed Tang Nano 9k, using debug.v and uart.v.
//
// What it does:
//   - A counter goes up every half second, and the LED shows its lowest
//     bit: it blinks once a second.
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
// To try it: upload it, then open Tools > Serial Monitor (9600 baud: this
// board's USB chip runs its serial port at 9600 on macOS, whatever speed
// the computer asks for, so 9600 works everywhere).
//
// Which port: the board's USB chip (a BL702) has two channels, so computers
// list two serial ports for it. Interface 0 programs the board; use the one
// for interface 1, usually the second of the two (on Linux usually
// /dev/ttyUSB1). If you see nothing, try the other.

module main #(
  parameter BAUD = 9_600,            // Serial speed, in bits per second.
  parameter TICK_CYCLES = 13_500_000 // Clock cycles between counts: 0.5 s at 27 MHz.
) (
  input sys_clk,      // 27 MHz clock.
  output uart_tx,     // Serial line to the computer.
  input uart_rx,      // Serial line from the computer.
  output led          // LED, lit when 0.
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

  // The counter: one more at each tick, or the value typed (below).
  reg [15:0] count = 16'd0;
  wire [15:0] new_value;
  wire got_value;
  always @(posedge sys_clk) begin
    if (got_value) count <= new_value;
    else if (tick) count <= count + 16'd1;
  end

  // The LED: the counter's lowest bit, inverted: the LED is lit by a 0.
  assign led = ~count[0];

  // ---------------------------------------------------------------------
  // The debugger: shows the counter, and sets it to a value typed
  // ---------------------------------------------------------------------

  // Printed one cycle after a change, when count has its new value.
  reg print = 1'b0;
  always @(posedge sys_clk) print <= tick || got_value;

  debug #(
    .CLK_HZ(27_000_000), .BAUD(BAUD),
    .LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4), .OVERWRITE(1)
  ) dbg (
    .clk(sys_clk),
    .uart_tx(uart_tx), .uart_rx(uart_rx),
    .print(print), .value(count),
    .typed_value(new_value), .typed(got_value)
  );

endmodule
