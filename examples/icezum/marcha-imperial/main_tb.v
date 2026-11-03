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

  // The signals driven by the design.
  wire [7:0] leds;
  wire buzzer;

  // Clocks per note, reduced to run quickly.
  localparam integer DUR = 3;

  main #(
      .DUR(DUR)
  ) dut (
      .sys_clk(sys_clk),
      .leds(leds),
      .buzzer(buzzer)
  );

  // The ROM address and note as of the previous clock.
  reg [7:0] prev_addr;
  reg [15:0] prev_note;

  // Loop index for the checks.
  integer i;

  // The expected ROM address. Compared through [7:0], to match the
  // signal's width.
  integer expected;

  // The test sequence.
  initial begin
    $dumpvars(0, main_tb);

    // The ROM address goes up every DUR clocks, the first time at clock
    // 1. The note is the ROM entry at the previous clock's address, and
    // the LEDs show its low byte. During a rest (note 0) the buzzer is
    // silent.
    for (i = 1; i <= 60; i = i + 1) begin
      prev_addr = dut.addr;
      prev_note = dut.note;
      @(posedge sys_clk);
      #1;
      expected = ((i - 1) / DUR + 1) % 256;
      `EXPECT_EQ(dut.addr, expected[7:0])
      `EXPECT_EQ(dut.note, dut.ROM.rom[prev_addr])
      `EXPECT_EQ(leds, dut.note[7:0])
      if (i > 1 && prev_note == 0) `EXPECT_EQ(buzzer, 1'b0)
    end

    $display("End of simulation");
    $finish;
  end

endmodule
