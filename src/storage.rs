use core::str;

use alloc::vec::Vec;
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

    pub fn reset(&mut self) {
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

    pub fn entries(&mut self) -> Vec<Vec<u8>> {
        let mut cursor = FLASH_START + 1;
        let mut results: Vec<Vec<u8>> = Vec::new();
        for _ in 0..self.count() {
            let mut bytes: [u8; 256] = [0 as u8; 256];
            let _ = self.storage.read(cursor, &mut bytes).unwrap();
            let len = bytes[0] as usize;
            let real_bytes = bytes[1..(len + 1)].to_vec();
            results.insert(0, real_bytes);
            cursor += 256;
        }

        return results;
    }

    pub fn next_offset(&mut self, new_bytes: &[u8]) -> Option<u32> {
        let mut cursor = FLASH_START + 1;

        for _ in 0..self.count() {
            let mut bytes: [u8; 256] = [0 as u8; 256];
            let _ = self.storage.read(cursor, &mut bytes).unwrap();
            let len: usize = bytes[0] as usize;
            let result = &bytes[1..(len + 1)];

            if result == new_bytes {
                // We've already got it
                let string = str::from_utf8(&new_bytes).unwrap();
                println!("Already know about {}, skipping", string);
                return None;
            } else {
                println!("New Result: {:?}", str::from_utf8(result));
                println!("New bytes: {:?}", str::from_utf8(new_bytes));
            }

            cursor += 256;
        }

        return Some(cursor);
    }

    pub fn append(&mut self, bytes: &[u8]) {
        if let Some(next_offset) = self.next_offset(bytes) {
            println!("Next offset is now {:?}", next_offset);

            let old_count = self.count();

            println!(
                "Flash writing bytes: {:?}, current count: {:?}",
                bytes,
                old_count + 1
            );

            // Write the length of the bytes as the first byte
            let len = bytes.len();
            self.storage
                .write(next_offset, [len as u8].as_slice())
                .unwrap();

            // Write the bytes after that
            self.storage.write(next_offset + 1, &bytes).unwrap();

            // Write the new count to the beginning of the storage
            self.storage
                .write(FLASH_START, (old_count + 1).to_be_bytes().as_slice())
                .unwrap();

            println!("Wrote {:?}", str::from_utf8(bytes));
        }
    }
}
