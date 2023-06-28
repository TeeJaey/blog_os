#![allow(dead_code)]

extern crate alloc;

use crate::println;
use core::{
    fmt,
    ops::{Deref, DerefMut}
};
use alloc::vec::Vec;
use spin::{Once, Mutex};
use x86_64::{PhysAddr, instructions::port::Port};
use bit_field::BitField;
use lazy_static::lazy_static;

// The below constants define the PCI configuration space. 
// More info here: <http://wiki.osdev.org/PCI#PCI_Device_Structure>
const PCI_VENDOR_ID:             u8 = 0x0;
const PCI_DEVICE_ID:             u8 = 0x2;
const PCI_COMMAND:               u8 = 0x4;
const PCI_STATUS:                u8 = 0x6;
const PCI_REVISION_ID:           u8 = 0x8;
const PCI_PROG_IF:               u8 = 0x9;
const PCI_SUBCLASS:              u8 = 0xA;
const PCI_CLASS:                 u8 = 0xB;
const PCI_CACHE_LINE_SIZE:       u8 = 0xC;
const PCI_LATENCY_TIMER:         u8 = 0xD;
const PCI_HEADER_TYPE:           u8 = 0xE;
const PCI_BIST:                  u8 = 0xF;
const PCI_BAR0:                  u8 = 0x10;
const PCI_BAR1:                  u8 = 0x14;
const PCI_BAR2:                  u8 = 0x18;
const PCI_BAR3:                  u8 = 0x1C;
const PCI_BAR4:                  u8 = 0x20;
const PCI_BAR5:                  u8 = 0x24;
const PCI_CARDBUS_CIS:           u8 = 0x28;
const PCI_SUBSYSTEM_VENDOR_ID:   u8 = 0x2C;
const PCI_SUBSYSTEM_ID:          u8 = 0x2E;
const PCI_EXPANSION_ROM_BASE:    u8 = 0x30;
const PCI_CAPABILITIES:          u8 = 0x34;
// 0x35 through 0x3B are reserved
const PCI_INTERRUPT_LINE:        u8 = 0x3C;
const PCI_INTERRUPT_PIN:         u8 = 0x3D;
const PCI_MIN_GRANT:             u8 = 0x3E;
const PCI_MAX_LATENCY:           u8 = 0x3F;

/// If a BAR's bits [2:1] equal this value, that BAR describes a 64-bit address.
/// If not, that BAR describes a 32-bit address.
const BAR_ADDRESS_IS_64_BIT: u32 = 2;

/// There is a maximum of 256 PCI buses on one system.
const MAX_PCI_BUSES: u16 = 256;
/// There is a maximum of 32 slots on one PCI bus.
const MAX_SLOTS_PER_BUS: u8 = 32;
/// There is a maximum of 32 functions (devices) on one PCI slot.
const MAX_FUNCTIONS_PER_SLOT: u8 = 8;

/// Addresses/offsets into the PCI configuration space should clear the least-significant 2 bits.
const PCI_CONFIG_ADDRESS_OFFSET_MASK: u8 = 0xFC; 
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// This port is used to specify the address in the PCI configuration space
/// for the next read/write of the `PCI_CONFIG_DATA_PORT`.
static PCI_CONFIG_ADDRESS_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_ADDRESS));

/// This port is used to transfer data to or from the PCI configuration space
/// specified by a previous write to the `PCI_CONFIG_ADDRESS_PORT`.
static PCI_CONFIG_DATA_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_DATA));

pub fn init() {
    println!("{}", isize::MAX);

    let bus_list = get_pci_buses();
    for bus in bus_list {
        let dev_list = &bus.devices;
        for dev in dev_list {
            println!("location:{} VID:{} DID:{}", dev.location, dev.vendor_id, dev.device_id);
        }
    }
}

/// Returns a list of all PCI buses in this system.
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
pub fn get_pci_buses() -> &'static Vec<PciBus> {
    lazy_static! {
        static ref PCI_BUSES: Once<Vec<PciBus>> = Once::new();
    }
        PCI_BUSES.call_once(scan_pci)
}

/// Returns a reference to the `PciDevice` with the given bus, slot, func identifier.
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
pub fn get_pci_device_bsf(bus: u8, slot: u8, func: u8) -> Option<&'static PciDevice> {
    for b in get_pci_buses() {
        if b.bus_number == bus {
            for d in &b.devices {
                if d.slot == slot && d.func == func {
                    return Some(d);
                }
            }
        }
    }
    None
}

