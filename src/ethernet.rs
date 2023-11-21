#![allow(dead_code)]

use crate::rtl8139;
use alloc::vec::Vec;
use x86_64::VirtAddr;
use core::mem::size_of_val;

#[derive(Debug)]
#[repr(C)]
pub struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    protocol: u16,
    payload: Vec<u8>
}

impl EthernetFrame {
    fn new(
        dst_mac: [u8; 6], 
        src_mac: [u8; 6], 
        protocol: u16, 
        payload: Vec<u8>
    ) -> Self {
        Self {dst_mac, src_mac, protocol, payload}
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        result.extend_from_slice(&self.dst_mac);
        result.extend_from_slice(&self.src_mac);
        let protocol_upper = ((self.protocol >> 8) & 0xff) as u8;
        let protocol_lower = (self.protocol & 0xff) as u8;
        result.push(protocol_upper);
        result.push(protocol_lower);
        result.extend(self.payload.iter());

        result
    }
}

fn send_frame(frame: EthernetFrame) {

    let len = size_of_val(&frame);

    let mut buffer: Vec<u8> = Vec::with_capacity(len);
    buffer.append(&mut frame.to_bytes());
    let buffer_virt_addr = VirtAddr::new(buffer.as_mut_ptr() as u64);
    
    rtl8139::send_packet(buffer_virt_addr, len as u32)
}

pub fn send_empty_frame() {
    let dst_mac: [u8; 6] = [0xff; 6];
    let src_mac: [u8; 6] = rtl8139::get_mac_address();
    let protocol = 0x0800; // IP-Ethernet-type
    let payload = Vec::new();

    let empty_frame = EthernetFrame::new(dst_mac, src_mac, protocol, payload);

    send_frame(empty_frame);
}