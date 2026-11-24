// Bare minimum for the Sipeed Tang Nano 9k: a design that does nothing yet.
//
// The board's clock, LED and USB serial port are already declared and wired
// in pinout.cst, so you can start adding your own logic right away. The LED
// is held off and the serial line idles until your logic uses them. Build
// and upload it as it is: nothing lights up, which shows the board takes a
// design.

module main (
  input sys_clk,    // 27 MHz clock
  output led,       // LED, lit when 0
  output uart_tx,   // Serial data to the computer, idles at 1
  input uart_rx     // Serial data from the computer, idles at 1
);

  // Hold the LED off: it lights at 0.
  assign led = 1'b1;

  // The serial line idles at 1 when nothing is sent.
  assign uart_tx = 1'b1;

  // Your logic goes here.

endmodule
