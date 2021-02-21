use std::collections::HashMap;

mod commitment;

type CustomerId = u32;

struct Commitments {
  customer_commitments: HashMap<CustomerId, Vec<commitment::Commitment>>,
}

fn main() {}
