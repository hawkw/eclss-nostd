#![no_std]
#![no_main]

extern crate alloc;
use esp32c3_hal::{
    clock::ClockControl,
    gpio::IO,
    i2c::I2C,
    pac::{Peripherals, I2C0},
    prelude::*,
    timer::TimerGroup,
    Rtc,
};
use esp_backtrace as _;
use esp_println::println;
use shared_bus::BusManagerSimple;
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

pub type I2cRef<'bus> = shared_bus::I2cProxy<'bus, shared_bus::NullMutex<I2C<I2C0>>>;

mod scd30;

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;

    extern "C" {
        static mut _heap_start: u32;
    }

    unsafe {
        let heap_start = &_heap_start as *const _ as usize;
        ALLOCATOR.init(heap_start as *mut u8, HEAP_SIZE);
    }
}
#[riscv_rt::entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();
    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    println!("starting eclss");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Create a new peripheral object with the described wiring
    // and standard I2C clock speed
    let i2c = I2C::new(
        peripherals.I2C0,
        // on the Adafruit QT Py ESP32c3, SDA is GPIO 5, and SCL is GPIO 6
        io.pins.gpio5,
        io.pins.gpio6,
        100u32.kHz(),
        &mut system.peripheral_clock_control,
        &clocks,
    );
    let bus = BusManagerSimple::new(i2c);
    let mut scd30 = scd30::bringup(&bus, &clocks).unwrap();

    // Poll for data
    loop {
        // Keep looping until ready
        match scd30.data_ready() {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => {
                println!("error waiting for SCD30 to become ready: {error:?}");
                continue;
            }
        }

        // Fetch data when available
        let m = scd30.read_data();
        println!("Measurement: {:?}", m);
    }
}
