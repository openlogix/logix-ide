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

  // The LEDs driven by the design.
  wire [1:0] leds_n;

  // A 4 bit counter, so leds_n[1] toggles every 8 clocks.
  main #(
      .N(4)
  ) dut (
      .sys_clk(sys_clk),
      .leds_n (leds_n)
  );

  // Loop index for the LED checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Before the first clock, the counter starts at 0.
    #1;
    `EXPECT_EQ(dut.counter, 4'd0)
    `EXPECT_EQ(leds_n, 2'b11)

    // After i clocks the counter equals i, so leds_n[1] shows the inverse of bit 3
    // of i. The other LEDs stay off.
    for (i = 1; i <= 32; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(leds_n[1], ~i[3])
      `EXPECT_EQ(leds_n[0], 1'b1)
    end

    $display("End of simulation");
    $finish;
  end

endmodule
