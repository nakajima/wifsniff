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

mod led;
mod wifi;

extern crate alloc;

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    gpio::Io,
    prelude::*,
    rmt::{Rmt, TxChannelConfig},
    rng::Rng,
    timer::{timg::TimerGroup, AnyTimer, PeriodicTimer},
};
use led::start_leds;
use wifi::start_wifi;

#[entry]
fn main() -> ! {
    esp_alloc::heap_allocator!(72 * 1024);
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init({
        let mut config = esp_hal::Config::default();
        config.cpu_clock = CpuClock::max();
        config
    });

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let rmt = Rmt::new(peripherals.RMT, 80.MHz()).unwrap();

    start_leds(io, rmt);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer: AnyTimer = timg0.timer0.into();
    let rng = Rng::new(peripherals.RNG);
    let radio_clock = peripherals.RADIO_CLK;
    let wifi = peripherals.WIFI;

    start_wifi(timer, wifi, rng, radio_clock);

    loop {}
}
