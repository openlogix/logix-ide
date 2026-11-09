// Testbench for the ULX3S 45K's bare minimum: checks that the LEDs are off
// and wifi_gpio0 is high. Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [7:0] leds;
  wire wifi_gpio0;

  main dut (.sys_clk(sys_clk), .leds(leds), .wifi_gpio0(wifi_gpio0));

  // The board's 25 MHz clock.
  always #20 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1;
    `EXPECT_EQ(leds, 8'b00000000)  // All off (lit when 1).
    `EXPECT_EQ(wifi_gpio0, 1'b1)  // Held high.

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
