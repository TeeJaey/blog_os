use crate::{gdt, hlt_loop, print, println};
use alloc::{vec::Vec, string::String};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 0x20;
pub const PIC_2_OFFSET: u8 = 0x28;

static mut COUNT_DOWN: u32 = 0;

#[derive(Debug, Clone)]
struct MyInterruptIndex {
    table: Vec<(String, u8)>
}

impl MyInterruptIndex {
    fn new() -> Self {
        MyInterruptIndex { table: Vec::new() }
    }

    fn insert(&mut self, key: &str, value: u8) {
        self.table.push((String::from(key), value));
    }

    fn get(&self, key: &str) -> Option<u8> {
        for pair in &self.table {
            if pair.0 == key {
                return Some(pair.1);
            }
        }
        None
    }
}

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref INDEX: Mutex<MyInterruptIndex> = Mutex::new({
        let mut index = MyInterruptIndex::new();
        index.insert("Timer", PIC_1_OFFSET);
        index.insert("Keyboard", PIC_1_OFFSET + 1);
        index
    });
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[INDEX.lock().get("Timer").unwrap() as usize].set_handler_fn(timer_interrupt_handler);
        idt[INDEX.lock().get("Keyboard").unwrap() as usize].set_handler_fn(keyboard_interrupt_handler);
        let rtl8139 = INDEX.lock().get("RTL8139");
        if rtl8139.is_some() {
            idt[rtl8139.unwrap() as usize].set_handler_fn(rtl8139_interrupt_handler);
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub unsafe fn sleep(time: u32) {
    COUNT_DOWN = time;
    println!("sleeping for {}", COUNT_DOWN);
    while COUNT_DOWN > 0 {
        print!(".");
        COUNT_DOWN -= 1;
        x86_64::instructions::hlt();
    }
    println!(".");
}

pub fn regiser_interrupt(name: &str, line: u8) {
    INDEX.lock().insert(name, PIC_1_OFFSET + line)
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // print!(".");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(INDEX.lock().get("Timer").unwrap());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(INDEX.lock().get("Keyboard").unwrap());
    }
}

extern "x86-interrupt" fn rtl8139_interrupt_handler(_stack_frame: InterruptStackFrame) {    
    use crate::rtl8139;
    rtl8139::handle_interrupt();
    
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(INDEX.lock().get("RTL8139").unwrap());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
