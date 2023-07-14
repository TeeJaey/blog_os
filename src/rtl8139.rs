#![allow(dead_code)]

use crate::{
    println,
    pci,
};
use alloc::vec::Vec;
use x86_64::instructions::port::Port;
use spin::Mutex;


const RTL8139_VENDOR_ID: u16 = 0x10EC;
const RTL8139_DEVICE_ID: u16 = 0x8139;
const BUFFER_SIZE: u32 = 8 * 1024 + 16 + 1500;
const TRANSMIT_DESCRIPTOR_COUNT: u8 = 4;



// Register
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

// Command
const BUFFER_EMPTY: u8 = 0x01;
const ENABLE_TRANSMITTER: u8 = 0x04;
const ENABLE_RECEIVER: u8 = 0x08;
const RESET: u8 = 0x10;

// Interrupt
const RECEIVE_OK: u16 = 0x0001;
const RECEIVE_ERROR: u16 = 0x0002;
const TRANSMIT_OK: u16 = 0x0004;
const TRANSMIT_ERROR: u16 = 0x0008;
const RX_BUFFER_OVERFLOW: u16 = 0x0010;
const PACKET_UNDERRUN_LINK_CHANGE: u16 = 0x0020;
const RX_FIFO_OVERFLOW: u16 = 0x0040;
const CABLE_LENGTH_CHANGE: u16 = 0x2000;
const TIMEOUT: u16 = 0x4000;
const SYSTEM_ERROR: u16 = 0x8000;

// ReceiveFlag
const ACCEPT_ALL: u32 = 0x0001;
const ACCEPT_PHYSICAL_MATCH: u32 = 0x0002;
const ACCEPT_MULTICAST: u32 = 0x0004;
const ACCEPT_BROADCAST: u32 = 0x0008;
const ACCEPT_RUNT: u32 = 0x0010;
const ACCEPT_ERROR: u32 = 0x0020;
const WRAP: u32 = 0x0080;
const LENGTH_8K: u32 = 0x0000;
const LENGTH_16K: u32 = 0x0800;
const LENGTH_32K: u32 = 0x1000;
const LENGTH_64K: u32 = 0x1800;

// TransmitStatus : uint32_t {
const OWN: u32 = 0x2000;
const FIFO_UNDERRUN: u32 = 0x4000;
const TRANSMIT_STATUS_OK: u32 = 0x8000;
const EARLY_TX_THRESHOLD: u32 = 0x10000;
const TRANSMIT_STATUS_ABORT: u32 = 0x40000000;
const CARRIER_SENSE_LOST: u32 = 0x80000000;

// ReceiveStatus : uint16_t {
const OK: u32 = 0x0001;
const FRAME_ALIGNMENT_ERROR: u32 = 0x0002;
const CHECKSUM_ERROR: u32 = 0x0004;
const LONG_PACKET: u32 = 0x0008;
const RUNT_PACKET: u32 = 0x0010;
const INVALID_SYMBOL: u32 = 0x0020;
const BROADCAST: u32 = 0x2000;
const PHYSICAL_ADDRESS: u32 = 0x4000;
const MULTICAST: u32 = 0x8000;

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

        rtl8139.pci_set_command_register_bit(pci::BUS_MASTER);
        rtl8139.pci_set_command_register_bit(pci::IO_SPACE);

        let ioaddr = rtl8139.determine_iobase(0).unwrap() as u16;

        println!("{:#x}", io_read_8(ioaddr, 0x0));
        println!("{:#x}", io_read_8(ioaddr, 0x1));
        println!("{:#x}", io_read_8(ioaddr, 0x2));
        println!("{:#x}", io_read_8(ioaddr, 0x3));
        println!("{:#x}", io_read_8(ioaddr, 0x4));
        println!("{:#x}", io_read_8(ioaddr, 0x5));

        println!("Powering on / Waking up RTL8139");

        io_write_8(ioaddr, CONFIG_1, 0x0);
        
        println!("Performing software reset");
        
        io_write_8(ioaddr, COMMAND, RESET);
        while (io_read_8(ioaddr, COMMAND) & RESET) != 0 {
            println!("RST-Bit is still high (1)");
        }

        println!("Masking interrupts");
        io_write_16(ioaddr, INTERRUPT_MASK, RECEIVE_OK | RECEIVE_ERROR | TRANSMIT_OK | TRANSMIT_ERROR);

        println!("Enabling receiver/transmitter");
        io_write_8(ioaddr, COMMAND, ENABLE_RECEIVER | ENABLE_TRANSMITTER);

        println!("Configuring receive buffer");
        let receive_buffer: Vec<u8> = Vec::with_capacity(BUFFER_SIZE as usize);
        let receive_buffer_addr: u32 = receive_buffer.as_ptr() as u32;

        io_write_32(ioaddr, RECEIVE_BUFFER_START, receive_buffer_addr);
        io_write_32(ioaddr, RECEIVE_CONFIGURATION, WRAP | ACCEPT_PHYSICAL_MATCH | ACCEPT_BROADCAST | LENGTH_8K);
    }
}

fn io_read_8(iobase: u16, offset: u8) -> u8 {
    let io_port: Mutex<Port<u8>> = Mutex::new(Port::new(iobase + offset as u16));
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_read_16(iobase: u16, offset: u8) -> u16 {
    let io_port: Mutex<Port<u16>> = Mutex::new(Port::new(iobase + offset as u16));
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_read_32(iobase: u16, offset: u8) -> u32 {
    let io_port: Mutex<Port<u32>> = Mutex::new(Port::new(iobase + offset as u16));
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_write_8(iobase: u16, offset: u8, value: u8) {
    let io_port: Mutex<Port<u8>> = Mutex::new(Port::new(iobase + offset as u16));
    unsafe{io_port.lock().write(value);}
}

fn io_write_16(iobase: u16, offset: u8, value: u16) {
    let io_port: Mutex<Port<u16>> = Mutex::new(Port::new(iobase + offset as u16));
    unsafe{io_port.lock().write(value);}
}

fn io_write_32(iobase: u16, offset: u8, value: u32) {
    let io_port: Mutex<Port<u32>> = Mutex::new(Port::new(iobase + offset as u16));
    unsafe{io_port.lock().write(value);}
}