// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk, // 27MHz clock
    output       led      // An LED, off. Active low.
);

  // led stays off.
  assign led = 1'b1;

  // Your design goes here.

endmodule
