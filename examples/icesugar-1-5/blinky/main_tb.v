// Testbench for blinky: with a 4-bit counter, the LED's green is off at
// first, on after 8 clock cycles and off again after 16.

`timescale 1ns / 1ps

// Reports a failed check with the actual and expected values. Not fatal.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("%s is %0h, expected %0h", `"actual_val`", (actual_val), (expected_val));

module main_tb;

  // The design's ports.
  reg sys_clk = 1'b0;
  wire [2:0] rgb_led;

  // A 4-bit counter, so the LED changes every 8 clock cycles.
  main #(.N(4)) dut (
    .sys_clk(sys_clk),
    .rgb_led(rgb_led)
  );

  // The board's 12 MHz clock: a period of 83.333 ns.
  always #41.667 sys_clk = !sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    // Green is off at first (the LED lights at 0).
    #1;
    `EXPECT_EQ(rgb_led[1], 1'b1)

    // Red and blue are held off.
    `EXPECT_EQ(rgb_led[0], 1'b1)
    `EXPECT_EQ(rgb_led[2], 1'b1)

    // Green is on after 8 clock cycles.
    repeat (8) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(rgb_led[1], 1'b0)

    // Off again after 16.
    repeat (8) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(rgb_led[1], 1'b1)

    // Add your own checks here.

    // Run on to 32 clock cycles.
    repeat (16) @(posedge sys_clk);

    $display("End of simulation");
    $finish;
  end

endmodule
