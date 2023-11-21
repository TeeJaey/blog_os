#![allow(dead_code)]

use crate::{
    println,
    pci,
    memory, interrupts,
};
use alloc::vec::Vec;
use x86_64::{
    instructions::port::Port,
    VirtAddr
};

use spin::Mutex;

// Register
const ID0: u8 = 0x00;
const ID1: u8 = 0x01;
const ID2: u8 = 0x02;
const ID3: u8 = 0x03;
const ID4: u8 = 0x04;
const ID5: u8 = 0x05;
const TRANSMIT_STATUS: u8 = 0x10;
const TRANSMIT_ADDRESS: u8 = 0x20;
const COMMAND: u8 = 0x37;
const RB_START: u8 = 0x30;
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

// TransmitStatus
const OWN: u32 = 0x2000;
const FIFO_UNDERRUN: u32 = 0x4000;
const TRANSMIT_STATUS_OK: u32 = 0x8000;
const EARLY_TX_THRESHOLD: u32 = 0x10000;
const TRANSMIT_STATUS_ABORT: u32 = 0x40000000;
const CARRIER_SENSE_LOST: u32 = 0x80000000;

// ReceiveStatus
const ROK: u16 = 0x0001;
const FRAME_ALIGNMENT_ERROR: u16 = 0x0002;
const CHECKSUM_ERROR: u16 = 0x0004;
const LONG_PACKET: u16 = 0x0008;
const RUNT_PACKET: u16 = 0x0010;
const INVALID_SYMBOL: u16 = 0x0020;
const BROADCAST: u16 = 0x2000;
const PHYSICAL_ADDRESS: u16 = 0x4000;
const MULTICAST: u16 = 0x8000;

const RTL8139_VENDOR_ID: u16 = 0x10EC;
const RTL8139_DEVICE_ID: u16 = 0x8139;
const BUFFER_SIZE: u32 = 8 * 1024 + 16 + 1500;
const TRANSMIT_DESCRIPTOR_COUNT: u8 = 4;

static mut RECEIVE_BUFFER: [u8; BUFFER_SIZE as usize] = [0; BUFFER_SIZE as usize];
static mut TRANSMIT_DESCRIPTOR: u8 = 0;
static mut IO_BASE_ADDR: u16 = 0;
static mut RECEIVE_INDEX: i16 = 0;

pub fn init() {
    println!("Beginning initialisation of RTL8139!");
    pci::get_pci_buses();
    
    // let bus_list = pci::get_pci_buses();
    // for bus in bus_list {
    //     let dev_list = &bus.devices;
    //     for dev in dev_list {
    //         println!("location:{} VID:{} DID:{}", dev.location, dev.vendor_id, dev.device_id);
    //     }
    // }

    let opt_rtl8139 = pci::get_pci_device_id(RTL8139_VENDOR_ID, RTL8139_DEVICE_ID);
    
    if opt_rtl8139.is_some() {
        let rtl8139_dev = opt_rtl8139.unwrap();

        rtl8139_dev.pci_set_command_register_bit(pci::BUS_MASTER);
        rtl8139_dev.pci_set_command_register_bit(pci::IO_SPACE);

        unsafe { IO_BASE_ADDR = rtl8139_dev.determine_iobase(0).unwrap() as u16; }
        
        interrupts::regiser_interrupt("RTL8139", rtl8139_dev.int_line);

        println!("Powering on / Waking up RTL8139");
        io_write_8(CONFIG_1, 0x0);
        
        println!("Performing software reset");
        io_write_8(COMMAND, RESET);
        while (io_read_8(COMMAND) & RESET) != 0 {
            println!("RST-Bit is still high (1)");
        }

        println!("Masking interrupts");
        io_write_16(INTERRUPT_MASK, RECEIVE_OK | RECEIVE_ERROR | TRANSMIT_OK | TRANSMIT_ERROR);

        println!("Enabling receiver/transmitter");
        io_write_8(COMMAND, ENABLE_RECEIVER | ENABLE_TRANSMITTER);

        println!("Configuring receive buffer");
        unsafe {
            let rxbuf_virt = VirtAddr::new_unsafe(RECEIVE_BUFFER.as_ptr() as u64);
            let virt_to_phys = memory::translate_addr(rxbuf_virt);
            let rxbuf_phys = virt_to_phys.unwrap().as_u64();
            io_write_32(RB_START, rxbuf_phys as u32);
            io_write_32(RECEIVE_CONFIGURATION, WRAP | ACCEPT_PHYSICAL_MATCH | ACCEPT_BROADCAST | LENGTH_8K);
        }
        println!("RTL8139 init complete...");
    } else {
        println!("Aborting RTL8139 initialisation...")
    }
}

pub fn get_mac_address() -> [u8; 6] {
    return [
        io_read_8(ID0),
        io_read_8(ID1),
        io_read_8(ID2),
        io_read_8(ID3),
        io_read_8(ID4),
        io_read_8(ID5)
    ]
}

