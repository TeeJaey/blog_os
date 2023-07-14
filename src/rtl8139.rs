#![allow(dead_code)]

use crate::{
    println,
    pci,
};
use x86_64::instructions::port::Port;
use spin::Mutex;


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
    let bus_list = pci::get_pci_buses();
    for bus in bus_list {
        let dev_list = &bus.devices;
        for dev in dev_list {
            println!("location:{} VID:{} DID:{}", dev.location, dev.vendor_id, dev.device_id);
        }
    }

    let opt_rtl8139 = pci::get_pci_device_id(RTL8139_VENDOR_ID, RTL8139_DEVICE_ID);
    
    if opt_rtl8139.is_some() {
        let rtl8139 = opt_rtl8139.unwrap();

        rtl8139.pci_set_command_bus_master_bit();

        rtl8139.pci_show_header_register(pci::PCI_BAR0);

        let ioaddr = rtl8139.determine_iobase(0).unwrap() as u16;

        println!("Waking up RTL8139");

        println!("{:#x}", read_io(ioaddr, CONFIG_1));
        write_io(ioaddr, CONFIG_1, 0x0);
        println!("{:#x}", read_io(ioaddr, CONFIG_1));
        
        println!("Starting Software Reset for RTL8139");
        
        println!("{:#x}", read_io(ioaddr, COMMAND));
        write_io(ioaddr, COMMAND, 0x10);
        while (read_io(ioaddr, COMMAND) & 0x10) != 0 {
            println!("RST-Bit is still high (1)");
        }
        println!("{:#x}", read_io(ioaddr, COMMAND));
    }
}

pub fn read_io(iobase: u16, offset: u8) -> u32 {
    let io_port: Mutex<Port<u32>> = Mutex::new(Port::new(iobase + offset as u16));
    let res = unsafe{io_port.lock().read()};
    res
}

pub fn write_io(iobase: u16, offset: u8, value: u32) {
    let io_port: Mutex<Port<u32>> = Mutex::new(Port::new(iobase + offset as u16));
    unsafe{io_port.lock().write(value);}
}