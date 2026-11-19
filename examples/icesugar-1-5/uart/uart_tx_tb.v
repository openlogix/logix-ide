// Testbench for uart_tx.v. Copy it along with uart_tx.v into any project.
//
// It reads the tx line as the other side would, and checks it against the
// serial format itself (start bit 0, 8 data bits least significant first,
// stop bit 1): the line is high when idle; each byte comes out bit by bit
// with the right timing; ready is 0 while a byte is being sent; with valid
// held at 1, bytes go out back to back.
//
// To keep the run short, it uses a fast baud rate: 1 Mbit/s from a 12 MHz
// clock, so a bit is 12 clock cycles. Run Tests checks it; Simulate shows
// the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module uart_tx_tb;

  localparam CYCLES_PER_BIT = 12;  // 12 MHz / 1 Mbit/s.

  reg clk = 1'b0;
  reg [7:0] data = 8'd0;
  reg valid = 1'b0;
  wire ready;
  wire tx;

  uart_tx #(.CLK_HZ(12_000_000), .BAUD(1_000_000)) dut (
    .clk(clk), .data(data), .valid(valid), .ready(ready), .tx(tx)
  );

  // A 12 MHz clock.
  always #41.667 clk = ~clk;

  // Gives a byte to the transmitter: waits for ready, then sets valid for
  // one clock cycle.
  task give_byte(input [7:0] b);
    begin
      @(negedge clk);
      while (!ready) @(negedge clk);
      data = b;
      valid = 1'b1;
      @(negedge clk);
      valid = 1'b0;
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

  reg [7:0] got;

  initial begin
    $dumpvars(0, uart_tx_tb);

    repeat (5) @(posedge clk);
    `EXPECT_EQ(tx, 1'b1)     // The line is idle (high).
    `EXPECT_EQ(ready, 1'b1)  // Ready to send.

    check_send(8'h55);  // Alternating 0s and 1s.
    check_send(8'h00);  // All 0s.
    check_send(8'hFF);  // All 1s.
    check_send("A");

    // ready is 0 while a byte is sent (10 bits), and 1 again after.
    give_byte(8'h3C);
    `EXPECT_EQ(ready, 1'b0)
    repeat (10 * CYCLES_PER_BIT) @(posedge clk);
    #1 `EXPECT_EQ(ready, 1'b1)
    `EXPECT_EQ(tx, 1'b1)

    // With valid held at 1, the next byte goes out right after the first
    // one's stop bit.
    @(negedge clk);
    data = 8'h81;
    valid = 1'b1;
    read_byte(got);
    `EXPECT_EQ(got, 8'h81)
    data = 8'h7E;  // Sent when the first byte's stop bit ends.
    read_byte(got);
    valid = 1'b0;
    `EXPECT_EQ(got, 8'h7E)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
