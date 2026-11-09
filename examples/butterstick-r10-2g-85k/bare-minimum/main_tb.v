// Testbench for the butterstick r1.0's bare minimum: checks that the LEDs
// are off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [6:0] leds;
  wire [2:0] leds_colors;

  main dut (.sys_clk(sys_clk), .leds(leds), .leds_colors(leds_colors));

  // The board's 30 MHz clock.
  always #16.667 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds, 7'b0000000)     // All off (lit when 1).
    `EXPECT_EQ(leds_colors, 3'b000)  // No color.

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
