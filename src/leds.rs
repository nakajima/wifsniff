use embassy_time::{Duration, Timer};
use esp_hal::rmt::TxChannel;
use esp_println::println;
use smart_leds::{brightness, gamma, SmartLedsWrite, RGB8};

use crate::smartled::SmartLedsAdapter;

pub fn fade_in<TX: TxChannel, const BUFFER_SIZE: usize>(
    led: &mut SmartLedsAdapter<TX, BUFFER_SIZE>,
    color: RGB8,
    level: u8,
) {
    let data = [color];
    led.color = color;
    led.bright = level;

    led.write(brightness(gamma(data.iter().cloned()), level))
        .unwrap();
}

pub async fn fade_out<TX: TxChannel, const BUFFER_SIZE: usize>(
    led: &mut SmartLedsAdapter<TX, BUFFER_SIZE>,
) {
    for i in 0..=led.bright {
        led.write(brightness(
            gamma([led.color].iter().cloned()),
            led.bright - i,
        ))
        .unwrap();
        println!("i = {:?}", i);
        Timer::after(Duration::from_millis(20)).await;
    }

    led.bright = 0
}
