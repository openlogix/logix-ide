// Testbench for uart_rx.v. Copy it along with uart_rx.v into any project.
//
// It drives the rx line as the other side would, in the serial format
// itself (start bit 0, 8 data bits least significant first, stop bit 1),
// and checks that: each byte comes out on data, with valid for one cycle;
// a byte with a bad stop bit gives error instead, and the next good byte
// is received normally; a short glitch on the line is ignored.
//
// To keep the run short, it uses a fast baud rate: 1 Mbit/s from a 12 MHz
// clock, so a bit is 12 clock cycles. Run Tests checks it; Simulate shows
// the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module uart_rx_tb;

  localparam CYCLES_PER_BIT = 12;  // 12 MHz / 1 Mbit/s.

  reg clk = 1'b0;
  reg rx = 1'b1;  // Idle high.
  wire [7:0] data;
  wire valid;
  wire error;

  uart_rx #(.CLK_HZ(12_000_000), .BAUD(1_000_000)) dut (
    .clk(clk), .rx(rx), .data(data), .valid(valid), .error(error)
  );

  // A 12 MHz clock.
  always #41.667 clk = ~clk;

  // Counts the valid and error pulses, and keeps the last byte received.
  integer bytes_received = 0;
  integer errors = 0;
  reg [7:0] last_byte = 8'd0;
  always @(posedge clk) begin
    if (valid) begin
      bytes_received = bytes_received + 1;
      last_byte = data;
    end
    if (error) errors = errors + 1;
  end

  // Sends a byte on rx, as the other side would: start bit, 8 data bits
  // (least significant first), then the stop bit (1 is correct, 0 is bad),
  // and one bit time of idle line.
  task send_byte(input [7:0] b, input stop_bit);
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

  initial begin
    $dumpvars(0, uart_rx_tb);

    repeat (5) @(posedge clk);
    `EXPECT_EQ(valid, 1'b0)  // Nothing received yet.
    `EXPECT_EQ(error, 1'b0)

    send_byte("A", 1'b1);
    `EXPECT_EQ(bytes_received, 1)
    `EXPECT_EQ(last_byte, "A")
    `EXPECT_EQ(data, "A")  // data keeps it until the next byte.

    send_byte(8'h00, 1'b1);  // All 0s.
    send_byte(8'hFF, 1'b1);  // All 1s.
    send_byte(8'hC5, 1'b1);
    `EXPECT_EQ(bytes_received, 4)
    `EXPECT_EQ(last_byte, 8'hC5)
    `EXPECT_EQ(errors, 0)

    // A bad stop bit: error, and the byte is dropped.
    send_byte(8'h5A, 1'b0);
    `EXPECT_EQ(errors, 1)
    `EXPECT_EQ(bytes_received, 4)

    // The next good byte is received normally.
    send_byte(8'h33, 1'b1);
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
