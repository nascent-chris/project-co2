#![no_std]
#![no_main]

pub mod sgp30;

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
    i2c::{BlockingI2c, DutyCycle, Mode},
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

    let cmd = SensorCommand::GetSerialId;
    i2c.write(DEVICE_ADDRESS, &cmd.to_command())
        .expect("write failed");

    // wait for 0.5 ms for the data to be ready, then read the 3 byte response
    asm::delay(500 * 8 * 1_000);
    let mut read_buffer = [0u8; 6];
    i2c.read(DEVICE_ADDRESS, &mut read_buffer)
        .expect("read failed");

    // verify CRC
    let crc_valid = |b: &[u8]| b.chunks(3).all(|chunk| chunk[2] == crc8(&chunk[0..2]));
    let crc_valid_str = |b: &[u8]| if crc_valid(b) { "OK" } else { "FAIL" };
    let mut hex_output = [0u8; 6 * 2]; // buffer must be twice the size of the input
    match to_hex_string(&read_buffer, &mut hex_output)
        .and_then(|_| core::str::from_utf8(&hex_output).map_err(|_| "invalid utf8"))
    {
        Ok(s) => rprintln!("serial: {} crc ok: {}", s, crc_valid(&read_buffer)),
        Err(e) => rprintln!("Error: {}", e),
    }

    let cmd = SensorCommand::InitAirQuality;

    i2c.write(DEVICE_ADDRESS, &cmd.to_command())
        .expect("write failed");

    loop {
        let timestamp = cortex_m::peripheral::DWT::cycle_count();

        // once per second, send the "measure_iaq" command
        let cmd = SensorCommand::MeasureAirQuality;
        i2c.write(DEVICE_ADDRESS, &cmd.to_command())
            .expect("write failed");

        // wait at least 12 ms for the data to be ready, then read the 6 byte response
        asm::delay(12 * 8 * 1_000);

        i2c.read(DEVICE_ADDRESS, &mut read_buffer)
            .expect("read failed");

        // verify CRC
        rprintln!(
            "data: {:X?} crc: {}",
            read_buffer,
            crc_valid_str(&read_buffer)
        );

        let co2 = ((read_buffer[0] as u16) << 8) | (read_buffer[1] as u16);
        let voc = ((read_buffer[3] as u16) << 8) | (read_buffer[4] as u16);
        rprintln!("co2: {} voc: {}", co2, voc);

        // wait 1 second
        let delay_left = 5_600_000 - (cortex_m::peripheral::DWT::cycle_count() - timestamp);
        asm::delay(delay_left);

        // toggle the LED state
        // led.toggle();
        led.set_high();
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

fn to_hex_string(input: &[u8], output: &mut [u8]) -> Result<(), &'static str> {
    const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";

    if output.len() < input.len() * 2 {
        return Err("Output buffer is too small");
    }

    for (i, &byte) in input.iter().enumerate() {
        output[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        output[i * 2 + 1] = HEX_CHARS[(byte & 0x0F) as usize];
    }

    Ok(())
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

impl From<SensorCommand> for [u8; 2] {
    fn from(val: SensorCommand) -> Self {
        [((val as u16) >> 8) as u8, (val as u16) as u8]
    }
}

impl SensorCommand {
    fn to_command(self) -> [u8; 3] {
        let bytes: [u8; 2] = self.into();
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
