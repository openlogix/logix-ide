// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 12MHz clock
    output [2:0] rgb_led  // Off. Active low.
);

  // rgb_led stays off.
  rgb_driver rgb_driver (
      .on  (3'b000),
      .pins(rgb_led)
  );

  // Your design goes here.

endmodule