/// Returns a reference to the `PciDevice` with the given bus, slot, func identifier.
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
pub fn get_pci_device_id(vendor: u16, device: u16) -> Option<&'static PciDevice> {
    for b in get_pci_buses() {
            for d in &b.devices {
                if d.vendor_id == vendor && d.device_id == device {
                    return Some(d);
                }
            }
    }
    None
}

/// Returns an iterator that iterates over all `PciDevice`s, in no particular guaranteed order. 
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
pub fn pci_device_iter() -> impl Iterator<Item = &'static PciDevice> {
    get_pci_buses().iter().flat_map(|b| b.devices.iter())
}

/// A PCI bus, which contains a list of PCI devices on that bus.
#[derive(Debug)]
pub struct PciBus {
    /// The number identifier of this PCI bus.
    pub bus_number: u8,
    /// The list of devices attached to this PCI bus.
    pub devices: Vec<PciDevice>,
}

/// Scans all PCI Buses (brute force iteration) to enumerate PCI Devices on each bus.
/// Initializes structures containing this information. 
fn scan_pci() -> Vec<PciBus> {
    let mut buses: Vec<PciBus> = Vec::new();
    for bus in 0..MAX_PCI_BUSES {
        let bus = bus as u8;
        let mut device_list: Vec<PciDevice> = Vec::new();

        for slot in 0..MAX_SLOTS_PER_BUS {
            let loc_zero = PciLocation { bus, slot, func: 0 };
            // skip the whole slot if the vendor ID is 0xFFFF
            if 0xFFFF == loc_zero.pci_read_16(PCI_VENDOR_ID) {
                continue;
            }
            // If the header's MSB is set, then there are multiple functions for this device,
            // and we should check all 8 of them to be sure.
            // Otherwise, we only need to check the first function, because it's a single-function device.
            let header_type = loc_zero.pci_read_8(PCI_HEADER_TYPE);
            let functions_to_check = if header_type & 0x80 == 0x80 {
                0..MAX_FUNCTIONS_PER_SLOT
            } else {
                0..1
            };
            for f in functions_to_check {
                let location = PciLocation { bus, slot, func: f };
                let vendor_id = location.pci_read_16(PCI_VENDOR_ID);
                if vendor_id == 0xFFFF {
                    continue;
                }
                let device = PciDevice {
                    vendor_id,
                    device_id:        location.pci_read_16(PCI_DEVICE_ID), 
                    command:          location.pci_read_16(PCI_COMMAND),
                    status:           location.pci_read_16(PCI_STATUS),
                    revision_id:      location.pci_read_8( PCI_REVISION_ID),
                    prog_if:          location.pci_read_8( PCI_PROG_IF),
                    subclass:         location.pci_read_8( PCI_SUBCLASS),
                    class:            location.pci_read_8( PCI_CLASS),
                    cache_line_size:  location.pci_read_8( PCI_CACHE_LINE_SIZE),
                    latency_timer:    location.pci_read_8( PCI_LATENCY_TIMER),
                    header_type:      location.pci_read_8( PCI_HEADER_TYPE),
                    bist:             location.pci_read_8( PCI_BIST),
                    bars:             [
                                          location.pci_read_32(PCI_BAR0),
                                          location.pci_read_32(PCI_BAR1), 
                                          location.pci_read_32(PCI_BAR2), 
                                          location.pci_read_32(PCI_BAR3), 
                                          location.pci_read_32(PCI_BAR4), 
                                          location.pci_read_32(PCI_BAR5), 
                                      ],
                    int_pin:          location.pci_read_8(PCI_INTERRUPT_PIN),
                    int_line:         location.pci_read_8(PCI_INTERRUPT_LINE),
                    location,
                };
                device_list.push(device);
            }
        }

        if !device_list.is_empty() {
            buses.push( PciBus {
                bus_number: bus, 
                devices: device_list,
            });
        }
    }

    buses   
}


/// The bus, slot, and function number of a given PCI device.
/// This offers methods for reading and writing the PCI config space. 
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PciLocation {
    bus:  u8,
    slot: u8,
    func: u8,
}

