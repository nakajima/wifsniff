use core::{
    borrow::{Borrow, BorrowMut},
    cell::{RefCell, RefMut},
    ops::Deref,
};

use alloc::{borrow::ToOwned, boxed::Box};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use esp_hal::{
    gpio::{AnyPin, GpioPin, Level, Output},
    ledc::{channel::Channel, timer, LSGlobalClkSource, Ledc, LowSpeed},
    prelude::_esp_hal_ledc_timer_TimerIFace,
};
use esp_hal::{ledc::channel, prelude::*};

#[derive(Clone, Debug)]
pub enum Color {
    Blue,
    Green,
    Yellow,
    White,
}

#[derive(Clone, Debug)]
struct LightChange {
    color: Color,
    brightness: u8,
    duration: u16,
}

pub async fn change(light: Color, enabled: bool) {
    let light_change = LightChange {
        color: light,
        brightness: if enabled { 20 } else { 0 },
        duration: 64,
    };

    LIGHTS_CHANNEL
        .publisher()
        .unwrap()
        .publish(light_change)
        .await;
}

pub async fn on(light: Color) {
    change(light, true).await;
}

pub async fn off(light: Color) {
    change(light, false).await;
}

pub async fn all_off() {
    change(Color::White, false).await;
    change(Color::Yellow, false).await;
    change(Color::Green, false).await;
    change(Color::Blue, false).await;
}

struct Light {
    brightness: u8,
    channel: Channel<'static, LowSpeed, AnyPin>,
}
impl Light {
    fn new(
        mut channel: Channel<'static, LowSpeed, AnyPin>,
        timer: &'static timer::Timer<'static, LowSpeed>,
    ) -> Self {
        channel
            .configure(channel::config::Config {
                timer: timer,
                duty_pct: 24,
                pin_config: channel::config::PinConfig::PushPull,
            })
            .unwrap();

        Self {
            brightness: 0,
            channel,
        }
    }

    async fn apply(&mut self, change: LightChange) {
        if change.brightness == self.brightness {
            // We're already there, no need to do anything
            return;
        }

        self.channel
            .start_duty_fade(self.brightness, change.brightness, change.duration)
            .unwrap();
        self.brightness = change.brightness;
    }
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
) -> ! {
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

    let timer = Box::leak(Box::new(lstimer0));

    let yellow_channel = ledc.get_channel(channel::Number::Channel0, yellow_output);
    let mut yellow = Light::new(yellow_channel, timer);

    let green_channel = ledc.get_channel(channel::Number::Channel1, green_output);
    let mut green = Light::new(green_channel, timer);

    let blue_channel = ledc.get_channel(channel::Number::Channel2, blue_output);
    let mut blue = Light::new(blue_channel, timer);

    let white_channel = ledc.get_channel(channel::Number::Channel3, white_output);
    let mut white = Light::new(white_channel, timer);

    let mut subscriber = LIGHTS_CHANNEL.subscriber().unwrap();

    loop {
        let change = subscriber.next_message_pure().await;

        match change.color {
            Color::White => white.apply(change).await,
            Color::Yellow => yellow.apply(change).await,
            Color::Green => green.apply(change).await,
            Color::Blue => blue.apply(change).await,
        }
    }
}
