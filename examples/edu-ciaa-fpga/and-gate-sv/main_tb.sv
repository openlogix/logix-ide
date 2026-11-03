// Testbench for main.sv.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock. The design has no clock, it only paces the checks.
  logic sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  int clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The buttons, driven by the testbench.
  logic [1:0] btns = 2'b00;

  // The LED driven by the design.
  logic led;

  main dut (
      .btns(btns),
      .led (led)
  );

  // Loop index for the checks.
  int i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Try every combination of the buttons, one per clock.
    for (i = 0; i < 4; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      btns = i[1:0];
      #1;
      `EXPECT_EQ(led, i[0] & i[1])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
