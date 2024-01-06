pub mod commands;
pub mod params;
pub mod response;

use core::convert::TryInto;

use cortex_m::{
    asm,
    prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write as _},
};
use stm32f1xx_hal::i2c::{self, BlockingI2c};

pub enum Sgp30Error {
    Crc,
    I2c(i2c::Error),
    MissingData,
    InvalidCommandCode(u16),
}

use self::{
    commands::{
        command,
        CommandCode::{self, *},
    },
    params::ParamBytes,
    response::ResponseBytes,
};

pub struct Sgp30<I2C> {
    i2c: I2C,
    address: u8,
}

pub struct CommandBuilder<
    'a,
    const PARAM_SIZE: usize,
    const RESPONSE_SIZE: usize,
    const VERIFY_CRC: bool,
    I2C,
> {
    i2c: &'a mut Sgp30<I2C>,
    params: Option<ParamBytes<PARAM_SIZE>>,
    code: CommandCode,
}

impl<
        'a,
        const PARAM_SIZE: usize,
        const RESPONSE_SIZE: usize,
        const VERIFY_CRC: bool,
        I2C,
        PINS,
    > CommandBuilder<'a, PARAM_SIZE, RESPONSE_SIZE, VERIFY_CRC, BlockingI2c<I2C, PINS>>
where
    I2C: i2c::Instance,
{
    pub fn new(
        i2c: &'a mut Sgp30<BlockingI2c<I2C, PINS>>,
        code: CommandCode,
    ) -> CommandBuilder<'a, PARAM_SIZE, RESPONSE_SIZE, VERIFY_CRC, BlockingI2c<I2C, PINS>> {
        CommandBuilder {
            i2c,
            params: None,
            code,
        }
    }

    pub fn params(self, params: impl Into<ParamBytes<PARAM_SIZE>>) -> Self {
        CommandBuilder {
            i2c: self.i2c,
            params: Some(params.into()),
            code: self.code,
        }
    }

    pub fn verify_response_crc(
        self,
    ) -> CommandBuilder<'a, PARAM_SIZE, RESPONSE_SIZE, true, BlockingI2c<I2C, PINS>> {
        CommandBuilder {
            i2c: self.i2c,
            params: self.params,
            code: self.code,
        }
    }

    pub fn ignore_response_crc(
        self,
    ) -> CommandBuilder<'a, PARAM_SIZE, RESPONSE_SIZE, false, BlockingI2c<I2C, PINS>> {
        CommandBuilder {
            i2c: self.i2c,
            params: self.params,
            code: self.code,
        }
    }

    pub fn exec(self) -> Result<ResponseBytes<RESPONSE_SIZE, VERIFY_CRC>, Sgp30Error> {
        self.i2c
            .exec_cmd::<PARAM_SIZE, RESPONSE_SIZE, VERIFY_CRC>(self.code, self.params)
    }
}

impl<I2C, PINS> Sgp30<BlockingI2c<I2C, PINS>>
where
    I2C: i2c::Instance,
{
    fn exec_cmd<const PARAM_SIZE: usize, const RESPONSE_SIZE: usize, const VERIFY_CRC: bool>(
        &mut self,
        cmd: CommandCode,
        params: Option<ParamBytes<PARAM_SIZE>>,
    ) -> Result<ResponseBytes<RESPONSE_SIZE, VERIFY_CRC>, Sgp30Error> {
        let mut output_buffer = [0u8; RESPONSE_SIZE];
        let cmd = command(cmd);
        let cmd_data = cmd.with_crc();

        self.i2c
            .write(self.address, &cmd_data)
            .map_err(Sgp30Error::I2c)?;

        // if let Some(params) = params {
        if PARAM_SIZE > 0 {
            let params = params.ok_or(Sgp30Error::MissingData)?;

            self.i2c
                .write(self.address, &params.0)
                .map_err(Sgp30Error::I2c)?;
        }

        // if the response length is set, we're expecting a response
        if RESPONSE_SIZE > 0 {
            asm::delay(cmd.duration_cycles);
            self.i2c
                .read(self.address, &mut output_buffer[..RESPONSE_SIZE])
                .map_err(Sgp30Error::I2c)?;

            Ok(ResponseBytes {
                data: Some(output_buffer),
            })
        } else {
            Ok(ResponseBytes::default())
        }
    }

    /// Param length: 0 bytes, Response length: 0 bytes
    pub fn init_air_quality(&mut self) -> Result<(), Sgp30Error> {
        CommandBuilder::<0, 0, true, _>::new(self, InitAirQuality)
            .ignore_response_crc()
            .exec()
            .map(Into::into)
    }

    /// Param length: 0 bytes, Response length: 9 bytes
    pub fn get_serial_id(&mut self) -> Result<[u8; 9], Sgp30Error> {
        CommandBuilder::<0, 9, true, _> {
            i2c: self,
            code: GetSerialId,
            params: None,
        }
        .exec()?
        .try_into()
    }

    /// Param length: 0 bytes, Response length: 6 bytes
    pub fn measure_air_quality(&mut self) -> Result<(u16, u16), Sgp30Error> {
        CommandBuilder::<0, 6, true, _> {
            i2c: self,
            code: MeasureAirQuality,
            params: None,
        }
        .exec()?
        .try_into()
    }

    /// Param length: 0 bytes, Response length: 6 bytes
    pub fn get_baseline(&mut self) -> Result<[u8; 6], Sgp30Error> {
        CommandBuilder::<0, 6, true, _> {
            i2c: self,
            code: GetBaseline,
            params: None,
        }
        .exec()?
        .try_into()
    }

    /// Param length: 6 bytes, Response length: 0 bytes
    pub fn set_baseline(&mut self) -> Result<(), Sgp30Error> {
        CommandBuilder::<6, 0, true, _>::new(self, SetBaseline)
            .ignore_response_crc()
            .exec()
            .map(Into::into)
    }

    /// Param length: 3 bytes, Response length: 0 bytes
    pub fn set_humidity(&mut self, params: impl Into<ParamBytes<3>>) -> Result<(), Sgp30Error> {
        CommandBuilder::<3, 0, true, _>::new(self, SetHumidity)
            .ignore_response_crc()
            .params(params)
            .exec()
            .map(Into::into)
    }

    /// Param length: 0 bytes, Response length: 3 bytes
    pub fn measure_test(&mut self) -> Result<[u8; 3], Sgp30Error> {
        CommandBuilder::<0, 3, true, _> {
            i2c: self,
            code: MeasureTest,
            params: None,
        }
        .exec()?
        .try_into()
    }

    /// Param length: 0 bytes, Response length: 3 bytes
    pub fn get_feature_set_ver(&mut self) -> Result<[u8; 3], Sgp30Error> {
        CommandBuilder::<0, 3, true, _> {
            i2c: self,
            code: GetFeatureSetVer,
            params: None,
        }
        .exec()?
        .try_into()
    }

    /// Param length: 0 bytes, Response length: 6 bytes
    pub fn measure_raw_signals(&mut self) -> Result<[u8; 6], Sgp30Error> {
        CommandBuilder::<0, 6, true, _> {
            i2c: self,
            code: MeasureRawSignals,
            params: None,
        }
        .exec()?
        .try_into()
    }
}
