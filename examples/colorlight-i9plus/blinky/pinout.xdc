# Pins of Colorlight i9+ (Xilinx, Artix7) used by main.v.

# sys_clk: 25 MHz clock.
set_property -dict { PACKAGE_PIN K4 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 40.000 [get_ports {sys_clk}]

# led: LED (active high).
set_property -dict { PACKAGE_PIN A18 IOSTANDARD LVCMOS33 } [get_ports {led}]
