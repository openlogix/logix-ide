// Testbench for the FPGA 101 Workshop Badge's bare minimum: checks that the
// RGB LED is off. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [2:0] rgb_led;

  main dut (.sys_clk(sys_clk), .rgb_led(rgb_led));

  // The board's 12 MHz clock.
  always #41.667 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(rgb_led, 3'b111)  // All off (each color lit when 0).

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
