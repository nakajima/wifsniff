use core::cell::RefCell;

use esp_alloc as _;
use esp_backtrace as _;

use alloc::{
    collections::btree_set::BTreeSet,
    string::{String, ToString},
};
use critical_section::Mutex;
use esp_hal::{
    peripherals::{RADIO_CLK, WIFI},
    rng::Rng,
    timer::{AnyTimer, PeriodicTimer},
};
use esp_println::println;
use esp_wifi::{
    initialize,
    wifi::{self, PromiscuousPkt},
    EspWifiInitFor,
};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

use crate::storage;

static KNOWN_SSIDS: Mutex<RefCell<BTreeSet<String>>> = Mutex::new(RefCell::new(BTreeSet::new()));

#[embassy_executor::task]
pub async fn start_wifi(timer: AnyTimer, wifi: WIFI, rng: Rng, radio_clock: RADIO_CLK) {
    let timer = PeriodicTimer::new(timer);
    let init = initialize(EspWifiInitFor::Wifi, timer, rng, radio_clock).unwrap();

    // We must initialize some kind of interface and start it.
    let (_, mut controller) = wifi::new_with_mode(&init, wifi, wifi::WifiApDevice).unwrap();
    controller.start().unwrap();

    let mut sniffer = controller.take_sniffer().unwrap();
    sniffer.set_promiscuous_mode(true).unwrap();
    sniffer.set_receive_cb(callback);
}

fn callback(packet: PromiscuousPkt<'_>) {
    let _ = match_frames! {
        packet.data,
        beacon = BeaconFrame => {
            let Some(ssid) = beacon.ssid() else {
                return;
            };

            if critical_section::with(|cs| {
                if KNOWN_SSIDS.borrow_ref_mut(cs).insert(ssid.to_string()) && ssid.to_string() != "" {
                    let mut storage = storage::Store::new();
                    let byte_string = ssid.to_string();
                    let bytes = byte_string.as_bytes();
                    storage.append(bytes);
                    return true;
                } else {
                    return false;
                }


            }) {
                println!("Found new AP with SSID: {ssid}");
            }
        }
    };
}
