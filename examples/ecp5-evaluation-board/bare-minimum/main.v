// Bare minimum for the ECP5-Evaluation-Board: the board's pins, ready for
// your design. It does nothing yet: the LED is held off.
//
// The 12 MHz clock from the board's FTDI chip reaches the FPGA only with
// jumper JP2 shorted, so there's no clock pin here.

module main (
  output led  // LED, lit when 0.
);

  // Everything off for now.
  assign led = 1'b1;

  // Your design goes here.

endmodule
