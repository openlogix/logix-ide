// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// in GTKWave with Simulate.

// Checks that 'actual_val' equals 'expected_val'. On a mismatch, reports
// an error with both values. Run Tests stops at the first error; Simulate
// runs to the end so the signals can be viewed.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected 'h%h, actual 'h%h", (expected_val), (actual_val));

`timescale 1ns / 1ps

module main_tb;

  // Clock. The design has no clock, it only paces the checks.
  reg sys_clk = 0;
  always #5 sys_clk = ~sys_clk;

  // Number of sys_clk rising edges so far.
  integer clk_num = 0;
  always @(posedge sys_clk) clk_num <= clk_num + 1;

  // The buttons, driven by the testbench.
  reg btn_n = 1;
  reg [2:0] btns = 3'b000;

  // The LEDs driven by the design.
  wire [1:0] leds_n;
  wire [2:0] pmod_leds;

  main dut (
      .btn_n(btn_n),
      .btns(btns),
      .leds_n(leds_n),
      .pmod_leds(pmod_leds)
  );

  // Loop index for the checks.
  integer i;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // Try every combination of the 4 buttons, one per clock.
    for (i = 0; i < 16; i = i + 1) begin
      @(posedge sys_clk);
      #1;
      btn_n = ~i[3];
      btns  = i[2:0];
      #1;
      // Pressed: red on (leds_n = 2'b10). Released: green on (2'b01).
      `EXPECT_EQ(leds_n, i[3] ? 2'b10 : 2'b01)
      `EXPECT_EQ(pmod_leds, i[2:0])
    end

    $display("End of simulation");
    $finish;
  end

endmodule
