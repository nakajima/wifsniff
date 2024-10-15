use embassy_futures::join;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use embassy_time::{Duration, Timer};
use embedded_hal::digital::{OutputPin, PinState};
use esp_hal::{
    gpio::{GpioPin, Level, Output},
    Async,
};
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
    battery_low_pin: GpioPin<1>,
    is_charging_pin: GpioPin<4>,
    bluetooth_mode: GpioPin<6>,
    button_press_pin: GpioPin<5>,
) {
    let mut battery = Output::new(battery_low_pin, Level::Low);
    let mut charging = Output::new(is_charging_pin, Level::Low);
    let mut bluetooth = Output::new(bluetooth_mode, Level::Low);
    let mut button_press = Output::new(button_press_pin, Level::Low);

    let mut subscriber = LIGHTS_CHANNEL.subscriber().unwrap();

    loop {
        let change = subscriber.next_message_pure().await;

        println!("light change: {:?}", change);

        match change {
            LightChange::BatteryLow(state) => battery
                .set_state(if state { PinState::High } else { PinState::Low })
                .unwrap(),
            LightChange::IsCharging(state) => charging
                .set_state(if state { PinState::High } else { PinState::Low })
                .unwrap(),
            LightChange::BlueoothEnabled(state) => bluetooth
                .set_state(if state { PinState::High } else { PinState::Low })
                .unwrap(),
            LightChange::ButtonPressed(state) => button_press
                .set_state(if state { PinState::High } else { PinState::Low })
                .unwrap(),
        }
    }
}
