// Testbench for uart.v: uart_tx and uart_rx. Copy it along with uart.v into
// any project.
//
// It checks each module on its own, against the serial format
// itself (start bit 0, 8 data bits least significant first, stop bit 1):
//
//   Transmitter: the line is high when idle; each byte comes out bit by bit
//   with the right timing; tx_ready is 0 while a byte is being sent; with
//   tx_valid held at 1, bytes go out back to back.
//
//   Receiver: bytes sent to rx come out on rx_data, with rx_valid for one
//   cycle; a byte with a bad stop bit gives rx_error instead; a short glitch
//   on the line is ignored.
//
// It doesn't connect tx to rx: a mistake made the same way in both halves
// (such as the wrong bit order) would then go unnoticed.
//
// To keep the run short, it uses a fast baud rate: 1 Mbit/s from a 12 MHz
// clock, so a bit is 12 clock cycles. Run Tests checks it; Simulate shows
// the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module uart_tb;

  localparam CYCLES_PER_BIT = 12;  // 12 MHz / 1 Mbit/s.

  reg clk = 1'b0;
  reg [7:0] tx_data = 8'd0;
  reg tx_valid = 1'b0;
  wire tx_ready;
  wire tx;
  reg rx = 1'b1;  // Idle high.
  wire [7:0] rx_data;
  wire rx_valid;
  wire rx_error;

  // The two modules, each on its own (not connected to each other).
  uart_tx #(.CLK_HZ(12_000_000), .BAUD(1_000_000)) sender (
    .clk(clk), .data(tx_data), .valid(tx_valid), .ready(tx_ready), .tx(tx)
  );
  uart_rx #(.CLK_HZ(12_000_000), .BAUD(1_000_000)) receiver (
    .clk(clk), .rx(rx), .data(rx_data), .valid(rx_valid), .error(rx_error)
  );

  // A 12 MHz clock.
  always #41.667 clk = ~clk;

  // Counts the receiver's rx_valid and rx_error pulses, and keeps the last
  // byte it received.
  integer bytes_received = 0;
  integer errors = 0;
  reg [7:0] last_byte = 8'd0;
  always @(posedge clk) begin
    if (rx_valid) begin
      bytes_received = bytes_received + 1;
      last_byte = rx_data;
    end
    if (rx_error) errors = errors + 1;
  end

  // ---------------------------------------------------------------------
  // Helpers for the transmitter
  // ---------------------------------------------------------------------

  // Gives a byte to the transmitter: waits for tx_ready, then sets
  // tx_valid for one clock cycle.
  task give_byte(input [7:0] b);
    begin
      @(negedge clk);
      while (!tx_ready) @(negedge clk);
      tx_data = b;
      tx_valid = 1'b1;
      @(negedge clk);
      tx_valid = 1'b0;
    end
  endtask

  // Reads one byte from tx, as the other side would: waits for the start
  // bit, then reads each bit in its middle, and checks the start and stop
  // bits.
  task read_byte(output [7:0] b);
    integer i;
    begin
      @(negedge tx);
      repeat (CYCLES_PER_BIT / 2) @(posedge clk);
      `EXPECT_EQ(tx, 1'b0)  // Start bit.
      for (i = 0; i < 8; i = i + 1) begin
        repeat (CYCLES_PER_BIT) @(posedge clk);
        b[i] = tx;          // Data bits, least significant first.
      end
      repeat (CYCLES_PER_BIT) @(posedge clk);
      `EXPECT_EQ(tx, 1'b1)  // Stop bit.
    end
  endtask

  // Sends a byte and checks that it comes out on tx.
  task check_send(input [7:0] b);
    reg [7:0] got;
    begin
      fork
        give_byte(b);
        read_byte(got);
      join
      `EXPECT_EQ(got, b)
    end
  endtask

  // ---------------------------------------------------------------------
  // Helpers for the receiver
  // ---------------------------------------------------------------------

  // Sends a byte on rx, as the other side would: start bit, 8 data bits
  // (least significant first), then the stop bit (1 is correct, 0 is bad),
  // and one bit time of idle line.
  task send_to_rx(input [7:0] b, input stop_bit);
    integer i;
    begin
      rx = 1'b0;
      repeat (CYCLES_PER_BIT) @(posedge clk);
      for (i = 0; i < 8; i = i + 1) begin
        rx = b[i];
        repeat (CYCLES_PER_BIT) @(posedge clk);
      end
      rx = stop_bit;
      repeat (CYCLES_PER_BIT) @(posedge clk);
      rx = 1'b1;
      repeat (CYCLES_PER_BIT) @(posedge clk);
    end
  endtask

  reg [7:0] got;

  initial begin
    $dumpvars(0, uart_tb);

    // ---- Transmitter ----

    repeat (5) @(posedge clk);
    `EXPECT_EQ(tx, 1'b1)        // The line is idle (high).
    `EXPECT_EQ(tx_ready, 1'b1)  // Ready to send.

    check_send(8'h55);  // Alternating 0s and 1s.
    check_send(8'h00);  // All 0s.
    check_send(8'hFF);  // All 1s.
    check_send(8'hA3);

    // tx_ready is 0 while a byte is sent (10 bits), and 1 again after.
    give_byte(8'h3C);
    `EXPECT_EQ(tx_ready, 1'b0)
    repeat (10 * CYCLES_PER_BIT) @(posedge clk);
    #1 `EXPECT_EQ(tx_ready, 1'b1)
    `EXPECT_EQ(tx, 1'b1)

    // With tx_valid held at 1, the next byte goes out right after the
    // first one's stop bit.
    @(negedge clk);
    tx_data = 8'h81;
    tx_valid = 1'b1;
    read_byte(got);
    `EXPECT_EQ(got, 8'h81)
    tx_data = 8'h7E;  // Sent when the first byte's stop bit ends.
    read_byte(got);
    tx_valid = 1'b0;
    `EXPECT_EQ(got, 8'h7E)
    repeat (12 * CYCLES_PER_BIT) @(posedge clk);

    // ---- Receiver ----

    send_to_rx("A", 1'b1);
    `EXPECT_EQ(bytes_received, 1)
    `EXPECT_EQ(last_byte, "A")
    `EXPECT_EQ(rx_data, "A")  // rx_data keeps it until the next byte.

    send_to_rx(8'h00, 1'b1);
    send_to_rx(8'hFF, 1'b1);
    send_to_rx(8'hC5, 1'b1);
    `EXPECT_EQ(bytes_received, 4)
    `EXPECT_EQ(last_byte, 8'hC5)
    `EXPECT_EQ(errors, 0)

    // A bad stop bit: rx_error, and the byte is dropped.
    send_to_rx(8'h5A, 1'b0);
    `EXPECT_EQ(errors, 1)
    `EXPECT_EQ(bytes_received, 4)

    // The next good byte is received normally.
    send_to_rx(8'h33, 1'b1);
    `EXPECT_EQ(bytes_received, 5)
    `EXPECT_EQ(last_byte, 8'h33)

    // A glitch shorter than half a bit isn't taken for a start bit.
    rx = 1'b0;
    repeat (CYCLES_PER_BIT / 4) @(posedge clk);
    rx = 1'b1;
    repeat (12 * CYCLES_PER_BIT) @(posedge clk);
    `EXPECT_EQ(bytes_received, 5)
    `EXPECT_EQ(errors, 1)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
