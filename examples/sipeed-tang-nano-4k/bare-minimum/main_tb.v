// Testbench for the Sipeed Tang Nano 4k's bare minimum: checks that the LED
// is off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire led;

  main dut (.sys_clk(sys_clk), .led(led));

  // The board's 27 MHz clock.
  always #18.519 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(led, 1'b1)  // Off (lit when 0).

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
