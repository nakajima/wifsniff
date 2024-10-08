use core::cell::RefCell;

use esp_alloc as _;
use esp_backtrace as _;

use alloc::{
    collections::btree_set::BTreeSet,
    string::{String, ToString},
};
use critical_section::Mutex;
use esp_hal::{
    clock::CpuClock,
    peripherals::{Peripherals, RADIO_CLK, WIFI},
    rng::Rng,
    timer::{timg::TimerGroup, AnyTimer, PeriodicTimer},
};
use esp_println::println;
use esp_wifi::{initialize, wifi, EspWifiInitFor};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};

static KNOWN_SSIDS: Mutex<RefCell<BTreeSet<String>>> = Mutex::new(RefCell::new(BTreeSet::new()));

pub fn start_wifi(timer: AnyTimer, wifi: WIFI, rng: Rng, radio_clock: RADIO_CLK) {
    // let peripherals = esp_hal::init({
    //     let mut config = esp_hal::Config::default();
    //     config.cpu_clock = CpuClock::max();
    //     config
    // });

    let timer = PeriodicTimer::new(timer);
    let init = initialize(EspWifiInitFor::Wifi, timer, rng, radio_clock).unwrap();

    // We must initialize some kind of interface and start it.
    let (_, mut controller) = wifi::new_with_mode(&init, wifi, wifi::WifiApDevice).unwrap();
    controller.start().unwrap();

    let mut sniffer = controller.take_sniffer().unwrap();
    sniffer.set_promiscuous_mode(true).unwrap();
    sniffer.set_receive_cb(|packet| {
        let _ = match_frames! {
            packet.data,
            beacon = BeaconFrame => {
                let Some(ssid) = beacon.ssid() else {
                    return;
                };
                if critical_section::with(|cs| {
                    KNOWN_SSIDS.borrow_ref_mut(cs).insert(ssid.to_string())
                }) {
                    println!("Found new AP with SSID: {ssid}");
                }
            }
        };
    });
}
