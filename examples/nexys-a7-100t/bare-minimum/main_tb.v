// Testbench for the Nexys A7-100T / Nexys 4 DDR's bare minimum: checks that
// the LEDs are off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [15:0] leds;

  main dut (.sys_clk(sys_clk), .leds(leds));

  // The board's 100 MHz clock.
  always #5 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds, 16'b0000000000000000)  // All off (lit when 1).

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
