// Testbench for the Fomu's blinky. With a 4-bit counter, the green LED is
// off at first, lit after 8 clock cycles and off again after 16; red and
// blue stay off and the USB pins stay low. Run Tests checks it; Simulate
// shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  wire [2:0] rgb_led;
  wire usb_dp;
  wire usb_dn;
  wire usb_dp_pu;

  // The design, with a 4-bit counter so the LED changes every 8 cycles.
  main #(.N(4)) dut (.sys_clk(sys_clk), .rgb_led(rgb_led), .usb_dp(usb_dp), .usb_dn(usb_dn), .usb_dp_pu(usb_dp_pu));

  // The board's 48 MHz clock.
  always #10.417 sys_clk = ~sys_clk;

  // Waits for n rising edges of the clock.
  task cycles(input integer n);
    repeat (n) @(posedge sys_clk);
  endtask

  initial begin
    $dumpvars(0, main_tb);

    #1 `EXPECT_EQ(rgb_led, 3'b111)  // All off (each color lit when 0).
    `EXPECT_EQ(usb_dp, 1'b0)  // Held low.
    `EXPECT_EQ(usb_dn, 1'b0)  // Held low.
    `EXPECT_EQ(usb_dp_pu, 1'b0)  // Held low.
    cycles(8);
    #1 `EXPECT_EQ(rgb_led, 3'b101)  // Green lit.
    cycles(8);
    #1 `EXPECT_EQ(rgb_led, 3'b111)  // Off again.
    cycles(16);

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
