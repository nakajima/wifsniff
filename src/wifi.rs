use core::cell::RefCell;

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    attribute_server::{AttributeServer, WorkResult},
    gatt, Ble, HciConnector,
};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;

use alloc::{
    collections::btree_set::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use critical_section::Mutex;
use esp_hal::{
    peripherals::{BT, RADIO_CLK, WIFI},
    rmt::Channel,
    rng::Rng,
    time,
    timer::AnyTimer,
};
use esp_println::println;
use esp_wifi::{
    ble::controller::BleConnector,
    init,
    wifi::{new_with_mode, PromiscuousPkt, WifiApDevice},
    EspWifiInitFor,
};
use ieee80211::{match_frames, mgmt_frame::BeaconFrame};
use smart_leds::RGB8;

use crate::{leds, smartled::SmartLedsAdapter, storage};

static KNOWN_SSIDS: Mutex<RefCell<BTreeSet<String>>> = Mutex::new(RefCell::new(BTreeSet::new()));

#[embassy_executor::task]
pub async fn start_wifi(
    timer: AnyTimer,
    rng: Rng,
    radio_clock: RADIO_CLK,
    wifi: WIFI,
    mut led: SmartLedsAdapter<Channel<esp_hal::Blocking, 0>, 25>,
) -> ! {
    let init = init(EspWifiInitFor::Wifi, timer, rng, radio_clock).unwrap();
    println!("wifi initialized");

    leds::fade_in(&mut led, RGB8 { r: 0, g: 64, b: 0 }, 100);

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
                        let mut storage = storage::Store::new();
                        let byte_string = ssid.to_string();
                        println!("Found SSID: {:?}", byte_string);
                        let bytes = byte_string.as_bytes();
                        let _ = storage.append(bytes);
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

    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
pub async fn start_bluetooth(
    timer: AnyTimer,
    rng: Rng,
    radio_clock: RADIO_CLK,
    mut bluetooth: BT,
    mut led: SmartLedsAdapter<Channel<esp_hal::Blocking, 0>, 25>,
) -> ! {
    let init = init(EspWifiInitFor::Ble, timer, rng, radio_clock).unwrap();
    println!("ble initialized");

    leds::fade_in(&mut led, RGB8 { r: 0, g: 0, b: 255 }, 100);

    loop {
        let now = || time::now().duration_since_epoch().to_millis();
        let connector = BleConnector::new(&init, &mut bluetooth);
        let hci = HciConnector::new(connector, now);
        let mut ble = Ble::new(&hci);

        ble.init().unwrap();
        ble.cmd_set_le_advertising_parameters().unwrap();

        ble.cmd_set_le_advertising_data(
            create_advertising_data(&[
                AdStructure::CompleteLocalName("wifblink"),
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
            ])
            .unwrap(),
        )
        .unwrap();
        println!("{:?}", ble.cmd_set_le_advertise_enable(true));
        println!("started advertising");

        let mut store = storage::Store::new();
        let entries_vec = store
            .entries()
            .into_iter()
            .map(|bytes| {
                let string = String::from_utf8(bytes).unwrap();
                println!("entry: {:?}", string);
                return string;
            })
            .collect::<Vec<String>>();
        let entries_string = entries_vec.join("\n");

        println!("entries string: {:?}", entries_string);

        let entries = entries_string.as_bytes();

        let mut rf = |_offset: usize, data: &mut [u8]| {
            data[..entries.len()].copy_from_slice(entries);
            entries.len()
        };

        let mut wf = |offset: usize, data: &[u8]| {
            println!("RECEIVED: {} {:?}", offset, data);
        };

        gatt!([service {
            uuid: "e6a0ea50-6a66-013d-0514-061a78fcc099",
            characteristics: [
                // Count of entries
                characteristic {
                    uuid: "4194bb90-6a6c-013d-0514-061a78fcc099",
                    read: rf,
                    write: wf,
                },
            ],
        },]);

        let mut rng = bleps::no_rng::NoRng;
        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        loop {
            let mut notification = None;
            match srv.do_work_with_notification(notification) {
                Ok(res) => {
                    if let WorkResult::GotDisconnected = res {
                        break;
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}
