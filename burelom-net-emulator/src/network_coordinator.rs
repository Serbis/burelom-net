use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use kiddo::{KdTree, SquaredEuclidean};
use log::info;
use prost::Message;
use burelom_net::{cobs, proto};
use crate::{test_node::TestNode, visualizer::{VisualizerState}};
use tap::Pipe;

pub struct NetworkCoorditor {
   node_spatial: Arc<Mutex<KdTree<f32, 2>>>,
   node_hash: Arc<Mutex<HashMap<u32, Arc<TestNode>>>>,
   visualizer_state: VisualizerState
}

impl NetworkCoorditor {
  pub fn new(visualizer_state: VisualizerState) -> Self {
    let node_spatial: KdTree<f32, 2> = KdTree::new();
    let node_hash: HashMap<u32, Arc<TestNode>> = HashMap::new();
     
    NetworkCoorditor {
      node_spatial: Arc::new(Mutex::new(node_spatial)),
      node_hash: Arc::new(Mutex::new(node_hash)),
      visualizer_state
    }
  }

  pub fn setup_nodes(&self, node_list: Vec<TestNode>) {
    for test_node in node_list {
      self.visualizer_state.add_node(&test_node);
      self.node_spatial
        .lock()
        .unwrap()
        .add(&test_node.position, test_node.addr as u64);

      self.node_hash
        .lock()
        .unwrap()
        .insert(test_node.addr, Arc::new(test_node));
    }

    for (_, (_, test_node)) in self.node_hash.lock().unwrap().iter_mut().enumerate() {
      test_node.node.run();
    }

    // Регулярно собираем данные о таблице маршрутизации каждой ноды
    let node_hash = self.node_hash.clone();
    let visualizer_state = self.visualizer_state.clone();
    tokio::spawn( async move {
      loop {
        for (_, (_, test_node)) in node_hash.lock().unwrap().iter().enumerate() {
          let rt = test_node.node.routing_table
            .routing_table
            .lock(|v| v.borrow().clone());
          let known_nodes = test_node.node.get_known_nodes();
          visualizer_state.update_routing(test_node.addr, &rt);
          visualizer_state.update_known_nodes(test_node.addr, &known_nodes);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
      }
      
    });
  }

  // Узел отправил данные в сеть
  pub fn send_from_mac(&self, from: u32, gw: u32, data: &Vec<u8>) {
    // Получаем ноду от которой идет отправка
    let from_test_node = self.node_hash
        .lock()
        .unwrap()
        .get(&from)
        .unwrap()
        .clone();

    // Находим все ноды в зоне видимости отправителя
    let neighbour_node_list = self.node_spatial
      .lock()
      .unwrap()
      .within::<SquaredEuclidean>(
          &from_test_node.position,
          (from_test_node.power as f32).powi(2)
      )
      .iter()
      .map(|v| v.item as u32)
      .filter(|v| *v != from)
      .collect::<Vec<u32>>()
      .pipe(|v| {
        let node_hash = self.node_hash.lock().unwrap();
        v.iter()
          .map(|a| {
            node_hash.get(a).unwrap().clone()
          })
          .collect::<Vec<Arc<TestNode>>>()
      });


    // Разбираем пакет для анализа (он всегда приходит целым)
    let packet_bin = cobs::decode(data).unwrap();
    let packet = proto::Packet::decode(&packet_bin[..]).unwrap();

    // Если это boradcast
    if gw == 0 {
      for node in neighbour_node_list {
        // Положить в очередь тестовой ноды данные
        node.data_tx.send((from, data.clone())).unwrap();

        // Создаем визуализацию передачи пакета
        self.visualizer_state.add_transction(&from_test_node, &node, &packet);
      }
    } else {
      let gateway_node = neighbour_node_list.iter()
        .find(|v| v.addr == gw);
      if let Some(node) = gateway_node {
        node.data_tx.send((from, data.clone())).unwrap();

        // Создаем визуализацию передачи пакета
        self.visualizer_state.add_transction(&from_test_node, &node, &packet);
      }
    }
  }

  // mac узла подписывается на входящие данные из сети
  pub async fn recv_from_mac(&self, addr: u32) -> (u32, Vec<u8>) {
    // Получаем тестовую ноду в которую будут приходить данные
    let test_node = self.node_hash
        .lock()
        .unwrap()
        .get(&addr)
        .unwrap()
        .clone();

    // Экслюзивно подписываемся на канал предели данных в ноду
    let (gw, data) = test_node.data_rx
      .clone()
      .lock()
      .await
      .recv()
      .await
      .unwrap();

    (gw, data)
  }

  pub async fn send_broadcast(&self, from: u32, data: Vec<u8>) {
    info!("Send boradcast data from={}, data={}", from, const_hex::encode_upper(&data));

    let test_node = self.node_hash
      .lock()
      .unwrap()
      .get(&from)
      .unwrap()
      .clone();

    test_node.node.send_broadcast(&data).await;
  }

  pub async fn send(&self, from: u32, to: u32, data: Vec<u8>) {
    info!("Send data from={}, to={}, data={}", from, to, const_hex::encode_upper(&data));

    let test_node = self.node_hash
      .lock()
      .unwrap()
      .get(&from)
      .unwrap()
      .clone();

    test_node.node.send(to, &data).await.unwrap();
  }

  pub async fn recv(&self, addr: u32) -> (u32, Vec<u8>) {
    let test_node = self.node_hash
      .lock()
      .unwrap()
      .get(&addr)
      .unwrap()
      .clone();

    test_node.node.recv().await.unwrap()
  }

  pub async fn route_request(&self, source: u32, dest: u32) -> Option<u32> {
    info!("Send route request source={}, dest={}", source, dest);

    let test_node = self.node_hash
      .lock()
      .unwrap()
      .get(&source)
      .unwrap()
      .clone();

    test_node.node.route_request(dest).await
  }
}