use packman::VecPack;
use tokio::sync::Mutex;

mod commitment;

struct CommitmentService {
  commitments: Mutex<VecPack<commitment::Customer>>,
}

impl CommitmentService {
  fn init(commitments: VecPack<commitment::Customer>) -> Self {
    Self {
      commitments: Mutex::new(commitments),
    }
  }
}

fn main() {}
