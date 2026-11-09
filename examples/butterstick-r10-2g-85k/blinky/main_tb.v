// Testbench for the butterstick r1.0's blinky. With a 4-bit counter, LED 0
// is off at first, lit after 8 clock cycles and off again after 16; the
// other LEDs stay off, and the color is always green. Run Tests checks it;
// Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [6:0] leds;
  wire [2:0] leds_colors;

  // The design, with a 4-bit counter so the LED changes every 8 cycles.
  main #(.N(4)) dut (.sys_clk(sys_clk), .leds(leds), .leds_colors(leds_colors));

  // The board's 30 MHz clock.
  always #16.667 sys_clk = ~sys_clk;

  // Waits for n rising clock edges.
  task cycles(input integer n);
    repeat (n) @(posedge sys_clk);
  endtask

  initial begin
    $dumpvars(0, main_tb);

    #1 `EXPECT_EQ(leds, 7'b0000000)  // All off.
    `EXPECT_EQ(leds_colors, 3'b010)  // Green.
    cycles(8);
    #1 `EXPECT_EQ(leds, 7'b0000001)  // LED 0 lit.
    cycles(8);
    #1 `EXPECT_EQ(leds, 7'b0000000)  // Off again.
    cycles(16);

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
