use embassy_futures::join;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use embedded_hal::digital::{OutputPin, PinState};
use esp_hal::{
    gpio::{GpioPin, Level, Output},
    ledc::{timer, LSGlobalClkSource, Ledc, LowSpeed},
    prelude::_esp_hal_ledc_timer_TimerIFace,
};
use esp_hal::{ledc::channel, prelude::*};
use esp_println::println;

#[derive(Clone, Debug)]
enum LightChange {
    BatteryLow(bool),
    IsCharging(bool),
    BlueoothEnabled(bool),
    ButtonPressed(bool),
}

static LIGHTS_CHANNEL: PubSubChannel<CriticalSectionRawMutex, LightChange, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, LightChange, 4, 4, 4>::new();

pub async fn battery_low(state: bool) {
    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(LightChange::BatteryLow(state))
        .await;
}

pub async fn is_charging(state: bool) {
    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(LightChange::IsCharging(state))
        .await;
}

pub async fn bluetooth_enabled(state: bool) {
    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(LightChange::BlueoothEnabled(state))
        .await;
}

pub async fn button_pressed(state: bool) {
    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(LightChange::ButtonPressed(state))
        .await;
}

#[embassy_executor::task]
pub async fn setup_lights(
    mut ledc: Ledc<'static>,
    battery_low_pin: GpioPin<1>,
    is_charging_pin: GpioPin<4>,
    bluetooth_mode: GpioPin<6>,
    button_press_pin: GpioPin<5>,
) {
    let battery_output = Output::new(battery_low_pin, Level::Low);
    let charging_output = Output::new(is_charging_pin, Level::Low);
    let bluetooth_output = Output::new(bluetooth_mode, Level::Low);
    let button_press_output = Output::new(button_press_pin, Level::Low);

    // Setup LEDC
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut lstimer0 = ledc.get_timer::<LowSpeed>(timer::Number::Timer0);
    lstimer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty5Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: 24.kHz(),
        })
        .unwrap();

    let mut battery = ledc.get_channel(channel::Number::Channel0, battery_output);
    battery
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 10,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut charging = ledc.get_channel(channel::Number::Channel1, charging_output);
    charging
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 10,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut bluetooth = ledc.get_channel(channel::Number::Channel2, bluetooth_output);
    bluetooth
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 10,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut button_press = ledc.get_channel(channel::Number::Channel3, button_press_output);
    button_press
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 10,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut subscriber = LIGHTS_CHANNEL.subscriber().unwrap();

    loop {
        let change = subscriber.next_message_pure().await;

        println!("light change: {:?}", change);

        match change {
            LightChange::BatteryLow(state) => {
                if state {
                    battery.start_duty_fade(0, 40, 400).unwrap();
                } else {
                    battery.start_duty_fade(40, 0, 400).unwrap();
                }
            }
            LightChange::IsCharging(state) => {
                if state {
                    charging.start_duty_fade(0, 20, 128).unwrap();
                } else {
                    charging.start_duty_fade(20, 0, 128).unwrap();
                }
            }
            LightChange::BlueoothEnabled(state) => {
                if state {
                    bluetooth.start_duty_fade(0, 20, 40).unwrap();
                } else {
                    bluetooth.start_duty_fade(20, 0, 40).unwrap();
                }
            }
            LightChange::ButtonPressed(state) => {
                if state {
                    button_press.start_duty_fade(0, 20, 40).unwrap();
                } else {
                    button_press.start_duty_fade(20, 0, 40).unwrap();
                }
            }
        }
    }
}
