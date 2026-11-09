// Bare minimum for the iCEBreaker: the board's pins, ready for your
// design. It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,         // 12 MHz clock.
  output led,            // LED, lit when 0.
  output [4:0] pmod_leds // 5 LEDs, lit when 1.
);

  // Everything off for now.
  assign led = 1'b1;
  assign pmod_leds = 5'b00000;

  // Your design goes here.

endmodule
