//! WiFi sniffer example
//!
//! Sniffs for beacon frames.

//% FEATURES: esp-wifi esp-wifi/wifi-default esp-wifi/wifi esp-wifi/utils esp-wifi/sniffer
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![allow(incomplete_features)]
#![feature(
    iter_collect_into,
    iter_array_chunks,
    array_chunks,
    generic_const_exprs
)]

mod LED;
mod battery;
mod smartled;
mod storage;
mod wifi;

extern crate alloc;

use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::gpio::{Input, Io, Output, WakeEvent};
use esp_hal::i2c::I2C;
use esp_hal::prelude::*;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::timer::AnyTimer;
use esp_hal::Async;
use esp_println::println;
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB8;
use smartled::SmartLedsAdapter;
use wifi::start_wifi;

#[embassy_executor::task]
async fn start_battery(i2c: I2C<'static, esp_hal::peripherals::I2C0, Async>) {
    let mut battery = battery::Max17048::new(i2c, 0x36).await;
    loop {
        _ = match battery.soc().await {
            Ok(soc) => println!("Battery is at {}", soc),
            Err(e) => println!("Error getting battery: {:?}", e),
        };

        Timer::after(Duration::from_secs(30)).await;
    }
}

#[esp_hal_embassy::main]
#[entry]
async fn main(spawner: embassy_executor::Spawner) {
    esp_alloc::heap_allocator!(92 * 1024);
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let rmt = Rmt::new(peripherals.RMT, 80.MHz()).unwrap();

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer: AnyTimer = timg0.timer0.into();
    let rng = Rng::new(peripherals.RNG);
    let radio_clock = peripherals.RADIO_CLK;
    let wifi = peripherals.WIFI;

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let timer1: AnyTimer = timg1.timer0.into();
    esp_hal_embassy::init(timer1);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    _ = Output::new(io.pins.gpio20, esp_hal::gpio::Level::High);

    let rmt_buffer = smartLedBuffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio9, rmt_buffer);

    LED::fade_in(
        &mut led,
        RGB8 {
            r: 0,
            g: 120,
            b: 255,
        },
        10,
    )
    .await;

    let i2c0 = I2C::new_async(peripherals.I2C0, io.pins.gpio19, io.pins.gpio18, 400.kHz());
    println!("spawning battery task");
    spawner.spawn(start_battery(i2c0)).unwrap();

    let mut button = Input::new(io.pins.gpio15, esp_hal::gpio::Pull::Down);
    button.wakeup_enable(true, WakeEvent::HighLevel);
    Timer::after(Duration::from_secs(1)).await;
    // println!("spawning wifi task");

    spawner
        .spawn(start_wifi(timer, wifi, rng, radio_clock, peripherals.LPWR))
        .unwrap();

    loop {
        button.wait_for_rising_edge().await;
        println!("button pressed!");
        Timer::after(Duration::from_millis(100)).await;
    }
}
