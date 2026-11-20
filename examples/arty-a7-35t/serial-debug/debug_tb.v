// Testbench for debug.v. Copy it along with debug.v (and uart.v) into any
// project.
//
// It plays the computer, on the serial lines, as the Serial Monitor would.
// Watch: a print sends "count: 0x002A" with a carriage return and a line
// feed; the line has the value as it was when print came; a print while
// sending is ignored; the hexadecimal digits A to F come out as capitals;
// with OVERWRITE, a line ends with a carriage return only. Set: "4d" and
// Enter gives 0x004D with typed for one cycle; capitals work; a carriage
// return and a line feed together give the value once; Enter alone gives
// nothing; another character drops what was typed; more digits than
// DIGITS keep the last ones.
//
// To keep the run short, it uses a fast baud rate (1 Mbit/s, so a bit is
// 12 clock cycles). Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module debug_tb;

  localparam CYCLES_PER_BIT = 12;  // 12 MHz / 1 Mbit/s.

  reg clk = 1'b0;
  reg uart_rx = 1'b1;  // The computer's line to the board, idle high.
  wire uart_tx;
  reg print = 1'b0;
  reg [15:0] value = 16'h002A;
  wire [15:0] typed_value;
  wire typed;

  debug #(
    .CLK_HZ(12_000_000), .BAUD(1_000_000),
    .LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4)
  ) dut (
    .clk(clk),
    .uart_tx(uart_tx), .uart_rx(uart_rx),
    .print(print), .value(value),
    .typed_value(typed_value), .typed(typed)
  );

  // The same with OVERWRITE, on its own TX line.
  wire uart_tx2;
  debug #(
    .CLK_HZ(12_000_000), .BAUD(1_000_000),
    .LABEL("v"), .LABEL_LENGTH(1), .DIGITS(2), .OVERWRITE(1)
  ) dut2 (
    .clk(clk),
    .uart_tx(uart_tx2), .uart_rx(1'b1),
    .print(print), .value(value[7:0]),
    .typed_value(), .typed()
  );

  // A 12 MHz clock.
  always #41.667 clk = ~clk;

  // Counts the typed pulses.
  integer values = 0;
  always @(posedge clk) if (typed) values = values + 1;

  // Which TX line read_char reads: 0 for dut, 1 for dut2.
  reg second = 1'b0;
  wire line = second ? uart_tx2 : uart_tx;

  // Reads one character from the board: waits for the start bit, then
  // reads each bit in its middle.
  task read_char(output [7:0] c);
    integer i;
    begin
      @(negedge line);
      repeat (CYCLES_PER_BIT / 2) @(posedge clk);
      `EXPECT_EQ(line, 1'b0)  // Start bit.
      for (i = 0; i < 8; i = i + 1) begin
        repeat (CYCLES_PER_BIT) @(posedge clk);
        c[i] = line;          // Data bits, least significant first.
      end
      repeat (CYCLES_PER_BIT) @(posedge clk);
      `EXPECT_EQ(line, 1'b1)  // Stop bit.
    end
  endtask

  // Reads a line of 15 characters ("count: 0x002A", a carriage return and
  // a line feed) and checks it.
  task check_line(input [8*15-1:0] expected);
    reg [8*15-1:0] text;
    reg [7:0] c;
    integer i;
    begin
      text = 0;
      for (i = 0; i < 15; i = i + 1) begin
        read_char(c);
        text = {text[8*14-1:0], c};
      end
      `EXPECT_EQ(text, expected)
    end
  endtask

  // Types one character to the board: start bit, 8 data bits (least
  // significant first), stop bit.
  task type_char(input [7:0] c);
    integer i;
    begin
      uart_rx = 1'b0;
      repeat (CYCLES_PER_BIT) @(posedge clk);
      for (i = 0; i < 8; i = i + 1) begin
        uart_rx = c[i];
        repeat (CYCLES_PER_BIT) @(posedge clk);
      end
      uart_rx = 1'b1;
      repeat (CYCLES_PER_BIT * 2) @(posedge clk);
    end
  endtask

  // Pulses print for one cycle.
  task print_now;
    begin
      @(negedge clk);
      print = 1'b1;
      @(negedge clk);
      print = 1'b0;
    end
  endtask

  reg [8*8-1:0] text2;
  reg [7:0] c;
  integer i;

  initial begin
    $dumpvars(0, debug_tb);

    repeat (3) @(posedge clk);
    `EXPECT_EQ(uart_tx, 1'b1)  // The line is idle (high).

    // ---- Watch ----

    // A line, with the value of the moment print came: changing it while
    // the line is sent doesn't change the line. A second print while it's
    // sent is ignored.
    fork
      check_line({"count: 0x002A", 8'h0D, 8'h0A});
      begin
        print_now;
        value = 16'h1234;
        repeat (100) @(posedge clk);
        print_now;  // Ignored.
      end
    join
    // Nothing more comes: the second print was ignored.
    repeat (CYCLES_PER_BIT * 20) @(posedge clk);
    `EXPECT_EQ(uart_tx, 1'b1)

    // The digits A to F, as capitals.
    value = 16'hBEEF;
    fork
      check_line({"count: 0xBEEF", 8'h0D, 8'h0A});
      print_now;
    join

    // With OVERWRITE (dut2, which printed each time too): "v: 0x" and the
    // digits, then a carriage return only.
    second = 1'b1;
    value = 16'h00C3;
    fork
      begin
        for (i = 0; i < 8; i = i + 1) begin
          read_char(c);
          text2 = {text2[8*7-1:0], c};
        end
      end
      print_now;
    join
    `EXPECT_EQ(text2, {"v: 0xC3", 8'h0D})
    second = 1'b0;
    repeat (CYCLES_PER_BIT * 200) @(posedge clk);  // dut's line is done too.

    // ---- Set ----

    // "4d" and Enter: 0x004D, once.
    type_char("4");
    type_char("d");
    `EXPECT_EQ(values, 0)   // Nothing before Enter.
    type_char(8'h0D);
    `EXPECT_EQ(values, 1)
    `EXPECT_EQ(typed_value, 16'h004D)

    // Capitals, and a carriage return with a line feed: given once.
    type_char("A");
    type_char("F");
    type_char(8'h0D);
    type_char(8'h0A);
    `EXPECT_EQ(values, 2)
    `EXPECT_EQ(typed_value, 16'h00AF)

    // Enter alone gives nothing.
    type_char(8'h0A);
    `EXPECT_EQ(values, 2)

    // Another character drops what was typed.
    type_char("1");
    type_char("x");
    type_char(8'h0D);
    `EXPECT_EQ(values, 2)
    `EXPECT_EQ(typed_value, 16'h00AF)  // Unchanged.

    // More digits than DIGITS: the last ones count.
    type_char("1");
    type_char("2");
    type_char("3");
    type_char("4");
    type_char("5");
    type_char(8'h0A);
    `EXPECT_EQ(values, 3)
    `EXPECT_EQ(typed_value, 16'h2345)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
