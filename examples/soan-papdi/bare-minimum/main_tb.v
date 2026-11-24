// Testbench for the bare minimum design: checks that every output is at its
// resting level (all LEDs off). Add your own checks as your design grows.

`timescale 1ns / 1ps

// Reports a failed check with the actual and expected values. Not fatal.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("%s is %0h, expected %0h", `"actual_val`", (actual_val), (expected_val));

module main_tb;

  // The design's ports.
  reg sys_clk = 1'b0;
  wire [7:0] leds;
  wire [3:0] status_leds;
  reg prog_button = 1'b0;      // Not pressed
  reg [3:0] switches_a = 4'b0000;  // All off
  reg [3:0] switches_b = 4'b0000;  // All off

  main dut (
    .sys_clk(sys_clk),
    .leds(leds),
    .status_leds(status_leds),
    .prog_button(prog_button),
    .switches_a(switches_a),
    .switches_b(switches_b)
  );

  // The board's 12 MHz clock: a period of 83.333 ns.
  always #41.667 sys_clk = !sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    // Let the design run for 32 clock cycles.
    repeat (32) @(posedge sys_clk);
    #1;

    // All LEDs are off: LEDs 0 to 7 at 0, S0 to S3 at 1.
    `EXPECT_EQ(leds, 8'b00000000)
    `EXPECT_EQ(status_leds, 4'b1111)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
