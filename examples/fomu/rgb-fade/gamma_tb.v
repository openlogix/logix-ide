// Testbench for gamma.v: every entry is round(4095 * (index / 255) ^ 2.5),
// computed here independently.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module gamma_tb;

  // Clock.
  reg clk = 0;
  always #5 clk = ~clk;

  reg  [ 7:0] index = 0;
  wire [11:0] value;

  gamma dut (
      .clk  (clk),
      .index(index),
      .value(value)
  );

  // Loop index and the expected value.
  integer i = 0;
  integer expected = 0;

  initial begin
    $dumpvars(0, gamma_tb);

    for (i = 0; i < 256; i = i + 1) begin
      index = i;
      // The value is ready one clock after the index.
      @(posedge clk);
      #1;
      expected = $rtoi(4095.0 * $pow(i / 255.0, 2.5) + 0.5);
      `EXPECT_EQ(value, expected[11:0])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
