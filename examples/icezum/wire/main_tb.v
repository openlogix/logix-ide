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

  // The header pin, driven by the testbench.
  reg  gpio = 0;

  // The LED driven by the design.
  wire led;

  main dut (
      .gpio(gpio),
      .led (led)
  );

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Try both levels of the pin, one per clock.
    for (i = 0; i < 2; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      gpio = i[0];
      #1;
      `EXPECT_EQ(led, i[0])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
