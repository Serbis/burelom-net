use burelom_net::traits::rand::Rand;
use esp_hal::rng::Rng;

pub struct Esp32Rand {
    rng: Rng
}

impl Esp32Rand {
    pub fn new() -> Self {
        Esp32Rand {
            rng: Rng::new()
        }
    }
}

impl Rand for Esp32Rand {
    fn get_u32(&self) -> u32 {
        self.rng.random()
    }
}