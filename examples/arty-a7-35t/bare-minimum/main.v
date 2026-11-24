// Bare minimum for the Arty A7-35T: a design that does nothing yet.
//
// The board's clock, LEDs and USB serial port are already declared and wired
// in pinout.xdc, so you can start adding your own logic right away. The LEDs
// are held off and the serial line idles until your logic uses them. Build
// and upload it as it is: nothing lights up, which shows the board takes a
// design.

module main (
  input sys_clk,        // 100 MHz clock
  output [3:0] leds,    // LEDs 0 to 3, lit when 1
  output uart_tx,       // Serial data to the computer, idles at 1
  input uart_rx         // Serial data from the computer, idles at 1
);

  // Hold the LEDs off: they light at 1.
  assign leds = 4'b0000;

  // The serial line idles at 1 when nothing is sent.
  assign uart_tx = 1'b1;

  // Your logic goes here.

endmodule
