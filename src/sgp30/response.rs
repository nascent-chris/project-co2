use core::convert::TryFrom;

use crate::crc8;

use super::Sgp30Error;

#[derive(Debug)]
pub struct ResponseBytes<const N: usize, const VERIFY_CRC: bool> {
    pub(super) data: Option<[u8; N]>,
}

impl<const N: usize, const VERIFY_CRC: bool> Default for ResponseBytes<N, VERIFY_CRC> {
    fn default() -> Self {
        Self { data: None }
    }
}

impl<const N: usize, const VERIFY_CRC: bool> ResponseBytes<N, VERIFY_CRC> {
    pub fn ignore_crc(self) -> Self {
        Self { data: self.data }
    }
}

impl TryFrom<ResponseBytes<3, true>> for u16 {
    type Error = Sgp30Error;

    fn try_from(val: ResponseBytes<3, true>) -> Result<Self, Self::Error> {
        let bytes = val.data.ok_or(Sgp30Error::MissingData)?;

        if crc8(&bytes[..2]) != bytes[2] {
            return Err(Sgp30Error::Crc);
        }

        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
}

impl TryFrom<ResponseBytes<2, false>> for u16 {
    type Error = Sgp30Error;

    fn try_from(val: ResponseBytes<2, false>) -> Result<Self, Self::Error> {
        let bytes = val.data.ok_or(Sgp30Error::MissingData)?;

        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
}

impl TryFrom<ResponseBytes<6, true>> for (u16, u16) {
    type Error = Sgp30Error;

    fn try_from(val: ResponseBytes<6, true>) -> Result<Self, Self::Error> {
        let bytes = val.data.ok_or(Sgp30Error::MissingData)?;

        if crc8(&bytes[..2]) != bytes[2] {
            return Err(Sgp30Error::Crc);
        }
        if crc8(&bytes[3..5]) != bytes[5] {
            return Err(Sgp30Error::Crc);
        }

        Ok((
            u16::from_be_bytes([bytes[0], bytes[1]]),
            u16::from_be_bytes([bytes[3], bytes[4]]),
        ))
    }
}

impl From<ResponseBytes<0, false>> for () {
    fn from(_: ResponseBytes<0, false>) -> Self {}
}

impl<const N: usize> TryFrom<ResponseBytes<N, true>> for [u8; N] {
    type Error = Sgp30Error;

    fn try_from(val: ResponseBytes<N, true>) -> Result<Self, Self::Error> {
        let bytes = val.data.ok_or(Sgp30Error::MissingData)?;

        let valid = bytes.chunks(3).all(|chunk| crc8(&chunk[..2]) == chunk[2]);
        if !valid {
            return Err(Sgp30Error::Crc);
        }

        Ok(bytes)
    }
}

impl<const N: usize> TryFrom<ResponseBytes<N, false>> for [u8; N] {
    type Error = Sgp30Error;

    fn try_from(val: ResponseBytes<N, false>) -> Result<Self, Self::Error> {
        val.data.ok_or(Sgp30Error::MissingData)
    }
}
