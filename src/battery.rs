use embassy_futures::{join, select};
use embassy_time::Timer;
use esp32c6::lp_aon::usb;
use esp_hal::{
    gpio::{GpioPin, Input, Pull},
    i2c::{Error, I2c},
    Async,
};
use esp_println::println;

use crate::lights::{battery_low, is_charging};

const DEFAULT_RCOMP: u8 = 0x97;

type AsyncI2C = I2c<'static, esp_hal::peripherals::I2C0, Async>;

#[embassy_executor::task]
pub async fn start_battery(i2c: AsyncI2C, usb_pin: GpioPin<16>) {
    let mut adapter = Max17048::new(i2c, 0x36).await;
    let mut usb = Input::new(usb_pin, Pull::Down);

    loop {
        println!("Battery charge rate: {:?}", adapter.charge_rate().await);
        println!("Battery SOC: {:?}", adapter.soc().await);
        println!("Battery Vcell: {:?}", adapter.vcell().await);
        println!("Is charging: {:?}", usb.get_level());

        if adapter.soc().await.unwrap() < 20 {
            battery_low(true).await;
        } else {
            battery_low(false).await;
        }

        if usb.is_high() {
            is_charging(true).await;
        } else {
            is_charging(false).await;
        }

        select::select(Timer::after_secs(5), usb.wait_for_any_edge()).await;
    }
}

pub struct Max17048 {
    i2c: AsyncI2C,
    addr: u8,
    recv_buffer: [u8; 2],
}

impl Max17048 {
    pub async fn new(i2c: AsyncI2C, addr: u8) -> Self {
        let mut max = Max17048 {
            i2c: i2c,
            addr: addr,
            recv_buffer: [0u8; 2],
        };
        let _ = max.compensation(DEFAULT_RCOMP).await;
        max
    }

    pub async fn version(&mut self) -> Result<u16, Error> {
        self.read(0x08).await
    }

    pub async fn soc(&mut self) -> Result<u16, Error> {
        match self.read(0x04).await {
            Ok(val) => Ok(val / 256),
            Err(e) => Err(e),
        }
    }

    /// Return C/Rate in %/hr
    pub async fn charge_rate(&mut self) -> Result<f32, Error> {
        match self.read(0x16).await {
            Ok(val) => Ok(val as f32 * 0.208),
            Err(e) => Err(e),
        }
    }

    pub async fn vcell(&mut self) -> Result<f32, Error> {
        match self.read(0x02).await {
            Ok(val) => Ok(val as f32 * 0.000078125),
            Err(e) => Err(e),
        }
    }

    pub async fn temp_compensation(&mut self, temp: f32) -> Result<(), Error> {
        let rcomp = if temp > 20.0 {
            DEFAULT_RCOMP as f32 + (temp - 20.0) * -0.5
        } else {
            DEFAULT_RCOMP as f32 + (temp - 20.0) * -5.0
        };
        self.compensation(rcomp as u8).await
    }

    async fn compensation(&mut self, rcomp: u8) -> Result<(), Error> {
        // read the current reg vals
        match self.read(0x0C).await {
            Ok(mut value) => {
                value &= 0x00FF;
                value |= (rcomp as u16) << 8;
                // write to the rcomp bits only
                self.write(0x0C, value).await?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn read(&mut self, reg: u8) -> Result<u16, Error> {
        match self
            .i2c
            .write_read(self.addr, &[reg], &mut self.recv_buffer)
            .await
        {
            Ok(_) => Ok((self.recv_buffer[0] as u16) << 8 | self.recv_buffer[1] as u16),
            Err(e) => Err(e),
        }
    }

    async fn write(&mut self, reg: u8, value: u16) -> Result<(), Error> {
        self.i2c.write(self.addr, &[reg]).await?;
        let msb = ((value & 0xFF00) >> 8) as u8;
        let lsb = ((value & 0x00FF) >> 0) as u8;
        self.i2c.write(self.addr, &[msb, lsb]).await?;
        Ok(())
    }
}
