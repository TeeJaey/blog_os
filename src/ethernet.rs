#![allow(dead_code)]

use core::mem::{size_of, transmute};
use crate::rtl8139::{self, rtl_receive_packet};
use alloc::vec::Vec;
use x86_64::VirtAddr;

#[derive(Debug)]
#[repr(C, align(1))]
pub struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ethertype: u16,
    payload: Vec<u8>
}

impl EthernetFrame {
    fn new(dst_mac: [u8; 6], src_mac: [u8; 6], ethertype: u16, payload: Vec<u8>) -> Self {
        Self {dst_mac, src_mac, ethertype, payload}
    }
}

fn eth_send_packet(dst_mac: [u8; 6], protocol: u16, payload: Vec<u8>) {
    const LEN: usize = size_of::<EthernetFrame>();

    let src_mac = rtl8139::get_mac_address();
    let frame = EthernetFrame::new(dst_mac, src_mac, protocol, payload);
    
    let buffer: [u8; LEN] = unsafe {
        transmute(frame)
    };
    let frame_virt_addr = VirtAddr::from_ptr(&buffer);
    
    rtl8139::rtl_send_packet(frame_virt_addr, LEN as u32)
}

pub fn send_empty_frame() {
    let dst_mac: [u8; 6] = [0xff; 6];
    let protocol = 0x0800; // IP-Ethernet-type
    let payload = Vec::new();
    eth_send_packet(dst_mac, protocol, payload);
}

pub fn wait_for_receive() {
    rtl_receive_packet()
}