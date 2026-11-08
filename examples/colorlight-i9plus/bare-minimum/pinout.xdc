# Constraint file of the bare-minimum example for the Colorlight i9+ (Xilinx, Artix7) board.
# It assigns the design's signals to FPGA pins, and lists only
# the pins that this example uses.

# Clock (25MHz).
set_property -dict { PACKAGE_PIN K4    IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 40.000 [get_ports {sys_clk}]

# LED, active high.
set_property -dict { PACKAGE_PIN A18   IOSTANDARD LVCMOS33 } [get_ports {led}]