impl PciLocation {
    pub fn bus(&self) -> u8 { self.bus }
    pub fn slot(&self) -> u8 { self.slot }
    pub fn function(&self) -> u8 { self.func }


    /// Computes a PCI address from bus, slot, func, and offset. 
    /// The least two significant bits of offset are masked, so it's 4-byte aligned addressing,
    /// which makes sense since we read PCI data (from the configuration space) in 32-bit chunks.
    fn pci_address(self, offset: u8) -> u32 {
        ((self.bus  as u32) << 16) | 
        ((self.slot as u32) << 11) | 
        ((self.func as u32) <<  8) | 
        ((offset as u32) & (PCI_CONFIG_ADDRESS_OFFSET_MASK as u32)) | 
        0x8000_0000
    }

    /// read 32-bit data at the specified `offset` from the PCI device specified by the given `bus`, `slot`, `func` set.
    fn pci_read_32(&self, offset: u8) -> u32 {
        unsafe { 
            PCI_CONFIG_ADDRESS_PORT.lock().write(self.pci_address(offset)); 
        }
        Self::read_data_port() >> ((offset & (!PCI_CONFIG_ADDRESS_OFFSET_MASK)) * 8)
    }

    /// Read 16-bit data at the specified `offset` from this PCI device.
    fn pci_read_16(&self, offset: u8) -> u16 {
        self.pci_read_32(offset) as u16
    } 

    /// Read 8-bit data at the specified `offset` from the PCI device.
    fn pci_read_8(&self, offset: u8) -> u8 {
        self.pci_read_32(offset) as u8
    }

    /// Write 32-bit data to the specified `offset` for the PCI device.
    fn pci_write(&self, offset: u8, value: u32) {
        unsafe {
            PCI_CONFIG_ADDRESS_PORT.lock().write(self.pci_address(offset)); 
            Self::write_data_port((value) << ((offset & 2) * 8));
        }
    }

    fn write_data_port(value: u32) {
        unsafe {
            PCI_CONFIG_DATA_PORT.lock().write(value);
        }
    }

    fn read_data_port() -> u32 {
        unsafe {
            PCI_CONFIG_DATA_PORT.lock().read()
        }
    }

    /// Sets the PCI device's bit 3 in the command portion, which is apparently needed to activate DMA (??)
    pub fn pci_set_command_bus_master_bit(&self) {
        unsafe { 
            PCI_CONFIG_ADDRESS_PORT.lock().write(self.pci_address(PCI_COMMAND));
        }
        let inval = Self::read_data_port(); 
        println!("pci_set_command_bus_master_bit: PciDevice: {}, read value: {:#x}", self, inval);
        Self::write_data_port(inval | (1 << 2));
        println!("pci_set_command_bus_master_bit: PciDevice: {}, read value AFTER WRITE CMD: {:#x}", 
            self,
            Self::read_data_port()
        );
    }

    /// Sets the PCI device's command bit 10 to disable legacy interrupts
    pub fn pci_set_interrupt_disable_bit(&self) {
        unsafe { 
            PCI_CONFIG_ADDRESS_PORT.lock().write(self.pci_address(PCI_COMMAND));
        }
        let command = Self::read_data_port(); 
        println!("pci_set_interrupt_disable_bit: PciDevice: {}, read value: {:#x}", self, command);
        const INTERRUPT_DISABLE: u32 = 1 << 10;
        Self::write_data_port(command | INTERRUPT_DISABLE);
        println!("pci_set_interrupt_disable_bit: PciDevice: {} read value AFTER WRITE CMD: {:#x}", 
            self, Self::read_data_port());
    }

}

impl fmt::Display for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "b{}.s{}.f{}", self.bus, self.slot, self.func)
    }
}

impl fmt::Debug for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{self}")
    }
}


/// Contains information common to every type of PCI Device,
/// and offers functions for reading/writing to the PCI configuration space.
///
/// For more, see [this partial table](http://wiki.osdev.org/PCI#Class_Codes)
/// of `class`, `subclass`, and `prog_if` codes, 
#[derive(Debug)]
pub struct PciDevice {
    /// the bus, slot, and function number that locates this PCI device in the bus tree.
    pub location: PciLocation,

