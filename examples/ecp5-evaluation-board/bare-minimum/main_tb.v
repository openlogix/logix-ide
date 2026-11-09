// Testbench for the ECP5-Evaluation-Board's bare minimum: checks that the
// LED is off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  wire led;

  main dut (.led(led));

  initial begin
    $dumpvars(0, main_tb);

    #100;
    `EXPECT_EQ(led, 1'b1)  // Off (lit when 0).

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
