// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk,    // 30MHz clock
    output [6:0] leds,       // LED matrix, off. Active high.
    output [2:0] leds_color  // Its colors: [0] red, [1] green, [2] blue. Active high.
);

  // leds stays off.
  assign leds = 7'b0000000;
  assign leds_color = 3'b000;

  // Your design goes here.

endmodule
