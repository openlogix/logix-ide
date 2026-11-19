// Testbench for the Icesugar Pro's serial port demo. It plays the computer's
// part: it reads what the board sends and types characters to it. It checks
// that:
//   - the greeting comes after a quiet time, and is complete;
//   - each character typed is echoed back and shown on the LED;
//   - typing starts the quiet time over, so the greeting waits;
//   - after typing, the greeting starts on a new line.
//
// To keep the run short, it uses a fast baud rate (1 Mbit/s, so a bit is 25
// clock cycles) and a short quiet time (500 bits instead of 5 seconds).
// Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module main_tb;

  localparam CYCLES_PER_BIT = 25;  // 25 MHz / 1 Mbit/s.
  localparam QUIET = 500 * CYCLES_PER_BIT;  // Quiet time before a greeting: 500 bits.

  reg sys_clk = 1'b0;
  reg uart_rx = 1'b1;   // The computer's TX line, idle high.
  wire uart_tx;
  wire led;

  main #(.BAUD(1_000_000), .QUIET_CYCLES(QUIET)) dut (
    .sys_clk(sys_clk), .uart_tx(uart_tx), .uart_rx(uart_rx), .led(led)
  );

  // The board's 25 MHz clock, and a count of its cycles to measure time.
  always #20.000 sys_clk = ~sys_clk;
  integer cycle = 0;
  always @(posedge sys_clk) cycle = cycle + 1;

  // Reads one character from the board, as the computer would: waits for
  // the start bit, then reads each bit in its middle. Also returns the
  // cycle its start bit began.
  task read_char(output [7:0] c, output integer started);
    integer i;
    begin
      @(negedge uart_tx);
      started = cycle;
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

  // Types a character and reads its echo at the same time.
  task type_and_check_echo(input [7:0] c);
    reg [7:0] echoed;
    integer started;
    begin
      fork
        type_char(c);
        read_char(echoed, started);
      join
      `EXPECT_EQ(echoed, c)
      `EXPECT_EQ(led, ~c[0])  // The LED shows its lowest bit, lit by a 0.
    end
  endtask

  // The greeting, and the same after typing: on a new line (a carriage
  // return and a line feed, \015\012, before it).
  localparam LENGTH = 28;
  localparam [8*LENGTH-1:0] GREETING = "Hello from your FPGA board\015\012";
  localparam [8*(LENGTH+2)-1:0] NEW_LINE_GREETING = {"\015\012", GREETING};

  // Reads a whole greeting and checks it: on a new line or not. Returns the
  // cycle it began.
  task check_greeting(input new_line, output integer started);
    reg [8*(LENGTH+2)-1:0] text;
    reg [7:0] c;
    integer i;
    integer char_started;
    begin
      text = 0;
      for (i = 0; i < (new_line ? LENGTH + 2 : LENGTH); i = i + 1) begin
        read_char(c, char_started);
        if (i == 0) started = char_started;
        text = {text[8*(LENGTH+2)-9:0], c};
      end
      if (new_line) begin
        `EXPECT_EQ(text, NEW_LINE_GREETING)
      end else begin
        `EXPECT_EQ(text[8*LENGTH-1:0], GREETING)
      end
    end
  endtask

  integer started;
  integer typed;

  initial begin
    $dumpvars(0, main_tb);

    repeat (2) @(posedge sys_clk);
    `EXPECT_EQ(uart_tx, 1'b1)  // The line is idle (high).
    `EXPECT_EQ(led, 1'b1)      // Nothing received yet: the LED is off.

    // After the quiet time, the greeting comes, complete (nothing typed
    // yet, so no new line first).
    check_greeting(1'b0, started);
    `EXPECT_EQ(started >= QUIET, 1'b1)

    // Each character is echoed and shown on the LED.
    type_and_check_echo("A");

    // Typing again before the quiet time is up starts it over: the next
    // thing the board sends is the echo of "B", not the greeting.
    repeat (QUIET / 2) @(posedge sys_clk);
    typed = cycle;
    type_and_check_echo("B");

    // The greeting comes one quiet time after "B" arrived: at least QUIET
    // cycles after typing it began, and at most one character (10 bits)
    // later than that. It starts on a new line, after the echoed "AB".
    check_greeting(1'b1, started);
    `EXPECT_EQ(started - typed >= QUIET, 1'b1)
    `EXPECT_EQ(started - typed <= QUIET + 10 * CYCLES_PER_BIT, 1'b1)

    // With nothing typed since, the next greeting has no new line first.
    check_greeting(1'b0, started);

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
