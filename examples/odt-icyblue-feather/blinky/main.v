// Blinky for the ODT_IcyBlue_Feather: blinks its LED about once a second.
//
// No board clock is described, so the clock comes from the FPGA's internal
// 48 MHz oscillator (SB_HFOSC).

`timescale 1ns / 1ps  // For the simulation-only clock below.

module main #(
  parameter N = 26  // Counter width: 2^26 cycles at 48 MHz is about 1.4 s.
) (
  output led  // LED, lit when 1.
);

  // The clock: the internal oscillator at 48 MHz.
  wire clk;
`ifdef SYNTHESIZE
  SB_HFOSC #(.CLKHF_DIV("0b00")) osc (  // 0b00: 48 MHz, undivided.
    .CLKHFPU(1'b1),  // Powered up.
    .CLKHFEN(1'b1),  // Output enabled.
    .CLKHF(clk),
    // Trim inputs: unused, left unconnected (they act only with
    // CLKHF_TRIM_EN).
    .TRIM0(), .TRIM1(), .TRIM2(), .TRIM3(), .TRIM4(),
    .TRIM5(), .TRIM6(), .TRIM7(), .TRIM8(), .TRIM9()
  );
`else
  // Simulation only: a 48 MHz clock, as the oscillator gives.
  reg sim_clk = 1'b0;
  always #10.417 sim_clk = ~sim_clk;
  assign clk = sim_clk;
`endif

  // Counts clock cycles. Its top bit is 1 half the time.
  reg [N-1:0] counter = 0;
  always @(posedge clk) counter <= counter + 1'b1;

  // The LED is lit while the top bit is 1.
  assign led = counter[N-1];

endmodule
