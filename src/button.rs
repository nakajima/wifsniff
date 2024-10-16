use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{publisher, PubSubChannel},
};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{GpioPin, Input};
use esp_println::println;

use crate::lights;

#[derive(Clone)]
enum ButtonPress {
    Single,
    Long,
}

pub static BUTTON_CHANNEL: PubSubChannel<CriticalSectionRawMutex, ButtonPress, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, ButtonPress, 4, 4, 4>::new();

#[embassy_executor::task]
pub async fn button_task(mut button: Input<'static, GpioPin<17>>) {
    let publisher = BUTTON_CHANNEL.publisher().unwrap();
    loop {
        button.wait_for_rising_edge().await;
        lights::change(lights::LightChange::White(true)).await;

        let mut is_long_press = true;
        for _ in 0..=200 {
            if button.is_low() {
                is_long_press = false;
                break;
            }

            Timer::after(Duration::from_millis(10)).await;
        }

        lights::change(lights::LightChange::White(false)).await;

        if is_long_press {
            publisher.publish(ButtonPress::Long).await;
        } else {
            publisher.publish(ButtonPress::Single).await;
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}
