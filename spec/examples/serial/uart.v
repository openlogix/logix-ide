// UART: a serial port's transmitter (uart_tx) and receiver (uart_rx), two
// modules in one file that work on their own: use either one, or both. A
// module you don't use costs nothing: the build leaves it out. Copy this
// file, and its testbench uart_tb.v, into any project that needs a serial
// port.
//
// Both use the common 8N1 format: each byte is a start bit (0), 8 data bits
// (least significant first) and a stop bit (1). Between bytes the line
// stays high. BAUD is the speed in bits per second, and CLK_HZ is your
// clock's frequency, so each module knows how many clock cycles a bit lasts.
//
// Settings that can't work stop the build and the simulation with an error
// about "uart_baud_rate_too_far_off": a baud rate more than 2% away from
// what the clock can make, or a baud rate too slow for the 16-bit counter.

// ---------------------------------------------------------------------------
// uart_tx: the transmitter, which sends bytes.
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
  // shifts right by one. Like every register here, it starts at 0: some
  // FPGAs, such as the iCE40, can only start a register at 0.
  reg [9:0] bits = 10'd0;
  reg [3:0] bits_left = 4'd0;       // Bits still to send. 0 = idle.
  reg [15:0] cycles_left = 16'd0;   // Clock cycles left in the current bit.

  assign ready = (bits_left == 4'd0);

  // The line: high while idle (before the first byte too), else the bit
  // being sent.
  assign tx = ready ? 1'b1 : bits[0];

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
      bits <= {1'b0, bits[9:1]};
      bits_left <= bits_left - 4'd1;
      cycles_left <= BIT_CYCLES - 16'd1;
    end else begin
      cycles_left <= cycles_left - 16'd1;
    end
  end

endmodule

// ---------------------------------------------------------------------------
// uart_rx: the receiver, which receives bytes.
//
// How to use it:
//
//   uart_rx #(.CLK_HZ(12_000_000), .BAUD(115_200)) receiver (
//     .clk(clk),
//     .rx(uart_rx),          // From the board's serial RX pin.
//     .data(byte_received),  // The last byte received.
//     .valid(got_byte),      // 1 for one cycle when a byte arrives.
//     .error()               // 1 for one cycle on a bad byte (unused here).
//   );
//
// Receiving a byte: when a byte has arrived, valid is 1 for exactly one
// clock cycle, and the byte is in data. Use it in that cycle (data keeps
// it until the next byte, but valid won't repeat). If a byte's stop bit is
// 0 (often a wrong baud rate), error is 1 for one cycle instead and the
// byte is dropped.
//

module uart_rx #(
  parameter CLK_HZ = 12_000_000,  // Your clock's frequency, in Hz.
  parameter BAUD = 115_200        // Bits per second.
) (
  input clk,                    // Your clock.
  input rx,                     // Serial line in, from the other side's TX.
  output reg [7:0] data = 8'd0, // The last byte received.
  output reg valid = 1'b0,      // 1 for one cycle: a new byte is in data.
  output reg error = 1'b0       // 1 for one cycle: a byte with a bad stop bit.
);

  // How many clock cycles one bit lasts, rounded to the nearest whole
  // number. At 12 MHz and 115200 baud that's 104.
  localparam integer CYCLES_PER_BIT = (CLK_HZ + BAUD / 2) / BAUD;
  localparam integer HALF_BIT = CYCLES_PER_BIT / 2;

  // The same two numbers as 16 bits, the size of the cycle counter below.
  localparam [15:0] BIT_CYCLES = CYCLES_PER_BIT[15:0];
  localparam [15:0] HALF_BIT_CYCLES = HALF_BIT[15:0];

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

  // rx comes from outside and can change at any time, not just on a clock
  // edge. Passing it through two flip-flops first (a "synchronizer") keeps
  // the rest of the receiver safe from that. They start at 0, like every
  // register here (some FPGAs, such as the iCE40, can only start a register
  // at 0), which looks like a start bit: so the receiver begins by waiting
  // for the line to be high, below.
  reg rx_meta = 1'b0;
  reg rx_synced = 1'b0;
  always @(posedge clk) begin
    rx_meta <= rx;
    rx_synced <= rx_meta;
  end

  // The receiver's steps. It reads each bit in the middle of its time,
  // where the signal is most stable.
  localparam WAIT_HIGH = 3'd0;   // At first, and after a bad stop bit: wait for a high line.
  localparam WAIT_START = 3'd1;  // Line high: waiting for it to go low (a start bit).
  localparam CHECK_START = 3'd2; // Half a bit later: still low? Then it's a real start bit.
  localparam READ_DATA = 3'd3;   // Reading the 8 data bits.
  localparam CHECK_STOP = 3'd4;  // Reading the stop bit: it must be 1.

  reg [2:0] step = WAIT_HIGH;
  reg [15:0] cycles_left = 16'd0;  // Clock cycles until the next read of the line.
  reg [2:0] bit_number = 3'd0;     // Which data bit is read next (0 to 7).
  reg [7:0] bits = 8'd0;           // The data bits read so far.

  always @(posedge clk) begin
    // valid and error are 1 for one cycle only.
    valid <= 1'b0;
    error <= 1'b0;

    case (step)
      WAIT_HIGH:
        if (rx_synced == 1'b1) step <= WAIT_START;

      WAIT_START:
        if (rx_synced == 1'b0) begin
          step <= CHECK_START;
          cycles_left <= HALF_BIT_CYCLES - 16'd1;
        end

      CHECK_START:
        if (cycles_left != 16'd0) begin
          cycles_left <= cycles_left - 16'd1;
        end else if (rx_synced == 1'b0) begin
          // Still low in the middle of the start bit: a byte is coming.
          step <= READ_DATA;
          cycles_left <= BIT_CYCLES - 16'd1;
          bit_number <= 3'd0;
        end else begin
          // High again already: it was a short glitch, not a start bit.
          step <= WAIT_START;
        end

      READ_DATA:
        if (cycles_left != 16'd0) begin
          cycles_left <= cycles_left - 16'd1;
        end else begin
          // The middle of a data bit. Bits arrive least significant first,
          // so each one goes in at the top and the others move down.
          bits <= {rx_synced, bits[7:1]};
          cycles_left <= BIT_CYCLES - 16'd1;
          if (bit_number == 3'd7) step <= CHECK_STOP;
          bit_number <= bit_number + 3'd1;
        end

      CHECK_STOP:
        if (cycles_left != 16'd0) begin
          cycles_left <= cycles_left - 16'd1;
        end else if (rx_synced == 1'b1) begin
          // A good stop bit: the byte is complete.
          data <= bits;
          valid <= 1'b1;
          step <= WAIT_START;
        end else begin
          // A bad stop bit: drop the byte.
          error <= 1'b1;
          step <= WAIT_HIGH;
        end

      default:  // No other steps: start over.
        step <= WAIT_HIGH;
    endcase
  end

endmodule
