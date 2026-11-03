// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock. The design makes its own clock with the oscillator model in
  // osc.v, which has a 10ns period.
  wire sys_clk = dut.sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The LED driven by the design.
  wire led;

  // A 4 bit counter, so the LED toggles every 8 clocks.
  main #(.N(4)) dut (.led(led));

  // Loop index for the LED checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Before the first clock, the counter starts at 0.
    #1;
    `EXPECT_EQ(dut.counter, 4'd0)
    `EXPECT_EQ(led, 1'b0)

    // After i clocks the counter equals i, so the LED shows bit 3
    // of i.
    for (i = 1; i <= 32; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(led, i[3])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
