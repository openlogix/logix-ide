// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk,   // 12MHz clock
    output       led,       // An LED, off. Active low.
    output [4:0] pmod_leds  // Off. Active high.
);

  // led stays off.
  assign led = 1'b1;

  // pmod_leds stays off.
  assign pmod_leds = 5'b00000;

  // Your design goes here.

endmodule
