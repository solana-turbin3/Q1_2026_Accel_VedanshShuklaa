use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Serialize, Deserialize};
use std::time::Instant;
use std::hint::black_box;

mod generic_storage;

use generic_storage::*;

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
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

    //Benchmarks

    println!("Running {} iterations...\n", ITERATIONS);

    benchmark_serializer("Borsh Save", ITERATIONS, || {
        let mut storage = Storage::new(Borsh);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("Bincode Save", ITERATIONS, || {
        let mut storage = Storage::new(Bincode);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("JSON Save", ITERATIONS, || {
        let mut storage = Storage::new(Json);
        storage.save(black_box(&user)).unwrap();
    });

    benchmark_serializer("Borsh Roundtrip", ITERATIONS, || {
        let mut storage = Storage::new(Borsh);
        storage.save(black_box(&user)).unwrap();
        let _: LargeUser = black_box(storage.load().unwrap());
    });

    benchmark_serializer("Bincode Roundtrip", ITERATIONS, || {
        let mut storage = Storage::new(Bincode);
        storage.save(black_box(&user)).unwrap();
        let _: LargeUser = black_box(storage.load().unwrap());
    });

    benchmark_serializer("JSON Roundtrip", ITERATIONS, || {
        let mut storage = Storage::new(Json);
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

    println!("{:<20} | total: {:?} | per op: {:.4} µs", name, duration, per_op);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
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
    
        let mut storage = Storage::new(Borsh);
        match storage.save(&user) {
            Ok(_) => println!("User saved successfully"),
            Err(e) => println!("Failed to save user: {}", e.message),
        }
        
        match storage.load() {
            Ok(user) => println!("User loaded successfully: {}", user.name),
            Err(e) => println!("Failed to load user: {}", e.message),
        }
        
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_bincode() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };
    
        let mut storage = Storage::new(Bincode);
        match storage.save(&user) {
            Ok(_) => println!("User saved successfully"),
            Err(e) => println!("Failed to save user: {}", e.message),
        }
        
        match storage.load() {
            Ok(user) => println!("User loaded successfully: {}", user.name),
            Err(e) => println!("Failed to load user: {}", e.message),
        }
        
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_json() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };
    
        let mut storage = Storage::new(Json);
        match storage.save(&user) {
            Ok(_) => println!("User saved successfully"),
            Err(e) => println!("Failed to save user: {}", e.message),
        }
        
        match storage.load() {
            Ok(user) => println!("User loaded successfully: {}", user.name),
            Err(e) => println!("Failed to load user: {}", e.message),
        }
        
        assert!(storage.has_data());
    }

    #[test]
    fn test_storage_convert_to_other_format() {
        let user = User {
            name: "Andre".to_string(),
            age: 30,
        };
    
        let mut storage = Storage::new(Borsh);
        match storage.save(&user) {
            Ok(_) => println!("User saved successfully"),
            Err(e) => println!("Failed to save user: {}", e.message),
        }
        
        let new_storage = storage.convert_to_other_format(Json);
        
        match new_storage.load() {
            Ok(user) => println!("User loaded successfully: {}", user.name),
            Err(e) => println!("Failed to load user: {}", e.message),
        }
        
        assert!(new_storage.has_data());
    }
}