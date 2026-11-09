# Pins of MicroPhase A7-Lite (Xilinx, Artix7) used by main.v.

# sys_clk: 50 MHz clock.
set_property -dict { PACKAGE_PIN J19 IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 20.000 [get_ports {sys_clk}]

# leds_n: 2 LEDs (active low).
set_property -dict { PACKAGE_PIN M18 IOSTANDARD LVCMOS33 } [get_ports {leds_n[0]}]
set_property -dict { PACKAGE_PIN N18 IOSTANDARD LVCMOS33 } [get_ports {leds_n[1]}]
