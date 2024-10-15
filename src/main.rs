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

mod battery;
mod bluetooth;
mod lights;
mod storage;
mod wifi;

extern crate alloc;

use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::gpio::{GpioPin, Input, Io, Output, Pull};
use esp_hal::i2c::I2c;
use esp_hal::ledc::Ledc;
use esp_hal::prelude::*;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::timer::AnyTimer;
use esp_println::println;
use lights::{battery_low, bluetooth_enabled, button_pressed, is_charging, setup_lights};
use wifi::start_wifi;

#[esp_hal_embassy::main]
async fn main(spawner: embassy_executor::Spawner) {
    esp_alloc::heap_allocator!(64 * 1024);
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer: AnyTimer = timg0.timer0.into();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    _ = Output::new(io.pins.gpio20, esp_hal::gpio::Level::High);

    let ledc = Ledc::new(peripherals.LEDC);

    spawner
        .spawn(setup_lights(
            ledc,
            io.pins.gpio1,
            io.pins.gpio4,
            io.pins.gpio6,
            io.pins.gpio5,
        ))
        .ok();

    Timer::after_millis(100).await;
    bluetooth_enabled(true).await;
    Timer::after_millis(100).await;
    is_charging(true).await;
    Timer::after_millis(100).await;
    battery_low(true).await;
    Timer::after_millis(100).await;
    button_pressed(true).await;
    Timer::after_millis(300).await;

    // storage::Store::reset();

    // MARK: Light task

    battery_low(false).await;
    bluetooth_enabled(false).await;
    is_charging(false).await;
    button_pressed(false).await;

    let i2c0 = I2c::new_async(peripherals.I2C0, io.pins.gpio19, io.pins.gpio18, 400.kHz());
    println!("spawning battery task");
    spawner
        .spawn(battery::start_battery(i2c0, io.pins.gpio16))
        .unwrap();

    let button = Input::new_typed(io.pins.gpio17, Pull::Down);
    let button_is_high = button.is_high();
    spawner.spawn(button_task(button)).unwrap();

    if button_is_high {
        lights::bluetooth_enabled(true).await;
        spawner
            .spawn(bluetooth::start_bluetooth(
                timer,
                Rng::new(peripherals.RNG),
                peripherals.RADIO_CLK,
                peripherals.BT,
            ))
            .unwrap();
    } else {
        spawner
            .spawn(start_wifi(
                timer,
                Rng::new(peripherals.RNG),
                peripherals.RADIO_CLK,
                peripherals.WIFI,
            ))
            .unwrap();
    }

    println!("We know about:");
    storage::Store::new().dump();

    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static, GpioPin<17>>) {
    loop {
        button.wait_for_rising_edge().await;
        button_pressed(true).await;

        let mut is_long_press = true;
        for _ in 0..=200 {
            if button.is_low() {
                is_long_press = false;
                break;
            }

            Timer::after(Duration::from_millis(10)).await;
        }

        button_pressed(false).await;

        if is_long_press {
            println!("Is a long press");
            // restart to put in bluetooth mode
            // software_reset();

            for _ in 0..=5 {
                bluetooth_enabled(true).await;
                Timer::after(Duration::from_millis(200)).await;
                bluetooth_enabled(false).await;
                Timer::after(Duration::from_millis(200)).await;
            }
        } else {
            println!("Not a long press");
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}
