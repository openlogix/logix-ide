// Testbench for the Sipeed Tang Nano 20k's serial debug demo. It plays the
// computer: it reads the board's serial output and types to it. It checks
// that each count comes out as a line ending in a carriage return, to
// overwrite the last ("count: 0x0001", then "count: 0x0002"), that the LED
// shows the count's lowest bit, and that typing "4d" and Enter sets the
// counter (printed at once), which then counts on. To keep the run short,
// it uses a fast baud rate (1 Mbit/s, so a bit is 27 clock cycles) and a
// short tick (400 bit times instead of half a second). Run Tests checks it;
// Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module main_tb;

  localparam CYCLES_PER_BIT = 27;  // 27 MHz / 1 Mbit/s.

  reg sys_clk = 1'b0;
  reg uart_rx = 1'b1;   // The computer's line to the board, idle high.
  wire uart_tx;
  wire led;

  main #(.BAUD(1_000_000), .TICK_CYCLES(400 * CYCLES_PER_BIT)) dut (
    .sys_clk(sys_clk), .uart_tx(uart_tx), .uart_rx(uart_rx), .led(led)
  );

  // The board's 27 MHz clock.
  always #18.519 sys_clk = ~sys_clk;

  // Reads one character from the board, as the computer would: waits for
  // the start bit, then reads each bit in its middle.
  task read_char(output [7:0] c);
    integer i;
    begin
      @(negedge uart_tx);
      repeat (CYCLES_PER_BIT / 2) @(posedge sys_clk);
      `EXPECT_EQ(uart_tx, 1'b0)  // Start bit.
      for (i = 0; i < 8; i = i + 1) begin
        repeat (CYCLES_PER_BIT) @(posedge sys_clk);
        c[i] = uart_tx;          // Data bits, least significant first.
      end
      repeat (CYCLES_PER_BIT) @(posedge sys_clk);
      `EXPECT_EQ(uart_tx, 1'b1)  // Stop bit.
    end
  endtask

  // Types one character to the board: start bit, 8 data bits (least
  // significant first), stop bit.
  task type_char(input [7:0] c);
    integer i;
    begin
      uart_rx = 1'b0;
      repeat (CYCLES_PER_BIT) @(posedge sys_clk);
      for (i = 0; i < 8; i = i + 1) begin
        uart_rx = c[i];
        repeat (CYCLES_PER_BIT) @(posedge sys_clk);
      end
      uart_rx = 1'b1;
      repeat (CYCLES_PER_BIT) @(posedge sys_clk);
    end
  endtask

  // Reads a line of 14 characters ("count: 0x0001" and a carriage return)
  // and checks it.
  task check_line(input [8*14-1:0] expected);
    reg [8*14-1:0] text;
    reg [7:0] c;
    integer i;
    begin
      text = 0;
      for (i = 0; i < 14; i = i + 1) begin
        read_char(c);
        text = {text[8*13-1:0], c};
      end
      `EXPECT_EQ(text, expected)
    end
  endtask

  initial begin
    $dumpvars(0, main_tb);

    repeat (2) @(posedge sys_clk);
    `EXPECT_EQ(uart_tx, 1'b1)  // The line is idle (high).
    `EXPECT_EQ(led, ~1'b0)     // The count starts at 0.

    // Each tick, the new count comes out as a line, and on the LED.
    check_line({"count: 0x0001", 8'h0D});
    `EXPECT_EQ(led, ~1'b1)
    check_line({"count: 0x0002", 8'h0D});
    `EXPECT_EQ(led, ~1'b0)

    // Typing "4d" and Enter sets the counter, printed at once.
    fork
      begin
        type_char("4");
        type_char("d");
        type_char(8'h0D);
      end
      check_line({"count: 0x004D", 8'h0D});
    join
    `EXPECT_EQ(led, ~1'b1)
    // Then it counts on from there.
    check_line({"count: 0x004E", 8'h0D});

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
