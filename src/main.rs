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

mod leds;
mod smartled;
mod storage;
mod wifi;

extern crate alloc;

use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::gpio::{GpioPin, Input, Io, Output, Pull};
use esp_hal::prelude::*;
use esp_hal::reset::software_reset;
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::timer::AnyTimer;
use esp_println::println;
use smart_leds::RGB8;
use smartled::SmartLedsAdapter;
use wifi::{start_bluetooth, start_wifi};

#[esp_hal_embassy::main]
async fn main(spawner: embassy_executor::Spawner) {
    esp_alloc::heap_allocator!(64 * 1024);
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer: AnyTimer = timg0.timer0.into();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let rmt = Rmt::new(peripherals.RMT, 80.MHz()).unwrap();

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    _ = Output::new(io.pins.gpio20, esp_hal::gpio::Level::High);

    let rmt_buffer = smartLedBuffer!(1);
    let mut led = SmartLedsAdapter::new(rmt.channel0, io.pins.gpio9, rmt_buffer);

    // let i2c0 = I2C::new_async(peripherals.I2C0, io.pins.gpio19, io.pins.gpio18, 400.kHz());
    // println!("spawning battery task");
    // spawner.spawn(start_battery(i2c0)).unwrap();

    let button = Input::new_typed(io.pins.gpio7, Pull::Down);
    let button_is_high = button.is_high();
    spawner.spawn(button_task(button)).unwrap();

    if button_is_high {
        leds::fade_in(&mut led, RGB8 { r: 0, g: 0, b: 30 }, 100);

        spawner
            .spawn(start_bluetooth(
                timer,
                Rng::new(peripherals.RNG),
                peripherals.RADIO_CLK,
                peripherals.BT,
                led,
            ))
            .unwrap();
    } else {
        spawner
            .spawn(start_wifi(
                timer,
                Rng::new(peripherals.RNG),
                peripherals.RADIO_CLK,
                peripherals.WIFI,
                led,
            ))
            .unwrap();
    }

    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static, GpioPin<7>>) {
    loop {
        button.wait_for_rising_edge().await;

        let mut is_long_press = true;
        for _ in 0..=200 {
            if button.is_low() {
                is_long_press = false;
                break;
            }

            Timer::after(Duration::from_millis(10)).await;
        }

        if is_long_press {
            println!("Is a long press");
            // restart to put in bluetooth mode
            software_reset();
        } else {
            println!("Not a long press");
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}
