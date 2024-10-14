use embassy_futures::join;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{PubSubChannel, Publisher},
};
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{GpioPin, Level, Output},
    Async,
};
use esp_println::println;

static BATTERY_LOW_CHANNEL: PubSubChannel<CriticalSectionRawMutex, bool, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, bool, 4, 4, 4>::new();

static IS_CHARGING_CHANNEL: PubSubChannel<CriticalSectionRawMutex, bool, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, bool, 4, 4, 4>::new();

static BLUETOOTH_CHANNEL: PubSubChannel<CriticalSectionRawMutex, bool, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, bool, 4, 4, 4>::new();

pub async fn battery_low(state: bool) {
    BATTERY_LOW_CHANNEL
        .publisher()
        .unwrap()
        .publish(state)
        .await;
}

pub async fn is_charging(state: bool) {
    IS_CHARGING_CHANNEL
        .publisher()
        .unwrap()
        .publish(state)
        .await;
}

pub async fn bluetooth_enabled(state: bool) {
    BLUETOOTH_CHANNEL.publisher().unwrap().publish(state).await;
}

async fn battery_future(io: &mut Output<'_>) {
    let mut subscriber = BATTERY_LOW_CHANNEL.subscriber().unwrap();
    let change = subscriber.next_message_pure().await;
    if change {
        println!("is low battery");
        io.set_high();
    } else {
        println!("is not charging");
        io.set_low();
    }
}

async fn charging_future(io: &mut Output<'_>) {
    let mut subscriber = IS_CHARGING_CHANNEL.subscriber().unwrap();
    let change = subscriber.next_message_pure().await;
    if change {
        println!("is charging");
        io.set_high();
    } else {
        println!("not charging");
        io.set_low();
    }
}

async fn bluetooth_future(io: &mut Output<'_>) {
    let mut subscriber = BLUETOOTH_CHANNEL.subscriber().unwrap();
    let change = subscriber.next_message_pure().await;
    if change {
        println!("is bluetooth");
        io.set_high();
    } else {
        println!("not bluetooth");
        io.set_low();
    }
}

#[embassy_executor::task]
pub async fn setup_lights(
    battery_low_pin: GpioPin<1>,
    is_charging_pin: GpioPin<4>,
    bluetooth_mode: GpioPin<6>,
) {
    let mut battery = Output::new(battery_low_pin, Level::Low);
    let mut charging = Output::new(is_charging_pin, Level::Low);
    let mut bluetooth = Output::new(bluetooth_mode, Level::Low);

    loop {
        join::join3(
            battery_future(&mut battery),
            charging_future(&mut charging),
            bluetooth_future(&mut bluetooth),
        )
        .await;
    }
}
