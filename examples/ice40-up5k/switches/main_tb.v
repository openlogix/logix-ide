// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock. The design has no clock, it only paces the checks.
  reg sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The switches, driven by the testbench.
  reg  [2:0] sws = 3'b000;

  // The RGB LED driven by the design.
  wire [2:0] rgb_led_n;

  main dut (
      .sws(sws),
      .rgb_led_n(rgb_led_n)
  );

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Try every combination of the switches, one per clock.
    for (i = 0; i < 8; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      sws = i[2:0];
      #1;
      `EXPECT_EQ(rgb_led_n, {i[0], i[1], i[2]})
    end

    $display("End of simulation");
    $finish;
  end

endmodule
