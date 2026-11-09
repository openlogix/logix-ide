# Pins of Zybo Z7-20 (Xilinx, Zynq7 PL) used by main.v.

# sys_clk: 125 MHz clock.
set_property -dict { PACKAGE_PIN K17 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 8.000 [get_ports {sys_clk}]

# leds: 4 LEDs (active high).
set_property -dict { PACKAGE_PIN M14 IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN M15 IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
set_property -dict { PACKAGE_PIN G14 IOSTANDARD LVCMOS33 } [get_ports {leds[2]}]
set_property -dict { PACKAGE_PIN D18 IOSTANDARD LVCMOS33 } [get_ports {leds[3]}]
