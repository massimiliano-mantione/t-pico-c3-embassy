use super::{AllChannelMeasurement, BitFlags, Error, Register, Tcs3472, DEVICE_ADDRESS};
use embedded_hal_1::i2c::I2c as I2cBlocking;
use embedded_hal_async::i2c::I2c as I2cAsync;

impl<I2C, E> Tcs3472<I2C>
where
    I2C: I2cBlocking<Error = E>,
{
    /// Check whether the RGB converter status is valid.
    ///
    /// Indicates that the RGBC channels have completed an integration cycle.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_rgbc_status_valid(&mut self) -> Result<bool, Error<E>> {
        let status = self.read_register(Register::STATUS)?;
        Ok((status & BitFlags::RGBC_VALID) != 0)
    }

    /// Read the clear (unfiltered) channel measurement data.
    pub fn read_clear_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::CDATA)
    }

    /// Read the red channel measurement data.
    pub fn read_red_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::RDATA)
    }

    /// Read the green channel measurement data.
    pub fn read_green_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::GDATA)
    }

    /// Read the blue channel measurement data.
    pub fn read_blue_channel(&mut self) -> Result<u16, Error<E>> {
        self.read_channel(Register::BDATA)
    }

    fn read_channel(&mut self, first_register: u8) -> Result<u16, Error<E>> {
        let mut cdata = [0; 2];
        self.read_registers(first_register, &mut cdata)?;
        Ok((u16::from(cdata[1])) << 8 | u16::from(cdata[0]))
    }

    /// Read the measurement data of all channels at once.
    pub fn read_all_channels(&mut self) -> Result<AllChannelMeasurement, Error<E>> {
        let mut data = [0; 8];
        self.read_registers(Register::CDATA, &mut data)?;
        Ok(AllChannelMeasurement {
            clear: u16::from(data[1]) << 8 | u16::from(data[0]),
            red: u16::from(data[3]) << 8 | u16::from(data[2]),
            green: u16::from(data[5]) << 8 | u16::from(data[4]),
            blue: u16::from(data[7]) << 8 | u16::from(data[6]),
        })
    }

    /// Read the device ID.
    ///
    /// The value returned corresponds to the part number identification:
    /// - `0x44` => `TCS34725`
    /// - `0x4D` => `TCS34727`
    pub fn read_device_id(&mut self) -> Result<u8, Error<E>> {
        self.read_register(Register::ID)
    }

    fn read_register(&mut self, register: u8) -> Result<u8, Error<E>> {
        let command = BitFlags::CMD | register;
        let mut data = [0];
        self.i2c
            .write_read(DEVICE_ADDRESS, &[command], &mut data)
            .map_err(Error::I2C)?;
        Ok(data[0])
    }

    fn read_registers(&mut self, first_register: u8, mut data: &mut [u8]) -> Result<(), Error<E>> {
        let command = BitFlags::CMD | BitFlags::CMD_AUTO_INC | first_register;
        self.i2c
            .write_read(DEVICE_ADDRESS, &[command], &mut data)
            .map_err(Error::I2C)
    }
}

impl<I2C, E> Tcs3472<I2C>
where
    I2C: I2cAsync<Error = E>,
{
    /// Check whether the RGB converter status is valid.
    ///
    /// Indicates that the RGBC channels have completed an integration cycle.
    #[allow(clippy::wrong_self_convention)]
    pub async fn is_rgbc_status_valid_async(&mut self) -> Result<bool, Error<E>> {
        let status = self.read_register_async(Register::STATUS).await?;
        Ok((status & BitFlags::RGBC_VALID) != 0)
    }

    /// Read the clear (unfiltered) channel measurement data.
    pub async fn read_clear_channel_async(&mut self) -> Result<u16, Error<E>> {
        self.read_channel_async(Register::CDATA).await
    }

    /// Read the red channel measurement data.
    pub async fn read_red_channel_async(&mut self) -> Result<u16, Error<E>> {
        self.read_channel_async(Register::RDATA).await
    }

    /// Read the green channel measurement data.
    pub async fn read_green_channel_async(&mut self) -> Result<u16, Error<E>> {
        self.read_channel_async(Register::GDATA).await
    }

    /// Read the blue channel measurement data.
    pub async fn read_blue_channel_async(&mut self) -> Result<u16, Error<E>> {
        self.read_channel_async(Register::BDATA).await
    }

    async fn read_channel_async(&mut self, first_register: u8) -> Result<u16, Error<E>> {
        let mut cdata = [0; 2];
        self.read_registers_async(first_register, &mut cdata)
            .await?;
        Ok((u16::from(cdata[1])) << 8 | u16::from(cdata[0]))
    }

    /// Read the measurement data of all channels at once.
    pub async fn read_all_channels_async(&mut self) -> Result<AllChannelMeasurement, Error<E>> {
        let mut data = [0; 8];
        self.read_registers_async(Register::CDATA, &mut data)
            .await?;
        Ok(AllChannelMeasurement {
            clear: u16::from(data[1]) << 8 | u16::from(data[0]),
            red: u16::from(data[3]) << 8 | u16::from(data[2]),
            green: u16::from(data[5]) << 8 | u16::from(data[4]),
            blue: u16::from(data[7]) << 8 | u16::from(data[6]),
        })
    }

    /// Read the device ID.
    ///
    /// The value returned corresponds to the part number identification:
    /// - `0x44` => `TCS34725`
    /// - `0x4D` => `TCS34727`
    pub async fn read_device_id_async(&mut self) -> Result<u8, Error<E>> {
        self.read_register_async(Register::ID).await
    }

    async fn read_register_async(&mut self, register: u8) -> Result<u8, Error<E>> {
        let command = BitFlags::CMD | register;
        let mut data = [0];
        self.i2c
            .write_read(DEVICE_ADDRESS, &[command], &mut data)
            .await
            .map_err(Error::I2C)?;
        Ok(data[0])
    }

    async fn read_registers_async(
        &mut self,
        first_register: u8,
        mut data: &mut [u8],
    ) -> Result<(), Error<E>> {
        let command = BitFlags::CMD | BitFlags::CMD_AUTO_INC | first_register;
        self.i2c
            .write_read(DEVICE_ADDRESS, &[command], &mut data)
            .await
            .map_err(Error::I2C)
    }
}
