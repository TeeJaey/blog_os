#![allow(dead_code)]

use crate::rtl8139;
use alloc::vec::Vec;
use x86_64::VirtAddr;
use core::mem::size_of_val;

#[derive(Debug)]
#[repr(C)]
pub struct EthernetHeader {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    protocol: u16,
}

impl EthernetHeader {
    fn new(
        dst_mac: [u8; 6], 
        src_mac: [u8; 6], 
        protocol: u16
    ) -> Self {
        Self {dst_mac, src_mac, protocol}
    }
}


#[derive(Debug)]
#[repr(C)]
pub struct EthernetFrame {
    header: EthernetHeader,
    payload: Vec<u8>
}
impl EthernetFrame {
    fn new(
        header: EthernetHeader, 
        payload: Vec<u8>
    ) -> Self {
        Self {header, payload}
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();
        result.extend_from_slice(&self.header.dst_mac);
        result.extend_from_slice(&self.header.src_mac);
        let protocol_upper = ((self.header.protocol >> 8) & 0xff) as u8;
        let protocol_lower = (self.header.protocol & 0xff) as u8;
        result.push(protocol_upper);
        result.push(protocol_lower);
        result.extend(self.payload.iter());

        result
    }
}

fn send_frame(frame: EthernetFrame) {

    let mut buffer: Vec<u8> = Vec::with_capacity(size_of_val(&frame));

    buffer.append(&mut frame.to_bytes());

    let buffer_virt_addr = VirtAddr::new(buffer.as_mut_ptr() as u64);
    
    rtl8139::send_packet(buffer_virt_addr, buffer.len() as u32)
}

pub fn send_empty_frame() {
    let header = EthernetHeader::new(
        [0xff; 6],
        rtl8139::get_mac_address(),
        0x1122);
    let payload = Vec::new();
    let empty_frame = EthernetFrame::new(header, payload);

    send_frame(empty_frame);
}

    // let payload: Vec<u8>  = Vec::from([
        //     0x45,
        //     0x00,
        //     0x00, 0x00,
        //     0x81, 0x39,
        //     0x00, 0x00,
        //     0x80,
        //     0x11,
        //     0x00, 0x00,
        //     0x00, 0x00, 0x00 , 0x00,
        //     0xff, 0xff, 0xff, 0xff,
        //     0x22, 0x76,
        //     0x22, 0x76,
        //     0x00, 0x06
        // ]);