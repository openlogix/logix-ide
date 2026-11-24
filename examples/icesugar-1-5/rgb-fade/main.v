// rgb-fade for the iCESugar v1.5: fades the RGB LED in and out, one color at
// a time: red, then green, then blue, then red again. Each color rises
// from off to its highest brightness and falls back to off, and the LED
// moves to the next color each time it's off. Red and green, which look
// brighter at the same power, go less high than blue, so the three look
// alike. The fade follows a 2.5 gamma curve (gamma.v), so it looks even
// to the eye from off to the top.
//
// Try it: Upload, and watch the RGB LED fade in and out, red, then green,
// then blue. Run Tests checks the design and the gamma table; Simulate
// shows the PWM and the fade.

// The LED is switched with 12-bit pulse width modulation: on for 'duty'
// of every 4096 clocks, about 2,930 times a second, far too fast to
// see. STEP sets the speed: the fade moves one step every STEP clocks,
// 4,094 steps up and down per color. With the default STEP = 11724 and
// the 12 MHz clock, each color rises and falls in about 4 seconds. The
// testbench uses a small STEP to run quickly.
//
// RED_MAX, GREEN_MAX and BLUE_MAX are each color's highest brightness, in
// PWM steps out of 4096: 2048 is 50%, 1024 is 25%. Tune red and green by
// eye until they look as bright as blue. To look X times as bright, all
// three take X^2.5 times the power (the gamma): 0.75^2.5 = 0.49 times
// the power looks 25% dimmer.
module main #(
    parameter integer STEP = 11724,
    parameter [11:0] RED_MAX = 121,
    parameter [11:0] GREEN_MAX = 76,
    parameter [11:0] BLUE_MAX = 243
) (
    input        sys_clk,  // 12 MHz clock
    output [2:0] rgb_led   // RGB LED: [0] red, [1] green, [2] blue, lit when 0.
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
  wire [11:0] color_max = (color == 0) ? RED_MAX :
                          (color == 1) ? GREEN_MAX : BLUE_MAX;
  wire [23:0] product = curve * color_max;
  wire [11:0] duty = product[23:12];  // curve * color_max / 4096

  // Each color's PWM: on while the counter is below the duty, in the
  // fade's color only.
  wire on = pwm < duty;
  wire red_on = on && color == 0;
  wire green_on = on && color == 1;
  wire blue_on = on && color == 2;

  // The RGB LED driver. The board wires the LED to the driver's pads:
  // red to RGB1, green to RGB0, blue to RGB2.
  // A PWM input of 1 lights that color; the pads sink the lowest current,
  // 2 mA, plenty for an indicator LED.
`ifdef SYNTHESIZE
  SB_RGBA_DRV #(
      .CURRENT_MODE("0b1"),  // Half current: the lowest settings.
      .RGB0_CURRENT("0b000001"),
      .RGB1_CURRENT("0b000001"),
      .RGB2_CURRENT("0b000001")
  ) rgb_driver (
      .CURREN  (1'b1),  // Current reference on.
      .RGBLEDEN(1'b1),  // Driver on.
      .RGB0PWM (green_on),
      .RGB1PWM (red_on),
      .RGB2PWM (blue_on),
      .RGB0    (rgb_led[1]),
      .RGB1    (rgb_led[0]),
      .RGB2    (rgb_led[2])
  );
`else
  // Simulation only: each pad low (lit) while its color's PWM is 1.
  assign rgb_led = ~{blue_on, green_on, red_on};
`endif

endmodule
