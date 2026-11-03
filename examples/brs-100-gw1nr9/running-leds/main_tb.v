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

  // Clocks between steps, reduced to run quickly.
  localparam integer DIV = 3;

  // The LEDs driven by the design.
  wire [5:0] leds_n;

  main #(
      .DIV(DIV)
  ) dut (
      .sys_clk(sys_clk),
      .leds_n (leds_n)
  );

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // The reset is high for the first 3 clocks. After clock i >= 3 the
    // lit LED has moved (i - 3) / DIV steps from leds[0], wrapping after
    // leds[5].
    for (i = 1; i <= 30; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(dut.sys_rst, i < 3)
      if (i >= 3) `EXPECT_EQ(leds_n, ~(6'b00_0001 << (((i - 3) /DIV) % 6)))
    end

    $display("End of simulation");
    $finish;
  end

endmodule
