# Pins of ZXTRES+ (Xilinx, Artix7) used by main.v.

# sys_clk: 50 MHz clock.
set_property -dict { PACKAGE_PIN Y18 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 20.000 [get_ports {sys_clk}]

# led: LED (active low).
set_property -dict { PACKAGE_PIN H13 IOSTANDARD LVCMOS33 } [get_ports {led}]
