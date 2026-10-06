pub trait Rand: Send + Sync {
  fn get_u32(&self) -> u32;
}