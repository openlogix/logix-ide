// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock.
  reg sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // Clocks per color, reduced to run quickly.
  localparam integer DIV = 3;

  // The button, driven by the testbench. Released at the start.
  reg btn_n = 1;

  // The RGB LED driven by the design.
  wire [2:0] rgb_led_n;

  main #(
      .DIV(DIV)
  ) dut (
      .sys_clk(sys_clk),
      .btn_n(btn_n),
      .rgb_led_n(rgb_led_n)
  );

  // The colors in order, red, blue and green, as one-hot values.
  function [2:0] color(input integer step);
    case (step % 3)
      0: color = 3'b001;
      1: color = 3'b100;
      default: color = 3'b010;
    endcase
  endfunction

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // The reset is high for the first 3 clocks. After clock i >= 3 the
    // colors have moved (i - 3) / DIV steps from red.
    for (i = 1; i <= 15; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      `EXPECT_EQ(dut.sys_rst, i < 3)
      if (i >= 3) `EXPECT_EQ(rgb_led_n, ~color((i - 3) /DIV))
    end

    // Press the button for 2 clocks. After the synchronizer's delay the
    // design resets. The reset ends at the 5th clock after the release, and
    // the colors start again from red.
    btn_n = 0;
    repeat (2) @(posedge sys_clk);
    #1;
    btn_n = 1;
    for (i = 1; i <= 10; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      if (i == 1) `EXPECT_EQ(dut.sys_rst, 1'b1)
      if (i >= 5) `EXPECT_EQ(dut.sys_rst, 1'b0)
      if (i >= 5) `EXPECT_EQ(rgb_led_n, ~color((i - 5) /DIV))
    end

    $display("End of simulation");
    $finish;
  end

endmodule
