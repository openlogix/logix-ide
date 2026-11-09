# Pins of Arty A7-35T (Xilinx, Artix7) used by main.v.

# sys_clk: 100 MHz clock.
set_property -dict { PACKAGE_PIN E3 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 10.000 [get_ports {sys_clk}]

# leds: 4 LEDs (active high).
set_property -dict { PACKAGE_PIN H5 IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN J5 IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
set_property -dict { PACKAGE_PIN T9 IOSTANDARD LVCMOS33 } [get_ports {leds[2]}]
set_property -dict { PACKAGE_PIN T10 IOSTANDARD LVCMOS33 } [get_ports {leds[3]}]
