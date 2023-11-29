#!/bin/bash
ip link add name br0 type bridge
ip addr flush dev enx000ec6bd962d
ip link set enx000ec6bd962d master br0
ip tuntap add tap0 mode tap
ip link set tap0 master br0
ip link set up dev enx000ec6bd962d
ip link set up dev tap0
ip link set up dev br0
dhclient -v br0