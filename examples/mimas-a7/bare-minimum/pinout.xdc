# Constraint file of the bare-minimum example for the Mimas A7 (Xilinx, Artix7) board.
# It assigns the design's signals to FPGA pins, and lists only
# the pins that this example uses.

# Clock (100MHz).
set_property -dict { PACKAGE_PIN H4    IOSTANDARD LVCMOS33 } [get_ports {sys_clk}]
create_clock -period 10.000 [get_ports {sys_clk}]

# LEDs, active high.
set_property -dict { PACKAGE_PIN K17   IOSTANDARD LVCMOS33 } [get_ports {leds[0]}]
set_property -dict { PACKAGE_PIN J17   IOSTANDARD LVCMOS33 } [get_ports {leds[1]}]
set_property -dict { PACKAGE_PIN L14   IOSTANDARD LVCMOS33 } [get_ports {leds[2]}]
set_property -dict { PACKAGE_PIN L15   IOSTANDARD LVCMOS33 } [get_ports {leds[3]}]
set_property -dict { PACKAGE_PIN L16   IOSTANDARD LVCMOS33 } [get_ports {leds[4]}]
set_property -dict { PACKAGE_PIN K16   IOSTANDARD LVCMOS33 } [get_ports {leds[5]}]
set_property -dict { PACKAGE_PIN M15   IOSTANDARD LVCMOS33 } [get_ports {leds[6]}]
set_property -dict { PACKAGE_PIN M16   IOSTANDARD LVCMOS33 } [get_ports {leds[7]}]
