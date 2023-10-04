#![allow(dead_code)]

use core::mem::size_of;
use alloc::vec::Vec;
use crate::rtl8139;

#[derive(Debug)]
pub struct EthernetFrame {
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ethertype: u16,
    payload: Vec<u8>,
}

impl EthernetFrame {
    fn new(dst_mac: [u8; 6], src_mac: [u8; 6], ethertype: u16, payload: Vec<u8>) -> Self {
        Self {dst_mac, src_mac, ethertype, payload}
    }
}

pub fn eth_send_packet(dst_mac: [u8; 6], protocol: u16, payload: Vec<u8>) {

    let len = size_of::<EthernetFrame>();
    let src_mac = rtl8139::get_mac_address();
    let frame = EthernetFrame::new(dst_mac, src_mac, protocol, payload);

    let mut frame_data: Vec<EthernetFrame> = Vec::with_capacity(len);
    frame_data.push(frame);

    rtl8139::rtl_send_packet(frame_data.as_ptr(), len)
}

//hellp