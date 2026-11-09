# Pins of Mimas A7 (Xilinx, Artix7) used by main.v.

# sys_clk: 100 MHz clock.
set_property -dict { PACKAGE_PIN H4 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 10.000 [get_ports {sys_clk}]

# leds: 8 LEDs (active high).
set_property -dict { PACKAGE_PIN K17 IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN J17 IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
set_property -dict { PACKAGE_PIN L14 IOSTANDARD LVCMOS33 } [get_ports {leds[2]}]
set_property -dict { PACKAGE_PIN L15 IOSTANDARD LVCMOS33 } [get_ports {leds[3]}]
set_property -dict { PACKAGE_PIN L16 IOSTANDARD LVCMOS33 } [get_ports {leds[4]}]
set_property -dict { PACKAGE_PIN K16 IOSTANDARD LVCMOS33 } [get_ports {leds[5]}]
set_property -dict { PACKAGE_PIN M15 IOSTANDARD LVCMOS33 } [get_ports {leds[6]}]
set_property -dict { PACKAGE_PIN M16 IOSTANDARD LVCMOS33 } [get_ports {leds[7]}]
