// Blinky for the ECP5-Evaluation-Board: blinks its LED about once a second.
//
// The 12 MHz clock from the board's FTDI chip reaches the FPGA only with
// jumper JP2 shorted, so the clock comes from the FPGA's internal
// oscillator (OSCG), about 2.4 MHz (310 MHz divided by 128).

`timescale 1ns / 1ps  // For the simulation-only clock below.

module main #(
  parameter N = 22  // Counter width: 2^22 cycles at 2.4 MHz is about 1.7 s.
) (
  output led  // LED, lit when 0.
);

  // The clock: the internal oscillator, about 2.4 MHz.
  wire clk;
`ifdef SYNTHESIZE
  OSCG #(.DIV(128)) osc (.OSC(clk));  // 310 MHz / 128.
`else
  // Simulation only: a 2.42 MHz clock, as the oscillator gives.
  reg sim_clk = 1'b0;
  always #206.452 sim_clk = ~sim_clk;
  assign clk = sim_clk;
`endif

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge clk) counter <= counter + 1'b1;

  // The LED is lit while the top bit is 1 (it lights at 0, so inverted).
  assign led = ~counter[N-1];

endmodule
