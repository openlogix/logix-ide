// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // The design's outputs.
  wire       led;

  // The design.
  main dut (
      .led(led)
  );

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // At the start, the LEDs are off and the held pins at their levels.
    #1;
    `EXPECT_EQ(led, 1'b1)

    // Add your own checks here.
    #100;

    $display("End of simulation");
    $finish;
  end

endmodule
