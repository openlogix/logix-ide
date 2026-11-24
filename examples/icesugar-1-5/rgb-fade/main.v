// Fades the RGB LED up and down, one color at a time: red, green, blue,
// red, and so on. Each color rises from off to its highest brightness and
// falls back to off, and the LED moves to the next color each time it's
// off. Blue goes up to about 12% of full power; red and green, which
// look brighter at the same power, go less high, so the three look
// alike. The fade follows a 2.5 gamma curve (gamma.v), so it looks even
// to the eye from off to the top.
//
// Try it: Upload, and watch the RGB LED fade in and out, red, then green,
// then blue. Run Tests checks the design and the gamma table; Simulate
// shows the PWM and the fade.

// The LED is switched with 12-bit pulse width modulation: on for 'duty'
// of every 4096 clocks, about 2,900 times a second, far too fast to see.
// STEP sets the speed: the fade moves one step every STEP clocks, 4,094
// steps up and down per color. With the default STEP = 11724 and a 12MHz
// clock, each color rises and falls in about 4 seconds. The testbench
// uses a small STEP to run quickly.
//
// RED_MAX, GREEN_MAX and BLUE_MAX are each color's highest brightness, in
// PWM steps out of 4096: 2048 is 50%, 1024 is 25%. Tune red and green by
// eye until they look as bright as blue. To look X times as bright, all
// three take X^2.5 times the power (the gamma): they were 512, 320 and
// 1024, and 0.75^2.5 = 0.487 of those made the LED look 25% dimmer.
module main #(
    parameter integer STEP = 11724,
    parameter [11:0] RED_MAX = 249,
    parameter [11:0] GREEN_MAX = 156,
    parameter [11:0] BLUE_MAX = 499
) (
    input        sys_clk,   // 12MHz clock
    output [2:0] rgb_led_n  // RGB LED, active low. [0]=red, [1]=green, [2]=blue.
);

  // The PWM counter: the LED is on while it's below the duty.
  reg [11:0] pwm = 0;

  // Counts the clocks of a fade step, from 0 to STEP - 1.
  localparam integer TW = $clog2(STEP);
  localparam integer LAST = STEP - 1;
  reg  [TW-1:0] timer = 0;
  wire          step = timer == LAST[TW-1:0];

  // How far the fade is, from 0 (off) to 2047 (the top), whether it's
  // rising or falling, and the color: 0 red, 1 green, 2 blue.
  reg [10:0] level = 0;
  reg        rising = 1;
  reg [ 1:0] color = 0;

  always @(posedge sys_clk) begin
    pwm   <= pwm + 1;
    timer <= step ? 0 : timer + 1;
    // Once every STEP clocks, one step up or down.
    if (step) begin
      if (rising) begin
        level <= level + 1;
        if (level == 2046) rising <= 0;
      end else begin
        level <= level - 1;
        // Off again: rise again, in the next color.
        if (level == 1) begin
          rising <= 1;
          color  <= (color == 2) ? 2'd0 : color + 2'd1;
        end
      end
    end
  end

  // The brightness on the gamma curve, from the fade's top 8 bits: 0 to
  // 4095, ready one clock later.
  wire [11:0] curve;
  gamma gamma_table (
      .clk  (sys_clk),
      .index(level[10:3]),
      .value(curve)
  );

  // The color's highest brightness, and the duty: the curve scaled to it,
  // so at the top it's the color's highest.
  wire [11:0] color_max = (color == 0) ? RED_MAX : (color == 1) ? GREEN_MAX : BLUE_MAX;
  wire [23:0] product = curve * color_max;
  wire [11:0] duty = product[23:12];  // curve * color_max / 4096

  // The LED is on for 'duty' of the 4096 PWM steps, in its color.
  wire       on = pwm < duty;
  wire [2:0] rgb_led = on ? (3'b001 << color) : 3'b000;

  // The RGB LED is active low, so invert it.
  assign rgb_led_n = ~rgb_led;

endmodule
