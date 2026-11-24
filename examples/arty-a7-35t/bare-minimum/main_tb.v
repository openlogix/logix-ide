// Testbench for the bare minimum design: checks that every output is at its
// resting level (the LEDs off, the serial line idle). Add your own checks as
// your design grows.

`timescale 1ns / 1ps

// Reports a failed check with the actual and expected values. Not fatal.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("%s is %0h, expected %0h", `"actual_val`", (actual_val), (expected_val));

module main_tb;

  // The design's ports.
  reg sys_clk = 1'b0;
  wire [3:0] leds;
  wire uart_tx;
  reg uart_rx = 1'b1;   // Idle

  main dut (
    .sys_clk(sys_clk),
    .leds(leds),
    .uart_tx(uart_tx),
    .uart_rx(uart_rx)
  );

  // The board's 100 MHz clock: a period of 10 ns.
  always #5.000 sys_clk = !sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    // Let the design run for 32 clock cycles.
    repeat (32) @(posedge sys_clk);
    #1;

    // The LEDs are off, at 0.
    `EXPECT_EQ(leds, 4'b0000)

    // The serial line is idle, at 1.
    `EXPECT_EQ(uart_tx, 1'b1)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
