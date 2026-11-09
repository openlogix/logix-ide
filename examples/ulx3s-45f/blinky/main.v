// Blinky for the ULX3S 45K: blinks LED 0 about once a second.

module main #(
  parameter N = 25  // Counter width: 2^25 cycles at 25 MHz is about 1.3 s.
) (
  input sys_clk,     // 25 MHz clock.
  output [7:0] leds, // 8 LEDs, lit when 1.
  output wifi_gpio0  // Held high: GPIO0 of the board's ESP32 WiFi module.
);

  // Held high in every design: low, it makes the ESP32 reset the board.
  assign wifi_gpio0 = 1'b1;

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge sys_clk) counter <= counter + 1'b1;

  // LED 0 is lit while the top bit is 1; the others stay off.
  assign leds = {7'b0000000, counter[N-1]};

endmodule
