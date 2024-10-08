use embedded_storage::{ReadStorage, Storage};
use esp_backtrace as _;
use esp_hal::prelude::*;
use esp_println::println;
use esp_storage::FlashStorage;

// Where does the flash memory start
const FLASH_START: i32 = 0x9000;

pub fn store() {
    let mut bytes = [0u8; 32];
    let mut flash = FlashStorage::new();
    println!("Flash size = {}", flash.capacity());
    println!();
}
