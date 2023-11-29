from scapy.all import Ether, sendp
import psutil

def get_interfaces():
    interfaces = []
    print("Following Network Interfaces available:")
    for i, iface in enumerate(psutil.net_if_addrs().keys(), start=1):
        print(f"{i}. {iface}")
        interfaces.append(iface)
    return interfaces

def choose_interface(interfaces):
    try:
        choice = int(input("Please enter the number of the network interface you want to use: "))
        if choice <= 0 or choice > len(interfaces):
            print("Invalid number, please choose a number between 1 and", len(interfaces))
            return choose_interface(interfaces)
        else:
            return interfaces[choice-1]
    except ValueError:
        print("Invalid input, please enter a number.")
        return choose_interface(interfaces)
    
def get_mac_address(interface):
    # get the mac address of the interface
    mac_address = psutil.net_if_addrs()[interface][-1].address
    return mac_address

interfaces = get_interfaces()
interface = choose_interface(interfaces)

print("Chosen interface: ", interface)

# Ethernet frame details
dst_mac = "ff:ff:ff:ff:ff:ff" # destination MAC address
src_mac = get_mac_address(interface) # source MAC address
ether_type = 0x1234          # EtherType

try:
    num_frames = int(input("Please enter the number of frames you want to send: "))
    if num_frames <= 0:
        print("Invalid number, please enter a number greater than 0.")
        num_frames = int(input("Please enter the number of frames you want to send: "))
except ValueError:
    print("Invalid input, please enter a number.")
    num_frames = int(input("Please enter the number of frames you want to send: "))

# Creating that many frames and sending them at the end
frames = [Ether(dst=dst_mac, src=src_mac, type=ether_type) for _ in range(num_frames)]
print("Sending following frames: ", frames)
sendp(frames, iface=interface)