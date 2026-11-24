// Testbench for main.v.
// Run it in batch mode with Run Tests, or view the signals
// over time with Simulate.

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

  // The RGB LED driven by the design.
  wire [2:0] rgb_led_n;

  // The fade moves one step every 2 clocks, so a color rises and falls in
  // about 8,200 clocks. Each color's highest brightness is as in main.v.
  localparam [11:0] RED_MAX = 249, GREEN_MAX = 156, BLUE_MAX = 499;
  main #(
      .STEP(2),
      .RED_MAX(RED_MAX),
      .GREEN_MAX(GREEN_MAX),
      .BLUE_MAX(BLUE_MAX)
  ) dut (
      .sys_clk  (sys_clk),
      .rgb_led_n(rgb_led_n)
  );

  // The color of the fade now (0 red, 1 green, 2 blue), its highest duty:
  // the top of the gamma curve (4095) scaled to the color's highest, the
  // highest duty seen in this fade, the duty a clock ago, and whether the
  // fade has started falling.
  integer color = 0;
  integer top = 0;
  integer peak = 0;
  integer last = 0;
  reg     falling = 0;
  // How many fades ended, and the loop index.
  integer fades = 0;
  integer i = 0;

  function integer top_of(input integer c);
    top_of = (4095 * (c == 0 ? RED_MAX : c == 1 ? GREEN_MAX : BLUE_MAX)) / 4096;
  endfunction

  // The test sequence: 4 fades, red, green, blue, red again. Every clock:
  // the LED is off or on in the fade's color only, on exactly while the
  // PWM counter is below the duty, and the duty rises, then falls, never
  // above the color's highest. Each fade reaches exactly its highest.
  initial begin
    $dumpvars(0, main_tb);
    top = top_of(0);

    for (i = 0; fades < 4; i = i + 1) begin
      if (i > 0) @(posedge sys_clk);
      #1;
      // The next fade: the color changed while the LED was off.
      if (dut.color != color) begin
        `EXPECT_EQ(peak, top)
        `EXPECT_EQ(dut.color, (color + 1) % 3)
        color = dut.color;
        top = top_of(color);
        peak = 0;
        last = 0;
        falling = 0;
        fades = fades + 1;
      end
      if (rgb_led_n !== 3'b111) `EXPECT_EQ(rgb_led_n, ~(3'b001 << color))
      `EXPECT_EQ(rgb_led_n !== 3'b111, dut.pwm < dut.duty)
      if (dut.duty > top) $error("duty %0d is above the color's highest, %0d", dut.duty, top);
      if (dut.duty < last) falling = 1;
      if (falling && dut.duty > last) $error("duty rose from %0d to %0d while falling", last, dut.duty);
      if (dut.duty > peak) peak = dut.duty;
      last = dut.duty;
    end

    $display("End of simulation");
    $finish;
  end

endmodule
