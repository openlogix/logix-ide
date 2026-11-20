// Testbench for debug.v: debug_print and debug_input. Copy it along with
// debug.v into any project.
//
// It checks each module on its own, standing in for the UART modules:
//
//   debug_print (it stands in for uart_tx, taking each character when
//   tx_ready is 1): the line "count: 0x002A" with a carriage return and a
//   line feed; the value as it was when print came; a print while sending
//   is ignored; the hexadecimal digits A to F; with OVERWRITE, a carriage
//   return only.
//
//   debug_input (it stands in for uart_rx, giving characters with
//   rx_valid): "4d" and Enter gives 0x4D with valid for one cycle; capitals
//   work; a carriage return and a line feed together give the value once;
//   Enter alone gives nothing; another character drops what was typed; more
//   digits than DIGITS keep the last ones.
//
// Run Tests checks it; Simulate shows the signals.

`timescale 1ns / 1ps

// Reports an error, with both values, when actual_val isn't expected_val.
`define EXPECT_EQ(actual_val, expected_val) if ((actual_val) !== (expected_val)) $error("expected %h, got %h", (expected_val), (actual_val));

module debug_tb;

  reg clk = 1'b0;

  // A 12 MHz clock.
  always #41.667 clk = ~clk;

  // ---------------------------------------------------------------------
  // debug_print
  // ---------------------------------------------------------------------

  reg print = 1'b0;
  reg [15:0] print_value = 16'h002A;
  wire busy;
  wire [7:0] tx_data;
  wire tx_valid;
  reg tx_ready = 1'b1;

  debug_print #(.LABEL("count"), .LABEL_LENGTH(5), .DIGITS(4)) printer (
    .clk(clk), .print(print), .value(print_value), .busy(busy),
    .tx_data(tx_data), .tx_valid(tx_valid), .tx_ready(tx_ready)
  );

  // Stands in for uart_tx: takes a character on each clock edge where
  // tx_valid and tx_ready are 1, then is busy (tx_ready 0) for 3 cycles,
  // and collects the characters.
  reg [8*20-1:0] line = 0;
  integer chars = 0;
  always @(posedge clk) begin
    if (tx_valid && tx_ready) begin
      line <= {line[8*19-1:0], tx_data};
      chars <= chars + 1;
      tx_ready <= 1'b0;
      repeat (3) @(posedge clk);
      tx_ready <= 1'b1;
    end
  end

  // The same with OVERWRITE, sending to its own stand-in, always ready.
  wire busy2;
  wire [7:0] tx_data2;
  wire tx_valid2;
  debug_print #(.LABEL("v"), .LABEL_LENGTH(1), .DIGITS(2), .OVERWRITE(1)) printer2 (
    .clk(clk), .print(print), .value(print_value[7:0]), .busy(busy2),
    .tx_data(tx_data2), .tx_valid(tx_valid2), .tx_ready(1'b1)
  );
  reg [8*8-1:0] line2 = 0;
  integer chars2 = 0;
  always @(posedge clk) begin
    if (tx_valid2) begin
      line2 <= {line2[8*7-1:0], tx_data2};
      chars2 <= chars2 + 1;
    end
  end

  // Pulses print for one cycle.
  task print_now;
    begin
      @(negedge clk);
      print = 1'b1;
      @(negedge clk);
      print = 1'b0;
    end
  endtask

  // Waits until the line is sent, then checks it (15 characters).
  task expect_line(input [8*15-1:0] expected);
    begin
      @(negedge clk);
      while (busy) @(negedge clk);
      repeat (5) @(negedge clk);
      `EXPECT_EQ(chars, 15)
      `EXPECT_EQ(line[8*15-1:0], expected)
      chars = 0;
    end
  endtask

  // ---------------------------------------------------------------------
  // debug_input
  // ---------------------------------------------------------------------

  reg [7:0] rx_data = 8'd0;
  reg rx_valid = 1'b0;
  wire [7:0] typed_value;
  wire typed_valid;

  debug_input #(.DIGITS(2)) command (
    .clk(clk), .rx_data(rx_data), .rx_valid(rx_valid),
    .value(typed_value), .valid(typed_valid)
  );

  // Counts the valid pulses.
  integer values = 0;
  always @(posedge clk) if (typed_valid) values = values + 1;

  // Gives one character, as uart_rx would: rx_valid for one cycle.
  task give(input [7:0] c);
    begin
      @(negedge clk);
      rx_data = c;
      rx_valid = 1'b1;
      @(negedge clk);
      rx_valid = 1'b0;
      repeat (3) @(negedge clk);
    end
  endtask

  // ---------------------------------------------------------------------
  // The checks
  // ---------------------------------------------------------------------

  initial begin
    $dumpvars(0, debug_tb);

    // ---- debug_print ----

    repeat (3) @(negedge clk);
    `EXPECT_EQ(busy, 1'b0)    // Nothing to send yet.
    `EXPECT_EQ(tx_valid, 1'b0)

    // A line, with the value of the moment print came: changing it while
    // the line is sent doesn't change the line.
    print_now;
    `EXPECT_EQ(busy, 1'b1)
    print_value = 16'h1234;
    expect_line({"count: 0x002A", 8'h0D, 8'h0A});

    // A print while a line is being sent is ignored.
    print_now;
    repeat (4) @(negedge clk);
    print_now;
    expect_line({"count: 0x1234", 8'h0D, 8'h0A});

    // The digits A to F.
    print_value = 16'hBEEF;
    print_now;
    expect_line({"count: 0xBEEF", 8'h0D, 8'h0A});

    // With OVERWRITE: "v: 0x" and the digits, then a carriage return only.
    chars2 = 0;
    print_value = 16'h00C3;
    print_now;
    repeat (20) @(negedge clk);
    `EXPECT_EQ(chars2, 8)
    `EXPECT_EQ(line2[8*8-1:0], {"v: 0xC3", 8'h0D})

    // ---- debug_input ----

    // "4d" and Enter: 0x4D, once.
    give("4");
    give("d");
    `EXPECT_EQ(values, 0)   // Nothing before Enter.
    give(8'h0D);
    `EXPECT_EQ(values, 1)
    `EXPECT_EQ(typed_value, 8'h4D)

    // Capitals, and a carriage return with a line feed: given once.
    give("A");
    give("F");
    give(8'h0D);
    give(8'h0A);
    `EXPECT_EQ(values, 2)
    `EXPECT_EQ(typed_value, 8'hAF)

    // Enter alone gives nothing.
    give(8'h0A);
    `EXPECT_EQ(values, 2)

    // Another character drops what was typed.
    give("1");
    give("x");
    give(8'h0D);
    `EXPECT_EQ(values, 2)
    `EXPECT_EQ(typed_value, 8'hAF)  // Unchanged.

    // More digits than DIGITS: the last ones count.
    give("1");
    give("2");
    give("3");
    give(8'h0A);
    `EXPECT_EQ(values, 3)
    `EXPECT_EQ(typed_value, 8'h23)

    // Add your own checks here.

    $display("End of simulation");
    $finish;
  end

endmodule
