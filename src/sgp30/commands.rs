pub struct Command {
    pub code: u16,
    pub duration_cycles: u32,
    pub max_duration_cycles: u32,
}

impl Command {
    pub fn with_crc(&self) -> [u8; 3] {
        let bytes = self.without_crc();
        [bytes[0], bytes[1], crc8(&bytes)]
    }

    pub fn without_crc(&self) -> [u8; 2] {
        [(self.code >> 8) as u8, self.code as u8]
    }
}

impl From<CommandCode> for [u8; 2] {
    fn from(val: CommandCode) -> Self {
        command(val).without_crc()
    }
}

impl From<CommandCode> for [u8; 3] {
    fn from(val: CommandCode) -> Self {
        command(val).with_crc()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum CommandCode {
    InitAirQuality = 0,
    MeasureAirQuality,
    GetBaseline,
    SetBaseline,
    SetHumidity,
    MeasureTest,
    GetFeatureSetVer,
    MeasureRawSignals,
    GetSerialId,
}

impl CommandCode {
    pub fn with_crc(&self) -> [u8; 3] {
        command(*self).with_crc()
    }

    pub fn without_crc(&self) -> [u8; 2] {
        command(*self).without_crc()
    }
}

pub(super) fn command(code: CommandCode) -> &'static Command {
    &COMMANDS[code as usize]
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
| Get_serial_id        | 0x3682  | -                               | 9                               | 1                   | 2       |
+----------------------+---------+---------------------------------+---------------------------------+---------------------+---------+
*/

static COMMANDS: [Command; 9] = [
    // Init_air_quality
    Command {
        code: 0x2003,
        duration_cycles: 2,
        max_duration_cycles: 10,
    },
    // Measure_air_quality
    Command {
        code: 0x2008,
        duration_cycles: 10,
        max_duration_cycles: 12,
    },
    // Get_baseline
    Command {
        code: 0x2015,
        duration_cycles: 10,
        max_duration_cycles: 10,
    },
    // Set_baseline
    Command {
        code: 0x201e,
        duration_cycles: 10,
        max_duration_cycles: 10,
    },
    // Set_humidity
    Command {
        code: 0x2061,
        duration_cycles: 1,
        max_duration_cycles: 10,
    },
    // Measure_test
    Command {
        code: 0x2032,
        duration_cycles: 200,
        max_duration_cycles: 220,
    },
    // Get_feature_set_ver
    Command {
        code: 0x202f,
        duration_cycles: 1,
        max_duration_cycles: 2,
    },
    // Measure_raw_signals
    Command {
        code: 0x2050,
        duration_cycles: 20,
        max_duration_cycles: 25,
    },
    // Get_serial_id
    Command {
        code: 0x3682,
        duration_cycles: 1,
        max_duration_cycles: 2,
    },
];

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
