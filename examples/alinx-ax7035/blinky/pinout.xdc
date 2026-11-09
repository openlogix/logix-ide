# Pins of Alinx ARTIX-7 AX7035 used by main.v.

# sys_clk: 50 MHz clock.
set_property -dict { PACKAGE_PIN Y18 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 20.000 [get_ports {sys_clk}]

# leds_n: 4 LEDs (active low).
set_property -dict { PACKAGE_PIN F19 IOSTANDARD LVCMOS33 } [get_ports {leds_n[0]}]
set_property -dict { PACKAGE_PIN E21 IOSTANDARD LVCMOS33 } [get_ports {leds_n[1]}]
set_property -dict { PACKAGE_PIN D20 IOSTANDARD LVCMOS33 } [get_ports {leds_n[2]}]
set_property -dict { PACKAGE_PIN C20 IOSTANDARD LVCMOS33 } [get_ports {leds_n[3]}]
