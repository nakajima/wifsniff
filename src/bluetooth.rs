use alloc::{string::String, vec::Vec};
use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::WorkResult,
    gatt, HciConnector,
};
use embassy_time::{Duration, Timer};
use esp_hal::{
    peripherals::{BT, RADIO_CLK},
    rng::Rng,
    time,
    timer::AnyTimer,
};
use esp_println::println;
use esp_wifi::{ble::controller::asynch::BleConnector, init, EspWifiInitFor};

use crate::storage;

#[embassy_executor::task]
pub async fn start_bluetooth(
    timer: AnyTimer,
    rng: Rng,
    radio_clock: RADIO_CLK,
    mut bluetooth: BT,
) -> ! {
    // let init = init(EspWifiInitFor::Ble, timer, rng, radio_clock).unwrap();
    println!("ble initialized");

    loop {
        // let connector = BleConnector::new(&init, &mut bluetooth);

        // let now = || time::now().duration_since_epoch().to_millis();
        // let mut ble = Ble::new(connector, now);

        // ble.init().await;
        // ble.cmd_set_le_advertising_parameters().await;

        // ble.cmd_set_le_advertising_data(
        //     create_advertising_data(&[
        //         AdStructure::CompleteLocalName("wifblink"),
        //         AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        //         AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
        //     ])
        //     .unwrap(),
        // )
        // .await;
        // // println!("{:?}", ble.cmd_set_le_advertise_enable(true).await);
        // println!("started advertising");

        // let mut store = storage::Store::new();
        // let entries_vec = store
        //     .entries()
        //     .into_iter()
        //     .map(|bytes| {
        //         let string = String::from_utf8(bytes).unwrap();
        //         println!("entry: {:?}", string);
        //         return string;
        //     })
        //     .collect::<Vec<String>>();
        // let entries_string = entries_vec.join("\n");

        // println!("entries string: {:?}", entries_string);

        // let entries = entries_string.as_bytes();

        // let mut rf = |_offset: usize, data: &mut [u8]| {
        //     data[..entries.len()].copy_from_slice(entries);
        //     entries.len()
        // };

        // let mut wf = |offset: usize, data: &[u8]| {
        //     println!("RECEIVED: {} {:?}", offset, data);
        // };

        // gatt!([service {
        //     uuid: "e6a0ea50-6a66-013d-0514-061a78fcc099",
        //     characteristics: [
        //         // Count of entries
        //         characteristic {
        //             uuid: "4194bb90-6a6c-013d-0514-061a78fcc099",
        //             read: rf,
        //             write: wf,
        //         },
        //     ],
        // },]);

        // let mut rng = bleps::no_rng::NoRng;
        // let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        // let mut notification = None;
        // match srv.do_work_with_notification(notification).await {
        //     Ok(res) => {
        //         if let WorkResult::GotDisconnected = res {
        //             break;
        //         }
        //     }
        //     Err(err) => {
        //         println!("{:?}", err);
        //     }
        // }
        Timer::after(Duration::from_secs(10)).await;
    }
}
