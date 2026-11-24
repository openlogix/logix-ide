// Testbench for the bare minimum design: checks that every output is at its
// resting level (the LED off, the serial line idle). Add your own checks as
// your design grows.

`timescale 1ns / 1ps

// Reports a failed check with the actual and expected values. Not fatal.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("%s is %0h, expected %0h", `"actual_val`", (actual_val), (expected_val));

module main_tb;

  // The design's ports.
  reg sys_clk = 1'b0;
  wire [2:0] rgb_led;
  reg [3:0] switches = 4'b1111;  // All off
  wire uart_tx;
  reg uart_rx = 1'b1;            // Idle

  main dut (
    .sys_clk(sys_clk),
    .rgb_led(rgb_led),
    .switches(switches),
    .uart_tx(uart_tx),
    .uart_rx(uart_rx)
  );

  // The board's 12 MHz clock: a period of 83.333 ns.
  always #41.667 sys_clk = !sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    // Let the design run for 32 clock cycles.
    repeat (32) @(posedge sys_clk);
    #1;

    // The RGB LED is off: all three colors at 1.
    `EXPECT_EQ(rgb_led, 3'b111)

    // The serial line is idle, at 1.
    `EXPECT_EQ(uart_tx, 1'b1)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
