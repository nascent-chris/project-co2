use core::convert::TryFrom;

use crate::crc8;

pub struct ParamBytes<const N: usize>(pub(super) [u8; N]);

impl From<()> for ParamBytes<0> {
    fn from(_: ()) -> Self {
        Self([])
    }
}

impl From<u16> for ParamBytes<3> {
    fn from(v: u16) -> Self {
        let mut bytes = [0u8; 3];
        bytes[0] = (v >> 8) as u8;
        bytes[1] = v as u8;
        bytes[2] = crc8(&bytes[..2]);
        Self(bytes)
    }
}

impl From<(u16, u16)> for ParamBytes<6> {
    fn from((v1, v2): (u16, u16)) -> Self {
        let mut bytes = [0u8; 6];
        bytes[0] = (v1 >> 8) as u8;
        bytes[1] = v1 as u8;
        bytes[2] = crc8(&bytes[..2]);
        bytes[3] = (v2 >> 8) as u8;
        bytes[4] = v2 as u8;
        bytes[5] = crc8(&bytes[3..5]);
        Self(bytes)
    }
}

impl From<(u16, u16, u16)> for ParamBytes<9> {
    fn from((v1, v2, v3): (u16, u16, u16)) -> Self {
        let mut bytes = [0u8; 9];
        bytes[0] = (v1 >> 8) as u8;
        bytes[1] = v1 as u8;
        bytes[2] = crc8(&bytes[..2]);
        bytes[3] = (v2 >> 8) as u8;
        bytes[4] = v2 as u8;
        bytes[5] = crc8(&bytes[3..5]);
        bytes[6] = (v3 >> 8) as u8;
        bytes[7] = v3 as u8;
        bytes[8] = crc8(&bytes[6..8]);
        Self(bytes)
    }
}

impl<const N: usize> TryFrom<&[u8]> for ParamBytes<N> {
    type Error = ();

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if N % 3 != 0 {
            return Err(());
        }
        if bytes.len() % 2 != 0 {
            return Err(());
        }
        let mut output = [0u8; N];
        for (i, chunk) in bytes.chunks(2).enumerate() {
            output[i * 3] = chunk[0];
            output[i * 3 + 1] = chunk[1];
            output[i * 3 + 2] = crc8(chunk);
        }
        Ok(Self(output))
    }
}
