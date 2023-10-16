#![allow(dead_code)]

use core::mem::size_of;
use crate::rtl8139;
use x86_64::VirtAddr;

#[derive(Debug)]
pub struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ethertype: u16,
    payload: [u8; 18]
}

impl EthernetFrame {
    fn new(dst_mac: [u8; 6], src_mac: [u8; 6], ethertype: u16, payload: [u8; 18]) -> Self {
        Self {dst_mac, src_mac, ethertype, payload}
    }
}

fn eth_send_packet(dst_mac: [u8; 6], protocol: u16, payload: [u8; 18], physical_memory_offset: VirtAddr) {
    let len = size_of::<EthernetFrame>();
    let src_mac = rtl8139::get_mac_address();
    let frame = EthernetFrame::new(dst_mac, src_mac, protocol, payload);
    
    let frame_virt_addr = VirtAddr::from_ptr(&frame);
    
    rtl8139::rtl_send_packet(frame_virt_addr, len, physical_memory_offset)
}

pub fn send_empty_frame(physical_memory_offset: VirtAddr) {

    let dst_mac: [u8; 6] = [0x2C,0x56,0xDC,0x3A,0x38,0x66];
    let protocol = 0x0800; // IP-Ethernet-type
    let payload: [u8; 18] = [1; 18];
    eth_send_packet(dst_mac, protocol, payload, physical_memory_offset);
}