use alloc::vec::Vec;
use embedded_storage::{ReadStorage, Storage};
use esp_backtrace as _;
use esp_println::println;
use esp_storage::FlashStorage;

const FLASH_START: u32 = 0x9000;

pub struct Store {
    storage: FlashStorage,
    capacity: usize,
}

impl Store {
    pub fn new() -> Self {
        let storage = FlashStorage::new();
        let capacity = storage.capacity();
        Self { storage, capacity }
    }

    pub fn reset() -> Self {
        let storage = FlashStorage::new();
        let capacity = storage.capacity();
        let mut store = Self { storage, capacity };

        store.ireset();
        store
    }

    fn ireset(&mut self) {
        for i in (FLASH_START as usize..self.capacity).step_by(4096) {
            let bytes: [u8; 4096] = [0; 4096];
            self.storage.write(i.try_into().unwrap(), &bytes).unwrap();
            println!("Wrote block {}. {}", i, (i) / self.capacity);
        }
    }

    pub fn count(&mut self) -> u32 {
        let mut bytes: [u8; 4] = [0; 4];
        self.storage.read(FLASH_START, &mut bytes).unwrap();
        return u32::from_le_bytes(bytes);
    }

    pub fn entries(&mut self) -> Vec<Vec<u8>> {
        let mut cursor = FLASH_START + 4;
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
        let mut cursor = FLASH_START + 4;

        for _ in 0..self.count() {
            let mut bytes: [u8; 256] = [0 as u8; 256];
            let _ = self.storage.read(cursor, &mut bytes).unwrap();
            let len: usize = bytes[0] as usize;
            let result = &bytes[1..(len + 1)];

            if result == new_bytes {
                // We've already got it
                return None;
            }

            cursor += 256;
        }

        return Some(cursor);
    }

    pub fn append(&mut self, bytes: &[u8]) {
        if let Some(next_offset) = self.next_offset(bytes) {
            let old_count = self.count();

            // Write the length of the bytes as the first byte
            let len = bytes.len();
            self.storage
                .write(next_offset, [len as u8].as_slice())
                .unwrap();

            // Write the bytes after that
            self.storage.write(next_offset + 1, &bytes).unwrap();

            // Write the new count to the beginning of the storage
            let new_count_bytes = (old_count + 1).to_le_bytes();
            self.storage
                .write(FLASH_START, new_count_bytes.as_slice())
                .unwrap();
        }
    }
}
