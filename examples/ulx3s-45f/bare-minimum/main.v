// The top module must be named 'main'. Its ports are the board's
// pins in the pin file. The outputs start off; your design goes here.

module main (
    input        sys_clk,    // 25MHz clock
    output [7:0] leds,       // Off. Active high.
    output       wifi_gpio0  // Held high, see below.
);

  // Held high: wifi_gpio0. GPIO0 of the board's ESP32 WiFi module: held
  // low, it makes the ESP32 reset the board.
  assign wifi_gpio0 = 1'b1;

  // leds stays off.
  assign leds = 8'b00000000;

  // Your design goes here.

endmodule
