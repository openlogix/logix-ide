// Testbench for the ODT_IcyBlue_Feather's blinky. With a 4-bit counter, the
// LED is off at first, lit after 8 clock cycles and off again after 16.
// Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  wire led;

  // The design, with a 4-bit counter so the LED changes every 8 cycles.
  main #(.N(4)) dut (.led(led));

  // Waits for n rising edges of the design's clock (the internal oscillator).
  task cycles(input integer n);
    repeat (n) @(posedge dut.clk);
  endtask

  initial begin
    $dumpvars(0, main_tb);

    #1 `EXPECT_EQ(led, 1'b0)  // Off (lit when 1).
    cycles(8);
    #1 `EXPECT_EQ(led, 1'b1)  // Lit.
    cycles(8);
    #1 `EXPECT_EQ(led, 1'b0)  // Off again.
    cycles(16);

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
