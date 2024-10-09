//! WiFi sniffer example
//!
//! Sniffs for beacon frames.

//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/sniffer
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]
#![allow(incomplete_features)]
#![feature(
    iter_collect_into,
    iter_array_chunks,
    array_chunks,
    generic_const_exprs
)]

mod battery;
mod led;
mod storage;
mod wifi;

extern crate alloc;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::i2c::I2C;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::Async;
use esp_hal::{gpio::Io, prelude::*, rmt::Rmt, rng::Rng, timer::AnyTimer};
use esp_println::println;
use led::start_leds;
use wifi::start_wifi;

#[embassy_executor::task]
async fn start_battery(i2c: I2C<'static, esp_hal::peripherals::I2C0, Async>) {
    let mut battery = battery::Max17048::new(i2c, 0x36).await;
    loop {
        let soc = match battery.soc().await {
            Ok(soc) => println!("Battery is at {}", soc),
            Err(e) => println!("Error getting battery: {:?}", e),
        };

        Timer::after(Duration::from_secs(60)).await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_alloc::heap_allocator!(72 * 1024);
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new_async(peripherals.RMT, 80.MHz()).unwrap();

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer: AnyTimer = timg0.timer0.into();
    let rng = Rng::new(peripherals.RNG);
    let radio_clock = peripherals.RADIO_CLK;
    let wifi = peripherals.WIFI;

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let timer1: AnyTimer = timg1.timer0.into();
    esp_hal_embassy::init(timer1);

    println!("spawning wifi task");
    spawner
        .spawn(start_wifi(timer, wifi, rng, radio_clock))
        .unwrap();

    println!("spawning LED task");
    spawner
        .spawn(start_leds(io.pins.gpio20, io.pins.gpio9, rmt))
        .unwrap();

    let i2c0 = I2C::new_async(peripherals.I2C0, io.pins.gpio19, io.pins.gpio18, 400.kHz());
    println!("spawning battery task");
    spawner.spawn(start_battery(i2c0)).unwrap();

    loop {
        let mut store = storage::Store::new();
        store.entries();
        Timer::after(Duration::from_secs(10)).await;
    }
}
