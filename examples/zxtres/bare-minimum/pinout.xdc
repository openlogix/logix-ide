# Constraint file of the bare-minimum example for the ZXTRES (Xilinx, Artix7) board.
# It assigns the design's signals to FPGA pins, and lists only
# the pins that this example uses.

# Clock (50MHz).
set_property -dict { PACKAGE_PIN Y18   IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 20.000 [get_ports {sys_clk}]

# LED, active low.
set_property -dict { PACKAGE_PIN H13   IOSTANDARD LVCMOS33 } [get_ports {led}]
