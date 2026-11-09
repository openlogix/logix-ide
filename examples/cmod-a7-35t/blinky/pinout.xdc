# Pins of Cmod A7-35T (Xilinx, Artix7) used by main.v.

# sys_clk: 12 MHz clock.
set_property -dict { PACKAGE_PIN L17 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 83.333 [get_ports {sys_clk}]

# leds: 2 LEDs (active high).
set_property -dict { PACKAGE_PIN A17 IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN C16 IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
