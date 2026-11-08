# Constraint file of the blinky example for the Cmod A7-35T (Xilinx, Artix7) board.
# It assigns the design's signals to FPGA pins, and lists only
# the pins that this example uses.

# Clock (12MHz).
set_property -dict { PACKAGE_PIN L17   IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 83.333 [get_ports {sys_clk}]

# LEDs, active high.
set_property -dict { PACKAGE_PIN A17   IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN C16   IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
