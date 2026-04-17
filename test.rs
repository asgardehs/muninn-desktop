// Nótt & Dagr: compliance memory capture
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BatchRecord {
    batch_number: String,
    quantity_kg: f64,
    captured_at: u64,
}

impl BatchRecord {
    fn new(batch_number: String, quantity_kg: f64) -> Self {
        Self {
            batch_number,
            quantity_kg,
            captured_at: 1713312000,
        }
    }

    fn slug(&self) -> String {
        self.batch_number.to_lowercase()
    }
}

fn main() {
    let record = BatchRecord::new(
        "CHEM-2026-0417".to_string(),
        42.5,
    );
    println!("captured: {} ({} kg)", record.slug(), record.quantity_kg);
}