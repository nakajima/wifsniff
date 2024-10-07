use esp_idf_hal::gpio::*;
use anyhow::{Result};
use esp_idf_hal::{
    delay::FreeRtos,
    prelude::Peripherals,
    rmt::{config::TransmitConfig, TxRmtDriver},
};

mod led;

pub fn main() -> Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;

    // Turn on power to the LED
    let mut led_power_pin = PinDriver::output(peripherals.pins.gpio20)?;
    led_power_pin.set_high()?;

    // Onboard RGB LED pin
    let led = peripherals.pins.gpio9;
    let channel = peripherals.rmt.channel0;
    let config = TransmitConfig::new().clock_divider(1);
    let mut tx = TxRmtDriver::new(channel, led, &config)?;

    // 3 seconds white at 10% brightness
    led::neopixel(led::Rgb::new(25, 25, 25), &mut tx)?;
    FreeRtos::delay_ms(3000);

    // infinite rainbow loop at 20% brightness
    (0..360).cycle().try_for_each(|hue| {
        FreeRtos::delay_ms(10);
        let rgb = led::Rgb::from_hsv(hue, 100, 20)?;
        led::neopixel(rgb, &mut tx)
    })
}
