#!/bin/bash
ip link set tap0 nomaster
ip tuntap del tap0 mode tap
ip link set enx000ec6bd962d nomaster
ip link set down dev br0
ip link del br0
ip link set up dev enx000ec6bd962d
dhclient -v enx000ec6bd962d