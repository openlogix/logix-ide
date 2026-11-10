// Testbench for the iCESugar v1.5's bare minimum: checks that the RGB LED
// is off and the UART is idle. Run Tests checks it; Simulate shows the
// signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %b, got %b", (expected_val), (actual_val));

module main_tb;

  reg sys_clk = 1'b0;
  reg [3:0] switches = 4'b1111;
  reg uart_rx = 1'b1;
  wire [2:0] rgb_led;
  wire uart_tx;

  // The design.
  main dut (
    .sys_clk(sys_clk),
    .rgb_led(rgb_led),
    .switches(switches),
    .uart_rx(uart_rx),
    .uart_tx(uart_tx)
  );

  // The board's 12 MHz clock.
  always #41.667 sys_clk = ~sys_clk;

  initial begin
    $dumpvars(0, main_tb);

    repeat (32) @(posedge sys_clk);
    #1 `EXPECT_EQ(rgb_led, 3'b111)  // Every color off (lit when 0).
    `EXPECT_EQ(uart_tx, 1'b1)       // The UART idle.

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
