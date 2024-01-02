#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _;
use panic_semihosting as _;
// you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32f1xx_hal::{
    i2c::{self, BlockingI2c, DutyCycle, Mode},
    pac,
    prelude::*,
};

use rtt_target::{rprintln, rtt_init_print};

const DEVICE_ADDRESS: u8 = 0x58;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    // take ownership of the device peripherals singleton
    let peripherals = pac::Peripherals::take().unwrap();

    // configure GPIOC pin 13 as a push-pull output to drive the on-board LED
    let mut gpioc = peripherals.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    // section 30.1, page 1076 of reference manual
    let flash_size_kb = unsafe { core::ptr::read_volatile(0x1FFFF7E0 as *const u16) };
    rprintln!("flash size: {} KB", flash_size_kb);

    // configure the use of the external 8 MHz crystal
    let mut flash = peripherals.FLASH.constrain();
    let rcc = peripherals.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(8000.kHz()).freeze(&mut flash.acr);

    //config PB8 and PB9 as I2C
    let mut gpiob = peripherals.GPIOB.split();
    let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);
    let mut afio = peripherals.AFIO.constrain();
    let mut i2c = BlockingI2c::i2c1(
        peripherals.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 200.kHz(),
            duty_cycle: DutyCycle::Ratio16to9,
        },
        clocks,
        1000,
        10,
        1000,
        1000,
    );

    // sleep for 1/2 second to let the sensor warm up
    asm::delay(4 * 1_000_000);

    let cmd = SensorCommand::InitAirQuality;

    let write_res = i2c.write(DEVICE_ADDRESS, &cmd.to_command());
    rprintln!("write_res: {:?}", write_res);

    // // wait for 0.5 ms for the data to be ready, then read the 3 byte response
    // asm::delay(220 * 8 * 1_000);
    // let mut data = [0u8; 3];
    // let res = i2c.read(DEVICE_ADDRESS, &mut data);
    // let crc = crc8(&data[0..data.len() - 1]);
    // rprintln!("read: {:?}", res);
    // rprintln!("data: {:00.X?}", data);
    // rprintln!("crc: {:X}", crc);

    loop {
        let timestamp = cortex_m::peripheral::DWT::cycle_count();

        // once per second, send the "measure_iaq" command
        let cmd = SensorCommand::MeasureAirQuality;
        let write_res = i2c.write(DEVICE_ADDRESS, &cmd.to_command());
        // wait at least 12 ms for the data to be ready, then read the 6 byte response
        asm::delay(12 * 8 * 1_000);
        let mut data = [0u8; 6];
        let read_res = i2c.read(DEVICE_ADDRESS, &mut data);
        let crc: [u8; 2] = [crc8(&data[0..2]), crc8(&data[3..5])];
        rprintln!("write_res: {:?}", write_res);
        rprintln!("read_res: {:?}", read_res);
        rprintln!("data: {:X?} crc: {:X?}", data, crc);
        // turn the first two bytes into a u16
        let co2 = ((data[0] as u16) << 8) | (data[1] as u16);
        let voc = ((data[3] as u16) << 8) | (data[4] as u16);
        rprintln!("co2: {} voc: {}", co2, voc);

        // wait 1 second
        let delay_left = 5_600_000 - (cortex_m::peripheral::DWT::cycle_count() - timestamp);
        asm::delay(delay_left as u32);
        // asm::delay(8 * 1_000_000);

        // toggle the LED state
        // led.toggle();
        led.set_high();

        // wait for 125ms
        // the external clock is 8MHz
        // so 8_000_000 cycles = 1 second
        // 1_000_000 cycles = 1/8 seconds = 125ms
        // asm::delay(10 * 8 * 1_000_000);
    }
}

/// CRC-8-ATM
fn crc8(data: &[u8]) -> u8 {
    let poly: u8 = 0x31; // Polynomial 0x31 (x^8 + x^5 + x^4 + x^1)
    let mut crc: u8 = 0xFF; // Initialization value

    for &byte in data {
        crc ^= byte; // Initial XOR
        for _ in 0..8 {
            // Process for each bit
            if (crc & 0x80) != 0 {
                // If high bit is set...
                crc = (crc << 1) ^ poly; // ...shift left and XOR with poly
            } else {
                crc <<= 1; // Otherwise, just shift left
            }
        }
    }
    // No final XOR in this algorithm (Final XOR = 0x00)
    crc
}

/*
+----------------------+---------+---------------------------------+---------------------------------+---------------------+---------+
| Feature Set          | Hex.    | Parameter length                | Response length                 | Measurement duration|         |
| Command              | Code    | including CRC [bytes]           | including CRC [bytes]           | Typ. [ms]           | Max. [ms]|
+----------------------+---------+---------------------------------+---------------------------------+---------------------+---------+
| Init_air_quality     | 0x2003  | -                               | -                               | 2                   | 10      |
| Measure_air_quality  | 0x2008  | -                               | 6                               | 10                  | 12      |
| Get_baseline         | 0x2015  | -                               | 6                               | 10                  | 10      |
| Set_baseline         | 0x201e  | 6                               | -                               | 10                  | 10      |
| Set_humidity         | 0x2061  | 3                               | -                               | 1                   | 10      |
| Measure_test         | 0x2032  | -                               | 3                               | 200                 | 220     |
| Get_feature_set_ver  | 0x202f  | -                               | 3                               | 1                   | 2       |
| Measure_raw_signals  | 0x2050  | -                               | 6                               | 20                  | 25      |
+----------------------+---------+---------------------------------+---------------------------------+---------------------+---------+
*/

#[repr(u16)]
#[derive(Debug, Copy, Clone)]
#[allow(unused)]
enum SensorCommand {
    InitAirQuality = 0x2003,
    MeasureAirQuality = 0x2008,
    GetBaseline = 0x2015,
    SetBaseline = 0x201e,
    SetHumidity = 0x2061,
    MeasureTest = 0x2032,
    GetFeatureSetVer = 0x202f,
    MeasureRawSignals = 0x2050,
    GetSerialId = 0x3682,
}

impl SensorCommand {
    fn to_bytes(&self) -> [u8; 2] {
        [((*self as u16) >> 8) as u8, (*self as u16) as u8]
    }

    fn to_command(&self) -> [u8; 3] {
        let bytes = self.to_bytes();
        [bytes[0], bytes[1], crc8(&bytes)]
    }
}

#[test]
fn crc8_test() {
    let data = [0xBE, 0xEF]; // Example data
    let checksum = crc8(&data);
    // println!("The CRC-8 checksum of {:X?} is: {:X}", data, checksum);
    assert_eq!(checksum, 0x92);
}
