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
pub enum LightChange {
    Blue(bool),
    Green(bool),
    Yellow(bool),
    White(bool),
}

pub async fn change(light_change: LightChange) {
    println!("Change: {:?}", light_change);
    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(light_change)
        .await;
}

pub async fn off() {
    change(LightChange::White(false)).await;
    change(LightChange::Yellow(false)).await;
    change(LightChange::Green(false)).await;
    change(LightChange::Blue(false)).await;
}

static LIGHTS_CHANNEL: PubSubChannel<CriticalSectionRawMutex, LightChange, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, LightChange, 4, 4, 4>::new();

#[embassy_executor::task]
pub async fn setup_lights(
    mut ledc: Ledc<'static>,
    yellow_pin: GpioPin<1>,
    green_pin: GpioPin<4>,
    blue_pin: GpioPin<6>,
    white_pin: GpioPin<5>,
) {
    let yellow_output = Output::new(yellow_pin, Level::Low);
    let green_output = Output::new(green_pin, Level::Low);
    let blue_output = Output::new(blue_pin, Level::Low);
    let white_output = Output::new(white_pin, Level::Low);

    // Setup LEDC
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut lstimer0 = ledc.get_timer::<LowSpeed>(timer::Number::Timer0);
    lstimer0
        .configure(timer::config::Config {
            duty: timer::config::Duty::Duty10Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: 24.kHz(),
        })
        .unwrap();

    let mut yellow = ledc.get_channel(channel::Number::Channel0, yellow_output);
    yellow
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 24,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut green = ledc.get_channel(channel::Number::Channel1, green_output);
    green
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 24,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut blue = ledc.get_channel(channel::Number::Channel2, blue_output);
    blue.configure(channel::config::Config {
        timer: &lstimer0,
        duty_pct: 24,
        pin_config: channel::config::PinConfig::PushPull,
    })
    .unwrap();

    let mut white = ledc.get_channel(channel::Number::Channel3, white_output);
    white
        .configure(channel::config::Config {
            timer: &lstimer0,
            duty_pct: 24,
            pin_config: channel::config::PinConfig::PushPull,
        })
        .unwrap();

    let mut subscriber = LIGHTS_CHANNEL.subscriber().unwrap();

    blue.set_duty(0).unwrap();
    green.set_duty(0).unwrap();
    yellow.set_duty(0).unwrap();
    white.set_duty(0).unwrap();

    loop {
        let change = subscriber.next_message_pure().await;

        match change {
            LightChange::Yellow(state) => {
                if state {
                    yellow.start_duty_fade(0, 40, 400).unwrap();
                } else {
                    yellow.start_duty_fade(40, 0, 400).unwrap();
                }
            }
            LightChange::Green(state) => {
                if state {
                    green.start_duty_fade(0, 20, 128).unwrap();
                } else {
                    green.start_duty_fade(20, 0, 128).unwrap();
                }
            }
            LightChange::Blue(state) => {
                if state {
                    blue.start_duty_fade(0, 20, 40).unwrap();
                } else {
                    blue.start_duty_fade(20, 0, 40).unwrap();
                }
            }
            LightChange::White(state) => {
                if state {
                    white.start_duty_fade(0, 20, 40).unwrap();
                } else {
                    white.start_duty_fade(20, 0, 40).unwrap();
                }
            }
        }
    }
}
