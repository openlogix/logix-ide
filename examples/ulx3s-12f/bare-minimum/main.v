// Bare minimum for the ULX3S 12K: the board's pins, ready for your design.
// It does nothing yet: the LEDs are held off.

module main (
  input sys_clk,     // 25 MHz clock.
  output [7:0] leds, // 8 LEDs, lit when 1.
  output wifi_gpio0  // Held high: GPIO0 of the board's ESP32 WiFi module.
);

  // Held high in every design: low, it makes the ESP32 reset the board.
  assign wifi_gpio0 = 1'b1;

  // Everything off for now.
  assign leds = 8'b00000000;

  // Your design goes here.

endmodule
