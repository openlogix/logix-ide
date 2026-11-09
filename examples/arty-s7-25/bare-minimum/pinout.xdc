# Pins of Arty S7-25 (Xilinx, Spartan7) used by main.v.

# sys_clk: 12 MHz clock.
set_property -dict { PACKAGE_PIN F14 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 83.333 [get_ports {sys_clk}]

# leds: 4 LEDs (active high).
set_property -dict { PACKAGE_PIN E18 IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN F13 IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
set_property -dict { PACKAGE_PIN E13 IOSTANDARD LVCMOS33 } [get_ports {leds[2]}]
set_property -dict { PACKAGE_PIN H15 IOSTANDARD LVCMOS33 } [get_ports {leds[3]}]
