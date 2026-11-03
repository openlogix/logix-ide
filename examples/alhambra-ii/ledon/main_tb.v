// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock. The design has no clock, it only times the checks.
  reg sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The LEDs driven by the design.
  wire [7:0] leds;

  // The design under test.
  main dut (.leds(leds));

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // leds[0] is on and the other LEDs are off, all the time.
    repeat (8) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(leds, 8'b0000_0001)
    end

    $display("End of simulation");
    $finish;
  end

endmodule
