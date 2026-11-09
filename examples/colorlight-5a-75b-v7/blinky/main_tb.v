// Testbench for the ColorLight-5A-75B-V7's blinky. With a 4-bit counter,
// the LED is off at first, lit after 8 clock cycles and off again after 16.
// Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire led;

  // The design, with a 4-bit counter so the LED changes every 8 cycles.
  main #(.N(4)) dut (.sys_clk(sys_clk), .led(led));

  // The board's 25 MHz clock.
  always #20 sys_clk = ~sys_clk;

  // Waits for n rising clock edges.
  task cycles(input integer n);
    repeat (n) @(posedge sys_clk);
  endtask

  initial begin
    $dumpvars(0, main_tb);

    #1 `EXPECT_EQ(led, 1'b1)  // Off (lit when 0).
    cycles(8);
    #1 `EXPECT_EQ(led, 1'b0)  // Lit.
    cycles(8);
    #1 `EXPECT_EQ(led, 1'b1)  // Off again.
    cycles(16);

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
