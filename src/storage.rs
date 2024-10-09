use core::str;

use embedded_storage::{ReadStorage, Storage};
use esp_backtrace as _;
use esp_println::println;
use esp_storage::FlashStorage;

// Where does the flash memory start
const FLASH_START: u32 = 0x9000;

pub struct Store {
    storage: FlashStorage,
    cursor: u32,
    pub capacity: usize,
}

impl Store {
    pub fn new() -> Self {
        let storage = FlashStorage::new();
        let capacity = storage.capacity();
        let mut store = Self {
            storage,
            cursor: 0,
            capacity,
        };

        store.cursor = store.count() * 256;
        store
    }

    pub fn initialize(&mut self) {
        for i in ((FLASH_START as usize)..self.capacity).step_by(65_536) {
            let bytes: [u8; 65_536] = [0; 65_536];
            self.storage.write(i.try_into().unwrap(), &bytes).unwrap();
            println!("Wrote block {}. {}", i, (i) / self.capacity);
        }
    }

    pub fn count(&mut self) -> u32 {
        let mut bytes: [u8; 4] = [0; 4];
        self.storage.read(FLASH_START, &mut bytes).unwrap();
        return u32::from_be_bytes(bytes);
    }

    pub fn entries(&mut self) {
        let mut cursor = FLASH_START + 1;
        for _ in 0..self.count() {
            let mut bytes: [u8; 256] = [0 as u8; 256];
            let _ = self.storage.read(cursor, &mut bytes).unwrap();
            let len = bytes[0] as usize;
            let string = str::from_utf8(&bytes[0..(len + 1)]).unwrap();
            println!("Entry: {}", string);
            cursor += 256;
        }
    }

    pub fn append(&mut self, bytes: &[u8]) {
        let old_count = self.count();

        println!(
            "Flash writing bytes: {:?}, current count: {:?}",
            bytes, old_count
        );

        let offset = FLASH_START + (256 * old_count);

        // Write the length of the bytes as the first byte
        self.storage
            .write(offset, [bytes.len() as u8].as_slice())
            .unwrap();

        // Write the bytes after that
        self.storage.write(offset + 1, bytes).unwrap();

        // Write the new count to the beginning of the storage
        self.storage
            .write(FLASH_START, (old_count + 1).to_be_bytes().as_slice())
            .unwrap();
    }
}
