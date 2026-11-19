// UART transmitter: sends bytes over a serial line.
//
// It uses the common 8N1 format: each byte is a start bit (0), 8 data bits
// (least significant first) and a stop bit (1). Between bytes the line
// stays high. BAUD is the speed in bits per second, and CLK_HZ is your
// clock's frequency, so the module knows how many clock cycles each bit
// lasts.
//
// To receive too, use uart_rx.v, its partner. Copy this file, and its
// testbench uart_tx_tb.v, into any project that sends serial data.
//
// How to use it:
//
//   uart_tx #(.CLK_HZ(12_000_000), .BAUD(115_200)) sender (
//     .clk(clk),
//     .data(byte_to_send),   // The byte to send.
//     .valid(send_it),       // 1: send it.
//     .ready(can_send),      // 1: ready for a byte.
//     .tx(uart_tx)           // To the board's serial TX pin.
//   );
//
// Sending a byte: wait until ready is 1, then put the byte on data and set
// valid to 1 for one clock cycle. ready goes to 0 while the byte is sent,
// and back to 1 when it's done. If you keep valid at 1, the next byte (on
// data at that time) is sent right after.
//
// Settings that can't work stop the build and the simulation with an error
// about "uart_baud_rate_too_far_off": a baud rate more than 2% away from
// what the clock can make, or a baud rate too slow for the 16-bit counter.

module uart_tx #(
  parameter CLK_HZ = 12_000_000,  // Your clock's frequency, in Hz.
  parameter BAUD = 115_200        // Bits per second.
) (
  input clk,          // Your clock.
  input [7:0] data,   // The byte to send.
  input valid,        // 1: send data now (if ready is 1).
  output ready,       // 1: ready to send a byte.
  output tx           // Serial line out, to the other side's RX.
);

  // How many clock cycles one bit lasts, rounded to the nearest whole
  // number. At 12 MHz and 115200 baud that's 104.
  localparam integer CYCLES_PER_BIT = (CLK_HZ + BAUD / 2) / BAUD;

  // The same number as 16 bits, the size of the cycle counter below.
  localparam [15:0] BIT_CYCLES = CYCLES_PER_BIT[15:0];

  // Check the settings. If they can't work, this instantiates a module that
  // doesn't exist, so the build stops with its name as the error message.
  localparam REAL_BAUD_TIMES_CYCLES = CYCLES_PER_BIT * BAUD;  // CLK_HZ if exact.
  generate
    if (CYCLES_PER_BIT < 2 || CYCLES_PER_BIT > 65535
        || REAL_BAUD_TIMES_CYCLES > CLK_HZ + CLK_HZ / 50
        || REAL_BAUD_TIMES_CYCLES < CLK_HZ - CLK_HZ / 50) begin : bad_settings
      uart_baud_rate_too_far_off check_CLK_HZ_and_BAUD ();
    end
  endgenerate

  // The bits still to send. Bit 0 is the one on the line now. A new byte is
  // loaded as {stop bit, 8 data bits, start bit}, and each bit time it
  // shifts right by one, filling with 1s, so the line ends up high (idle).
  reg [9:0] bits = 10'b11_1111_1111;
  reg [3:0] bits_left = 4'd0;       // Bits still to send. 0 = idle.
  reg [15:0] cycles_left = 16'd0;   // Clock cycles left in the current bit.

  assign tx = bits[0];
  assign ready = (bits_left == 4'd0);

  always @(posedge clk) begin
    if (bits_left == 4'd0) begin
      // Idle: start sending when asked.
      if (valid) begin
        bits <= {1'b1, data, 1'b0};
        bits_left <= 4'd10;
        cycles_left <= BIT_CYCLES - 16'd1;
      end
    end else if (cycles_left == 16'd0) begin
      // The current bit has lasted long enough: go to the next one.
      bits <= {1'b1, bits[9:1]};
      bits_left <= bits_left - 4'd1;
      cycles_left <= BIT_CYCLES - 16'd1;
    end else begin
      cycles_left <= cycles_left - 16'd1;
    end
  end

endmodule
