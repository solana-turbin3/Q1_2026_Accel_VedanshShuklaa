use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use wincode::{SchemaRead, SchemaWrite};
use std::hint::black_box;
use std::time::Instant;

mod generic_storage;

use generic_storage::*;

// Derive SchemaRead and SchemaWrite separately (not a single "Wincode" derive)
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, SchemaRead, SchemaWrite)]
struct LargeUser {
    name: String,
    age: u8,
    large_data: Vec<u8>,
}

fn main() {
    const ITERATIONS: usize = 100000;

    let user = LargeUser {
        name: "Andre".to_string(),
        age: 30,
        large_data: vec![0; 1024],
    };

    println!("Running {} iterations...\n", ITERATIONS);

    benchmark_serializer("Borsh Save", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(Borsh);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("Wincode Save", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(WincodeSerializer);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("JSON Save", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(Json);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("Borsh Roundtrip", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(Borsh);
        storage.save(black_box(&user)).unwrap();
        let _: LargeUser = black_box(storage.load().unwrap());
    });

    benchmark_serializer("Wincode Roundtrip", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(WincodeSerializer);
        storage.save(black_box(&user)).unwrap();
        let _: LargeUser = black_box(storage.load().unwrap());
    });

    benchmark_serializer("JSON Roundtrip", ITERATIONS, || {
        let mut storage: Storage<LargeUser, _> = Storage::new(Json);
        storage.save(black_box(&user)).unwrap();
        let _: LargeUser = black_box(storage.load().unwrap());
    });
}

fn benchmark_serializer<F>(name: &str, iterations: usize, mut f: F)
where
    F: FnMut(),
{
    let start = Instant::now();

    for _ in 0..iterations {
        f();
    }

    let duration = start.elapsed();
    let per_op = duration.as_secs_f64() / iterations as f64 * 1_000_000.0;

    println!(
        "{:<20} | total: {:?} | per op: {:.4} µs",
        name, duration, per_op
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, SchemaRead, SchemaWrite, Debug, PartialEq)]
    struct User {
        name: String,
        age: u8,
    }

    #[test]
    fn test_storage_borsh() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };

        let mut storage: Storage<User, _> = Storage::new(Borsh);
        storage.save(&user).unwrap();
        let loaded: User = storage.load().unwrap();
        
        assert_eq!(loaded, user);
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_wincode() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };

        let mut storage: Storage<User, _> = Storage::new(WincodeSerializer);
        storage.save(&user).unwrap();
        let loaded: User = storage.load().unwrap();
        
        assert_eq!(loaded, user);
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_json() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };

        let mut storage: Storage<User, _> = Storage::new(Json);
        storage.save(&user).unwrap();
        let loaded: User = storage.load().unwrap();
        
        assert_eq!(loaded, user);
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_convert_to_other_format() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };

        let mut storage: Storage<User, _> = Storage::new(Borsh);
        storage.save(&user).unwrap();

        let new_storage = storage.convert_to_other_format(Json);

        assert!(new_storage.has_data());
    }
}