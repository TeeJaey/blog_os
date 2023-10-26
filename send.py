from scapy.all import Ether, IP, UDP, sendp

# Create an Ethernet frame with IP and TCP layers
frame = Ether(dst="62:a3:a1:ad:bf:c3") / "hello"

# Send the frame
sendp(frame)