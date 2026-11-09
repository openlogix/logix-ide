// Testbench for the MicroPhase A7-Lite's bare minimum: checks that the LEDs
// are off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [1:0] leds_n;

  main dut (.sys_clk(sys_clk), .leds_n(leds_n));

  // The board's 50 MHz clock.
  always #10 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds_n, 2'b11)  // All off (lit when 0).

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
