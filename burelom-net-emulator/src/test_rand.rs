use burelom_net::traits::rand::Rand;
use rand::Rng;

pub struct TestRand {}

impl TestRand {
  pub fn new() -> Self {
    TestRand {}
  }
}

impl Rand for TestRand {
  fn get_u32(&self) -> u32 {
    rand::rng().next_u32()
  }
}