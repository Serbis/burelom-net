use core::cell::RefCell;

use alloc::sync::Arc;
use alloc::vec::{Vec};
use alloc::boxed::Box;
use async_trait::async_trait;
use burelom_net::traits::mac::Mac;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig};
use esp_radio::esp_now::{BROADCAST_ADDRESS, EspNow, EspNowManager, EspNowReceiver, EspNowSender, PeerInfo};

type LedBlinkChannel = Channel<CriticalSectionRawMutex, u8, 8>;

pub struct EspNowMac {
  sender: Arc<Mutex<CriticalSectionRawMutex, RefCell<EspNowSender>>>,
  receiver: Arc<Mutex<CriticalSectionRawMutex, RefCell<EspNowReceiver>>>,
  manager: Arc<Mutex<CriticalSectionRawMutex, RefCell<EspNowManager>>>,
  led_blink_tx: Sender<'static, CriticalSectionRawMutex, u8, 8>,
}

impl  EspNowMac  {
  pub fn new(esp_now: EspNow, led_pin: AnyPin<'static>, spawner: &embassy_executor::Spawner) -> Self {
    let (manager, sender, receiver) = esp_now.split();
    let sender = Arc::new(Mutex::new(RefCell::new(sender)));

    let channel: &'static LedBlinkChannel = Box::leak(Box::new(LedBlinkChannel::new()));

    let tx = channel.sender();
    let rx = channel.receiver();

    spawner.spawn(packet_recv_blink(led_pin, rx).unwrap());

    EspNowMac { 
      sender,
      receiver: Arc::new(Mutex::new(RefCell::new(receiver))),
      manager: Arc::new(Mutex::new(RefCell::new(manager))),
      led_blink_tx: tx
    }
  }
}

#[async_trait(?Send)]
impl Mac for EspNowMac {
  async fn send(&self, dest: u32, data: &Vec<u8>) {
    let addr: [u8; 6] = if dest == 0 {
      BROADCAST_ADDRESS
    } else {
      [0x02_u8, 0x00, 0x00, 0x00, 0x00, dest as u8]
    };

    let _ = self.sender
      .lock()
      .await
      .borrow_mut()
      .send_async(&addr, data)
      .await;
  }
  
  async fn recv(&self) -> (u32, i32, Vec<u8>) {
    let received = self.receiver
      .lock()
      .await
      .borrow_mut()
      .receive_async()
      .await;
    
    self.led_blink_tx.send(1).await;

    let manager_guard = self.manager.lock().await;
    let manager = manager_guard.borrow();

    if !manager.peer_exists(&received.info.src_address) {
      manager
        .add_peer(PeerInfo {
          interface: esp_radio::esp_now::EspNowWifiInterface::Station,
          peer_address: received.info.src_address,
          lmk: None,
          channel: None,
          encrypt: false,
        })
        .unwrap();
    }

    let addr = received.info.src_address[5] as u32;
    let rssi = received.info.rx_control.rssi;
    let data = received.data().to_vec();
    
    (addr, rssi, data)
  }
}

#[embassy_executor::task]
async fn packet_recv_blink(pin: AnyPin<'static>, rx: Receiver<'static, CriticalSectionRawMutex, u8, 8>) {
  let mut led = Output::new(pin, Level::Low, OutputConfig::default());

  loop {
    rx.receive().await;
    led.set_high();
    Timer::after(Duration::from_millis(100)).await;
    led.set_low();
  }  
}
