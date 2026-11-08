// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock.
  reg sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The display pins driven by the design.
  wire [6:0] segs_n;
  wire digit_sel;

  // A 4 bit counter, so the segment toggles every 8 clocks.
  main #(
      .N(4)
  ) dut (
      .sys_clk(sys_clk),
      .segs_n(segs_n),
      .digit_sel(digit_sel)
  );

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Before the first clock, the counter starts at 0, all the segments
    // are off and the right-hand digit is selected.
    #1;
    `EXPECT_EQ(dut.counter, 4'd0)
    `EXPECT_EQ(segs_n, 7'b111_1111)
    `EXPECT_EQ(digit_sel, 1'b1)

    // After i clocks the counter equals i, so segment A shows bit 3 of i
    // (low is on). The others stay off.
    for (i = 1; i <= 32; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(segs_n, {6'b11_1111, ~i[3]})
    end

    $display("End of simulation");
    $finish;
  end

endmodule