    /// The class code, used to determine device type.
    pub class: u8,
    /// The subclass code, used to determine device type.
    pub subclass: u8,
    /// The programming interface of this PCI device, also used to determine device type.
    pub prog_if: u8,
    /// The six Base Address Registers (BARs)
    pub bars: [u32; 6],
    pub vendor_id: u16,
    pub device_id: u16, 
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,
    pub int_pin: u8,
    pub int_line: u8,
}

impl PciDevice {
    /// Returns the base address of the memory region specified by the given `BAR` 
    /// (Base Address Register) for this PCI device. 
    ///
    /// # Argument
    /// * `bar_index` must be between `0` and `5` inclusively, as each PCI device 
    ///   can only have 6 BARs at the most.  
    ///
    /// Note that if the given `BAR` actually indicates it is part of a 64-bit address,
    /// it will be used together with the BAR right above it (`bar + 1`), e.g., `BAR1:BAR0`.
    /// If it is a 32-bit address, then only the given `BAR` will be accessed.
    ///
    /// TODO: currently we assume the BAR represents a memory space (memory mapped I/O) 
    ///       rather than I/O space like Port I/O. Obviously, this is not always the case.
    ///       Instead, we should return an enum specifying which kind of memory space the calculated base address is.
    pub fn determine_mem_base(&self, bar_index: usize) -> Result<PhysAddr, &'static str> {
        let mut bar = if let Some(bar_value) = self.bars.get(bar_index) {
            *bar_value
        } else {
            return Err("BAR index must be between 0 and 5 inclusive");
        };

        // Check bits [2:1] of the bar to determine address length (64-bit or 32-bit)
        let mem_base = if bar.get_bits(1..3) == BAR_ADDRESS_IS_64_BIT { 
            // Here: this BAR is the lower 32-bit part of a 64-bit address, 
            // so we need to access the next highest BAR to get the address's upper 32 bits.
            let next_bar = *self.bars.get(bar_index + 1).ok_or("next highest BAR index is out of range")?;
            // Clear the bottom 4 bits because it's a 16-byte aligned address
            PhysAddr::new(*bar.set_bits(0..4, 0) as u64 | ((next_bar as u64) << 32))
        } else {
            // Here: this BAR is the lower 32-bit part of a 64-bit address, 
            // so we need to access the next highest BAR to get the address's upper 32 bits.
            // Also, clear the bottom 4 bits because it's a 16-byte aligned address.
            PhysAddr::new(*bar.set_bits(0..4, 0) as u64)
        };  
        Ok(mem_base)
    }

    /// Returns the size in bytes of the memory region specified by the given `BAR` 
    /// (Base Address Register) for this PCI device.
    ///
    /// # Argument
    /// * `bar_index` must be between `0` and `5` inclusively, as each PCI device 
    /// can only have 6 BARs at the most. 
    ///
    pub fn determine_mem_size(&self, bar_index: usize) -> u32 {
        assert!(bar_index < 6);
        // Here's what we do: 
        // (1) Write all `1`s to the specified BAR
        // (2) Read that BAR value again
        // (3) Mask the info bits (bits [3:0]) of the BAR value read in Step 2
        // (4) Bitwise "not" (negate) that value, then add 1.
        //     The resulting value is the size of that BAR's memory region.
        // (5) Restore the original value to that BAR
        let bar_offset = PCI_BAR0 + (bar_index as u8 * 4);
        let original_value = self.bars[bar_index];

        self.pci_write(bar_offset, 0xFFFF_FFFF);          // Step 1
        let mut mem_size = self.pci_read_32(bar_offset);  // Step 2
        mem_size.set_bits(0..4, 0);                       // Step 3
        mem_size = !(mem_size);                           // Step 4
        mem_size += 1;                                    // Step 4
        self.pci_write(bar_offset, original_value);       // Step 5
        mem_size
    }

}

impl Deref for PciDevice {
    type Target = PciLocation;
    fn deref(&self) -> &PciLocation {
        &self.location
    }
}
impl DerefMut for PciDevice {
    fn deref_mut(&mut self) -> &mut PciLocation {
        &mut self.location
    }
}

/// Lists the 2 possible PCI configuration space access mechanisms
/// that can be found from the LSB of the devices's BAR0
pub enum PciConfigSpaceAccessMechanism {
    MemoryMapped = 0,
    IoPort = 1,
}