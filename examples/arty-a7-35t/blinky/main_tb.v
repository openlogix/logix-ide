// Testbench for blinky: with a 4-bit counter, LED 0 is off at first, on
// after 8 clock cycles and off again after 16.

`timescale 1ns / 1ps

// Reports a failed check with the actual and expected values. Not fatal.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("%s is %0h, expected %0h", `"actual_val`", (actual_val), (expected_val));

module main_tb;

  // The design's ports.
  reg sys_clk = 1'b0;
  wire [3:0] leds;

  // A 4-bit counter, so the LED changes every 8 clock cycles.
  main #(.N(4)) dut (
    .sys_clk(sys_clk),
    .leds(leds)
  );

  // The board's 100 MHz clock: a period of 10 ns.
  always #5.000 sys_clk = !sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    // The LED is off at first.
    #1;
    `EXPECT_EQ(leds[0], 1'b0)

    // The other LEDs are held off.
    `EXPECT_EQ(leds[3:1], 3'b000)

    // On after 8 clock cycles.
    repeat (8) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds[0], 1'b1)

    // Off again after 16.
    repeat (8) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds[0], 1'b0)

    // Add your own checks here.

    // Run on to 32 clock cycles.
    repeat (16) @(posedge sys_clk);

    $display("End of simulation");
    $finish;
  end

endmodule
