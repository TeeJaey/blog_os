from scapy.all import Ether, sendp

# Ethernet frame details
dst_mac = "ff:ff:ff:ff:ff:ff" # destination MAC address
src_mac = "00:0e:c6:bd:96:2d" # source MAC address
ether_type = 0x1122          # EtherType

# Creating the Ethernet frame
frame = Ether(dst=dst_mac, src=src_mac, type=ether_type)

# Sending the Ethernet frame
sendp(frame, iface="enx000ec6bd962d")