pub fn handle_interrupt() {
	let status = io_read_16(INTERRUPT_STATUS);
	io_write_16(INTERRUPT_STATUS, RECEIVE_OK | TRANSMIT_OK | RECEIVE_ERROR | TRANSMIT_ERROR);
	
    if (status & RECEIVE_OK) != 0 {
		// Received
        println!("RTL8139: RECEIVE_OK");
        while (io_read_8(COMMAND) & BUFFER_EMPTY) == 0 {
            receive_packets();
        }
	}
    else if (status & RECEIVE_ERROR) != 0 {
        println!("RTL8139: RECEIVE_ERROR");
	}
    else if (status & TRANSMIT_OK) != 0 {
		// Sent
        println!("RTL8139: TRANSMIT_OK");
	}
    else if (status & TRANSMIT_ERROR) != 0 {
        println!("RTL8139: TRANSMIT_ERROR");
	}
}

pub fn send_packet(buffer_virt_addr: VirtAddr, len: u32) {
    println!("sending packet");
    println!("buffer virt addr: {:?}", buffer_virt_addr);
    println!("buffer len: {:x}", len);

    loop{
        unsafe {
            let status = io_read_32(TRANSMIT_STATUS + (4 * TRANSMIT_DESCRIPTOR));
            println!("transmit status register of descriptor {}: {:x?}", TRANSMIT_DESCRIPTOR, status);
            if (status & OWN) != 0 {
                break;
            }
        }
    }

    let virt_to_phys = unsafe {memory::translate_addr(buffer_virt_addr)};
    let buffer_phys_addr = virt_to_phys.unwrap().as_u64() as u32;
    
    println!("buffer phys addr: {:x?}", buffer_phys_addr);

    set_transmit_buffer(buffer_phys_addr); 
    set_transmit_status(len);

    unsafe {
        TRANSMIT_DESCRIPTOR = (TRANSMIT_DESCRIPTOR + 1) % TRANSMIT_DESCRIPTOR_COUNT
    }
}

fn set_transmit_buffer(buffer: u32) {
    unsafe{
        let offset = TRANSMIT_ADDRESS +(4 * TRANSMIT_DESCRIPTOR);
        io_write_32(offset, buffer);
        // println!("{}", io_read_32(offset))
    }
}

fn set_transmit_status(size: u32) {
    unsafe{
        let offset = TRANSMIT_STATUS +(4 * TRANSMIT_DESCRIPTOR);
        io_write_32(offset, size);
        // println!("{}", io_read_32(offset))
    }
}

pub fn receive_packets() {
    let header: u16 = unsafe {(RECEIVE_BUFFER[RECEIVE_INDEX as usize + 1] as u16) << 8 | (RECEIVE_BUFFER[RECEIVE_INDEX as usize] as u16)};
    println!("header: {:x}", header);
    if (header & ROK) != 0 {
        let length: i16 = unsafe {(RECEIVE_BUFFER[RECEIVE_INDEX as usize + 3] as i16) << 8 | (RECEIVE_BUFFER[RECEIVE_INDEX as usize + 2] as i16)};
        println!("PACKET LENGTH: {:?} (including 4 CRC)", length);
        let payload: Vec<u8> = Vec::from(unsafe {&RECEIVE_BUFFER[RECEIVE_INDEX as usize + 4..RECEIVE_INDEX as usize + (length as usize)]});
        println!("PACKET PAYLOAD: {:x?}", payload);
        
        unsafe {
            println!("PRE-ADJUST RECEIVE_INDEX: {}", RECEIVE_INDEX);
            RECEIVE_INDEX += length + 4;
            RECEIVE_INDEX = (RECEIVE_INDEX + 3) & !0x3;
            RECEIVE_INDEX %= 0x2000;
            io_write_16(CURRENT_READ_ADDRESS, (RECEIVE_INDEX - 0x10) as u16);
            println!("POST-ADJUST RECEIVE_INDEX: {}", RECEIVE_INDEX);
        }
    }
}

fn io_read_8(offset: u8) -> u8 {
    let io_port: Mutex<Port<u8>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_read_16(offset: u8) -> u16 {
    let io_port: Mutex<Port<u16>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_read_32(offset: u8) -> u32 {
    let io_port: Mutex<Port<u32>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    let res = unsafe{io_port.lock().read()};
    res
}

fn io_write_8(offset: u8, value: u8) {
    let io_port: Mutex<Port<u8>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    unsafe{io_port.lock().write(value);}
}

fn io_write_16(offset: u8, value: u16) {
    let io_port: Mutex<Port<u16>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    unsafe{io_port.lock().write(value);}
}

fn io_write_32(offset: u8, value: u32) {
    let io_port: Mutex<Port<u32>> = Mutex::new(unsafe {Port::new(IO_BASE_ADDR + offset as u16)});
    unsafe{io_port.lock().write(value);}
}