#![allow(dead_code)]

use crate::{
    println,
    pci,
};
const RTL8139_VENDOR_ID: u16 = 0x10EC;
const RTL8139_DEVICE_ID: u16 = 0x8139;

const ID: u8 = 0x00;
const TRANSMIT_STATUS: u8 = 0x10;
const TRANSMIT_ADDRESS: u8 = 0x20;
const COMMAND: u8 = 0x37;
const RECEIVE_BUFFER_START: u8 = 0x30;
const CURRENT_READ_ADDRESS: u8 = 0x38;
const INTERRUPT_MASK: u8 = 0x3c;
const INTERRUPT_STATUS: u8 = 0x3e;
const RECEIVE_CONFIGURATION: u8 = 0x44;
const CONFIG_1: u8 = 0x52;

pub fn init() {
    let opt_rtl8139 = pci::get_pci_device_id(RTL8139_VENDOR_ID, RTL8139_DEVICE_ID);
    
    if opt_rtl8139.is_some() {
        let rtl8139 = opt_rtl8139.unwrap();
        rtl8139.pci_set_command_bus_master_bit();
        rtl8139.pci_set_interrupt_disable_bit();

        println!("Waking up RTL8139");
        rtl8139.pci_write_8(CONFIG_1, 0x00);

        println!("Resetting");
        rtl8139.pci_write_8(COMMAND, 0x10);
        while (rtl8139.pci_read_8(COMMAND) & 0x10) != 0 {
            println!("1");// wait for reset to complete
        };
    }
}