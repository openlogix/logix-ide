// Testbench for gamma.v: for every input, gamma_out is
// round(4095 * (gamma_in / 255) ^ 2.5),
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

  // The gamma function's input, and its output one clock later.
  reg  [ 7:0] gamma_in = 0;
  wire [11:0] gamma_out;

  gamma dut (
      .clk  (clk),
      .index(gamma_in),
      .value(gamma_out)
  );

  // Loop index and the expected value.
  integer i = 0;
  integer expected = 0;

  initial begin
    $dumpvars(0, gamma_tb);

    for (i = 0; i < 256; i = i + 1) begin
      gamma_in = i;
      // The output is ready one clock after the input.
      @(posedge clk);
      #1;
      expected = $rtoi(4095.0 * $pow(i / 255.0, 2.5) + 0.5);
      `EXPECT_EQ(gamma_out, expected[11:0])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
