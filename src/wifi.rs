use core::cell::RefCell;

use embassy_futures::block_on;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use esp_alloc as _;
use esp_backtrace as _;

use alloc::{
    collections::btree_set::BTreeSet,
    string::{String, ToString},
};
use critical_section::Mutex;
use esp_hal::{
    peripherals::{RADIO_CLK, WIFI},
    reset::software_reset,
    rng::Rng,
    timer::AnyTimer,
};
use esp_println::println;
use esp_wifi::{
    init,
    wifi::{new_with_mode, PromiscuousPkt, WifiApDevice},
    EspWifiInitFor,
};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

use crate::storage;

#[derive(Clone, PartialEq)]
enum WifiStatus {
    Sniffing,
    Bluetooth,
}

static WIFI_CHANNEL: PubSubChannel<CriticalSectionRawMutex, WifiStatus, 4, 4, 4> =
    PubSubChannel::<CriticalSectionRawMutex, WifiStatus, 4, 4, 4>::new();

static KNOWN_SSIDS: Mutex<RefCell<BTreeSet<String>>> = Mutex::new(RefCell::new(BTreeSet::new()));

#[embassy_executor::task]
pub async fn start_wifi(timer: AnyTimer, rng: Rng, radio_clock: RADIO_CLK, wifi: WIFI) {
    let init = init(EspWifiInitFor::Wifi, timer, rng, radio_clock).unwrap();
    println!("wifi initialized");

    // We must initialize some kind of interface and start it.
    let (_, mut controller) = new_with_mode(&init, wifi, WifiApDevice).unwrap();

    controller.start().await.unwrap();

    let mut sniffer = controller.take_sniffer().unwrap();
    sniffer.set_promiscuous_mode(true).unwrap();

    fn callback(packet: PromiscuousPkt<'_>) {
        let _ = match_frames! {
            packet.data,
            beacon = BeaconFrame => {
                let Some(ssid) = beacon.ssid() else {
                    return;
                };

                if critical_section::with(|cs| {
                    if KNOWN_SSIDS.borrow_ref_mut(cs).insert(ssid.to_string()) && ssid.to_string() != "" {
                        let byte_string = ssid.to_string();
                        let bytes = byte_string.as_bytes();
                        block_on(storage::append(bytes.to_vec()));
                        return true;
                    } else {
                        return false;
                    }
                }) {
                }
            }
        };
    }

    sniffer.set_receive_cb(callback);

    let mut subscriber = WIFI_CHANNEL.subscriber().unwrap();

    loop {
        let status = subscriber.next_message_pure().await;

        if status == WifiStatus::Sniffing {
            continue;
        }

        println!("Shutting down wifi");
        controller.disconnect().await.unwrap();
        println!("Done");
        drop(controller);

        software_reset();
        break;
    }
}
