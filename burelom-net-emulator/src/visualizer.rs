/// ATTINTION: All this code fully manage by AI. Be careful when
/// copying and using this code in production solutions!

use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use eframe::egui;
use enumset::EnumSet;
use burelom_net::node_registry::IdentyType;
use burelom_net::node_registry::NodeRegistryRow;
use burelom_net::proto;
use burelom_net::roles::device_roles::DeviceRole;
use burelom_net::routing_table::RoutinRow;
use tokio::time::Instant;
use crate::test_node::TestNode;


#[derive(Clone, Debug)]
struct RenderNode {
    addr: u32,
    power: u32,
    routing: Vec<RenderRouting>,         
    known_nodes: Vec<RenderKnownNode>,
    position: [f32; 2],  
}

#[derive(Clone, Debug)]
struct RenderRouting {
    target: u32,
    gateway: u32,
    hops: u32
}

#[derive(Clone, Debug)]
struct RenderKnownNode {
    addr: u32,
    name: Option<String>,
    hops: u32,
    rssi: Option<i32>,
    last_seen: Instant,
    neighbour_nodes: Option<Vec<RenderNeighbourNode>>,
    identy_type: EnumSet<IdentyType>,
    roles: Option<EnumSet<DeviceRole>>
}

#[derive(Clone, Debug)]
struct RenderNeighbourNode {
    addr: u32,
    rssi: i32
}

#[derive(Clone, Debug)]
struct RenderTransaction {
    time: u128,
    from_position: [f32; 2],
    to_position: [f32; 2],
    from_addr: u32, 
    to_addr: u32,
    packet: proto::Packet
}

#[derive(Clone)]
pub struct VisualizerState {
    nodes: Arc<Mutex<Vec<RenderNode>>>,
    transactions: Arc<Mutex<Vec<RenderTransaction>>>,
    egui_context: Option<egui::Context>
}

impl VisualizerState {
    pub fn new() -> Self {
        VisualizerState {
            nodes: Arc::new(Mutex::new(vec![])),
            transactions: Arc::new(Mutex::new(vec![])),
            egui_context: None
        }
    }
    
    pub fn add_node(&self, node: &TestNode) {
        if let Ok(mut nodes) = self.nodes.lock() {
            nodes.push(
                RenderNode {
                    addr: node.addr,
                    power: node.power,
                    position: node.position,
                    routing: vec![],
                    known_nodes: vec![]
                }
            );
        } 
        if let Some(ctx) = self.egui_context.as_ref() {
            ctx.request_repaint();
        }
    }

    pub fn remove_node(&self, addr: u32) {
        if let Ok(mut nodes) = self.nodes.lock() {
            nodes.retain(|v| v.addr != addr);
        } 
        if let Ok(mut transactions) = self.transactions.lock() {
            transactions.retain(|v| v.to_addr != addr && v.from_addr != addr);
        } 
        if let Some(ctx) = self.egui_context.as_ref() {
            ctx.request_repaint();
        }
    }
    
    pub fn add_transction(&self, from: &TestNode, to: &TestNode, packet: &proto::Packet) {
        if let Ok(mut rt) = self.transactions.lock() {
            let tr = RenderTransaction {
                time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
                from_position: from.position,
                to_position: to.position,
                from_addr: from.addr,
                to_addr: to.addr,
                packet: packet.clone()
            };
            //info!("{:?}", &tr);
            rt.push(
                tr
            );
        } 
        
        if let Some(ctx) = self.egui_context.as_ref() {
            ctx.request_repaint();
        }
    }
    
    pub fn update_routing(&self, addr: u32, routing_table: &Vec<RoutinRow>) {
        if let Ok(mut nodes) = self.nodes.lock() {
            if let Some(node) = nodes.iter_mut().find(|v| v.addr == addr) {
                node.routing = routing_table
                    .iter()
                    .map(|v| {
                        RenderRouting { target: v.target, gateway: v.gateway, hops: v.hops }
                    })
                    .collect();
            }
        }
    }

    pub fn update_known_nodes(&self, addr: u32, known_nodes: &Vec<NodeRegistryRow>) {
        if let Ok(mut nodes) = self.nodes.lock() {
            if let Some(node) = nodes.iter_mut().find(|v| v.addr == addr) {
                node.known_nodes = known_nodes
                    .iter()
                    .map(|v| {
                        let rnn: Vec<RenderNeighbourNode> = v.neighbour_nodes
                            .as_ref()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|v| {
                                RenderNeighbourNode {
                                    addr: v.addr,
                                    rssi: v.rssi
                                }
                            })
                            .collect();

                        RenderKnownNode {
                            addr: v.addr,
                            name: v.name.as_ref().cloned(),
                            hops: v.hops,
                            rssi: v.rssi,
                            last_seen: v.last_seen,
                            neighbour_nodes: Some(rnn),
                            identy_type: v.identy_type,
                            roles: v.roles
                        }
                    })
                    .collect();
            }
        }
    }
}

pub struct Visualizer {
    pan: egui::Vec2,      // смещение канваса
    zoom: f32,
    selected_tx: Option<(u32, u32, u32, u32)>,
    selected_node: Option<usize>,
    state: VisualizerState,
    node_selected_at: Option<Instant>
}

impl Visualizer {
    pub fn new(state: VisualizerState) -> Self {
        Visualizer {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            state,
            selected_tx: None,
            selected_node: None,
            node_selected_at: None
        }
    }
    
    pub fn run(mut self) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Burelom Visualizer"),
            ..Default::default()
        };
        
        eframe::run_native(
            "Burelom Visualizer",
            options,
            Box::new(|cc| {
                self.state.egui_context = Some(cc.egui_ctx.clone());
                Ok(Box::new(self))
            }),
        ).unwrap();
    }
}

/// Рисует пунктирный круг через серию коротких дуг.
fn draw_dashed_circle(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    color: egui::Color32,
    width: f32,
    dash_count: usize,
) {
    if radius < 1.0 {
        return;
    }
    let step = std::f32::consts::TAU / dash_count as f32;
    let dash_len = step * 0.55; // 55% окружности — «штрих», остальное — «пробел»
    
    for i in 0..dash_count {
        let a0 = i as f32 * step;
        let a1 = a0 + dash_len;
        // Разбиваем дугу на подотрезки, чтобы она выглядела плавно
        let sub = 4;
        let mut prev = center + egui::vec2(a0.cos(), a0.sin()) * radius;
        for s in 1..=sub {
            let t = a0 + (a1 - a0) * (s as f32 / sub as f32);
            let p = center + egui::vec2(t.cos(), t.sin()) * radius;
            painter.line_segment([prev, p], egui::Stroke::new(width, color));
            prev = p;
        }
    }
}

fn dist_point_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn pad_cell(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if width == 0 {
        return String::new();
    }
    if count <= width {
        format!("{:<width$}", s, width = width)
    } else {
        let mut out: String = s.chars().take(width - 1).collect();
        out.push('…');
        out
    }
}

fn fmt_enumset<T>(set: &enumset::EnumSet<T>) -> String
where
    T: enumset::EnumSetType + std::fmt::Debug,
{
    set.iter()
        .map(|v| format!("{:?}", v))
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_known_nodes(
    ego_addr: u32,
    known: &[RenderKnownNode],
    area: egui::Rect,
) -> std::collections::HashMap<u32, egui::Pos2> {
    use std::collections::HashMap;

    let mut result = HashMap::new();
    let n = known.len();

    if n == 0 {
        result.insert(ego_addr, area.center());
        return result;
    }

    let total = n + 1;
    let ego_idx = n;

    let mut idx_of: HashMap<u32, usize> = HashMap::with_capacity(n);
    for (i, kn) in known.iter().enumerate() {
        idx_of.insert(kn.addr, i);
    }

    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let mut pos: Vec<egui::Vec2> = Vec::with_capacity(total);
    for i in 0..total {
        let r = ((i as f32 + 0.5) / total as f32).sqrt() * 0.5;
        let a = i as f32 * golden;
        pos.push(egui::vec2(r * a.cos(), r * a.sin()));
    }

    let mut vel = vec![egui::Vec2::ZERO; total];
    let repulsion: f32  = 0.0012;
    let attraction: f32 = 0.08;
    let damping: f32    = 0.82;

    for _ in 0..120 {
        for i in 0..total {
            for j in (i + 1)..total {
                let d = pos[i] - pos[j];
                let d2 = d.length_sq().max(1e-4);
                let dist = d2.sqrt();
                let f = d / dist * (repulsion / d2);
                vel[i] += f;
                vel[j] -= f;
            }
        }

        for (i, kn) in known.iter().enumerate() {
            if let Some(neighs) = &kn.neighbour_nodes {
                for nb in neighs {
                    if let Some(&j) = idx_of.get(&nb.addr) {
                        if i != j {
                            let d = pos[j] - pos[i];
                            let f = d * attraction;
                            vel[i] += f;
                            vel[j] -= f;
                        }
                    }
                }
            }
        }

        for (i, kn) in known.iter().enumerate() {
            if kn.hops == 1 {
                let d = pos[i] - pos[ego_idx];
                let f = d * attraction * 1.5;
                vel[i]      -= f;
                vel[ego_idx] += f;
            }
        }

        for i in 0..total {
            pos[i] += vel[i];
            vel[i] *= damping;
        }

        let mut c = egui::Vec2::ZERO;
        for p in &pos { c += *p; }
        c /= total as f32;
        for p in &mut pos { *p -= c; }
    }

    let mut max_x: f32 = 1e-4;
    let mut max_y: f32 = 1e-4;
    for p in &pos {
        max_x = max_x.max(p.x.abs());
        max_y = max_y.max(p.y.abs());
    }
    let half_w = area.width()  * 0.5;
    let half_h = area.height() * 0.5;
    let scale = (half_w / max_x).min(half_h / max_y);

    let c = area.center();
    for (i, kn) in known.iter().enumerate() {
        result.insert(
            kn.addr,
            egui::pos2(c.x + pos[i].x * scale, c.y + pos[i].y * scale),
        );
    }
    result.insert(
        ego_addr,
        egui::pos2(c.x + pos[ego_idx].x * scale, c.y + pos[ego_idx].y * scale),
    );

    // ===== Жёсткое ограничение минимального расстояния (в пикселях) =====
    // Достаточно, чтобы точка (r=3) и подпись "12345" (≈30 px) визуально не сливались.
    const MIN_DIST: f32 = 26.0;
    const ITERS: usize  = 40;

    let mut pts: Vec<(u32, egui::Pos2)> = result.iter().map(|(k, v)| (*k, *v)).collect();
    pts.sort_by_key(|(k, _)| *k);
    
    let min_x = area.left()   + 2.0;
    let max_x = area.right()  - 2.0;
    let min_y = area.top()    + 2.0;
    let max_y = area.bottom() - 2.0;

    for _ in 0..ITERS {
        let mut moved = false;
        for i in 0..pts.len() {
            for j in (i + 1)..pts.len() {
                let a = pts[i].1;
                let b = pts[j].1;
                let d = b - a;
                let dist = d.length();
                if dist < MIN_DIST {
                    // Если точки почти идеально совпали — расталкиваем
                    // в детерминированном направлении, чтобы не делить на ноль.
                    let dir = if dist > 0.01 {
                        d / dist
                    } else {
                        let ang = (i as f32 * 2.399) + (j as f32 * 0.7);
                        egui::vec2(ang.cos(), ang.sin())
                    };
                    let push = dir * ((MIN_DIST - dist) * 0.5 + 0.5);

                    let mut na = a - push;
                    let mut nb = b + push;
                    na.x = na.x.clamp(min_x, max_x);
                    na.y = na.y.clamp(min_y, max_y);
                    nb.x = nb.x.clamp(min_x, max_x);
                    nb.y = nb.y.clamp(min_y, max_y);
                    pts[i].1 = na;
                    pts[j].1 = nb;
                    moved = true;
                }
            }
        }
        if !moved { break; }
    }

    result.clear();
    for (k, v) in pts {
        result.insert(k, v);
    }

    result
}

impl eframe::App for Visualizer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
        let rect = response.rect;

        // --- Панорамирование перетаскиванием ---
        if response.dragged() {
            self.pan += response.drag_delta();
        }

        // --- Зум колёсиком с якорем на курсоре ---
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.0 && response.hovered() {
            if let Some(cursor) = response.hover_pos() {
                let old_zoom = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.1, 5.0);
                let rel = cursor - rect.center() - self.pan;
                self.pan -= rel * (self.zoom / old_zoom - 1.0);
            }
        }

        // Перевод мировых координат в экранные
        let to_screen = |p: [f32; 2]| -> egui::Pos2 {
            rect.center()
                + egui::vec2(p[0] * self.zoom, p[1] * self.zoom)
                + self.pan
        };

        let click_pos = if response.clicked() {
            response.interact_pointer_pos()
        } else {
            None
        };

        let mut clicked_node: Option<usize> = None;
        // Ключ направления: (from.x_bits, from.y_bits, to.x_bits, to.y_bits)
        let mut clicked_dir_key: Option<(u32, u32, u32, u32)> = None;
        let mut best_arrow_dist: Option<f32> = None;

        // --- Отрисовка нод + детект клика + подсветка ---
        let hover_pos = response.hover_pos();

        if let Ok(nodes) = self.state.nodes.lock() {
            for (idx, node) in nodes.iter().enumerate() {
                let center = to_screen(node.position);
                let power_r = node.power as f32 * self.zoom;

                // Хитбокс наведения совпадает с хитбоксом клика (радиус 8 px)
                let is_hovered = hover_pos
                    .map_or(false, |hp| (hp - center).length() < 8.0);
                let is_selected = self.selected_node == Some(idx);

                // --- Контур зоны действия (power) ---
                let (power_color, power_width) = if is_selected {
                    // ярко-голубой, потолще
                    (egui::Color32::from_rgb(200, 225, 255), 2.0)
                } else if is_hovered {
                    // светлее обычного
                    (egui::Color32::from_rgb(150, 180, 255), 1.6)
                } else {
                    (egui::Color32::from_rgb(90, 110, 200), 1.2)
                };

                draw_dashed_circle(&painter, center, power_r, power_color, power_width, 24);

                // --- Линия-выноска к подписи ---
                let label_anchor = center + egui::vec2(12.0, -12.0);
                let line_color = if is_selected {
                    egui::Color32::from_rgb(255, 225, 100)
                } else if is_hovered {
                    egui::Color32::from_gray(200)
                } else {
                    egui::Color32::from_gray(120)
                };
                painter.line_segment(
                    [center, label_anchor],
                    egui::Stroke::new(1.0, line_color),
                );

                // --- Сам маркер ноды ---
                let (marker_fill, marker_stroke, marker_stroke_w) = if is_selected {
                    // Ярко-золотой маркер с белой обводкой — хорошо видно как «выбрано»
                    (
                        egui::Color32::from_rgb(255, 225, 100),
                        egui::Color32::from_rgb(255, 255, 255),
                        2.0,
                    )
                } else if is_hovered {
                    // Подсвеченный красный
                    (
                        egui::Color32::from_rgb(255, 150, 150),
                        egui::Color32::from_rgb(120, 60, 60),
                        1.5,
                    )
                } else {
                    (
                        egui::Color32::from_rgb(230, 80, 80),
                        egui::Color32::from_gray(40),
                        1.0,
                    )
                };

                // Внешнее «гало» для выбранной ноды
                if is_selected {
                    painter.circle_stroke(
                        center,
                        9.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 225, 100)),
                    );
                }

                painter.circle_filled(center, 4.0, marker_fill);
                painter.circle_stroke(
                    center,
                    4.0,
                    egui::Stroke::new(marker_stroke_w, marker_stroke),
                );

                // --- Подпись ---
                let label_color = if is_selected {
                    egui::Color32::from_rgb(255, 235, 150)
                } else if is_hovered {
                    egui::Color32::from_gray(255)
                } else {
                    egui::Color32::from_gray(230)
                };

                painter.text(
                    label_anchor + egui::vec2(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", node.addr),
                    egui::FontId::proportional(12.0),
                    label_color,
                );

                // --- Детект клика ---
                if let Some(cp) = click_pos {
                    if (cp - center).length() < 8.0 {
                        clicked_node = Some(idx);
                    }
                }
            }
        }

        // --- Отрисовка транзакций (стрелок), сгруппированных по направлению ---
        if let Ok(transactions) = self.state.transactions.lock() {
            use std::collections::HashMap;

            // 1) Группируем по НЕупорядоченной паре конечных точек.
            //    Т.е. A->B и B->A попадают в одну группу.
            let mut groups: HashMap<((u32, u32), (u32, u32)), Vec<usize>> = HashMap::new();
            for (idx, tx) in transactions.iter().enumerate() {
                let a = (tx.from_position[0].to_bits(), tx.from_position[1].to_bits());
                let b = (tx.to_position[0].to_bits(), tx.to_position[1].to_bits());
                let key = if a <= b { (a, b) } else { (b, a) };
                groups.entry(key).or_default().push(idx);
            }

            for (_pair_key, idxs) in &groups {
                // 2) Разбиваем группу по конкретному направлению.
                let mut dir_map: HashMap<(u32, u32, u32, u32), Vec<usize>> = HashMap::new();
                for &idx in idxs {
                    let tx = &transactions[idx];
                    let dk = (
                        tx.from_position[0].to_bits(), tx.from_position[1].to_bits(),
                        tx.to_position[0].to_bits(), tx.to_position[1].to_bits(),
                    );
                    dir_map.entry(dk).or_default().push(idx);
                }

                // Если направлений больше одного — стрелки нужно развести в стороны.
                let bidirectional = dir_map.len() > 1;

                for (dk, mut tx_idxs) in dir_map {
                    // 3) Сортируем пакеты по времени (по возрастанию).
                    tx_idxs.sort_by_key(|&i| transactions[i].time);

                    let first = &transactions[tx_idxs[0]];
                    let mut from = to_screen(first.from_position);
                    let mut to   = to_screen(first.to_position);

                    // Смещаем вдоль перпендикуляра. Для A->B и B->A
                    // перпендикуляры противоположны, поэтому "+offset" разводит
                    // встречные стрелки по разные стороны от осевой линии.
                    let raw = to - from;
                    let rl = raw.length();
                    if rl > 0.5 {
                        let dir_n = raw / rl;
                        let perp = egui::vec2(-dir_n.y, dir_n.x);
                        let off = if bidirectional { 10.0 } else { 0.0 };
                        from += perp * off;
                        to   += perp * off;
                    }

                    let is_selected = self.selected_tx == Some(dk);
                    let color = if is_selected {
                        egui::Color32::from_rgb(255, 210, 80)
                    } else {
                        egui::Color32::from_rgb(110, 220, 140)
                    };
                    let stroke_w = if is_selected { 3.0 } else { 1.8 };

                    painter.line_segment([from, to], egui::Stroke::new(stroke_w, color));

                    // Наконечник
                    let dir = to - from;
                    let len = dir.length();
                    if len > 1.0 {
                        let dir_n = dir / len;
                        let perp = egui::vec2(-dir_n.y, dir_n.x);
                        let arrow_len = 12.0f32.min(len * 0.5);
                        let arrow_w   = 6.0f32;
                        let base = to - dir_n * arrow_len;
                        let p1 = base + perp * arrow_w;
                        let p2 = base - perp * arrow_w;
                        painter.add(egui::Shape::convex_polygon(
                            vec![to, p1, p2],
                            color,
                            egui::Stroke::NONE,
                        ));
                    }

                    // 4) Детект клика: выбираем БЛИЖАЙШУЮ стрелку,
                    //    чтобы не было неоднозначности при перекрытии хитбоксов.
                    if let Some(cp) = click_pos {
                        let d = dist_point_to_segment(cp, from, to);
                        if d < 8.0 && best_arrow_dist.map_or(true, |bd| d < bd) {
                            best_arrow_dist = Some(d);
                            clicked_dir_key = Some(dk);
                        }
                    }

                    // 5) Бейдж с количеством пакетов в этом направлении.
                    let n = tx_idxs.len();
                    if n > 1 {
                        let mid = from + (to - from) * 0.5;
                        let r = 9.0;
                        painter.circle_filled(mid, r, egui::Color32::from_black_alpha(210));
                        painter.circle_stroke(mid, r, egui::Stroke::new(1.0, color));
                        painter.text(
                            mid,
                            egui::Align2::CENTER_CENTER,
                            format!("{}", n),
                            egui::FontId::monospace(11.0),
                            egui::Color32::from_gray(245),
                        );
                    }
                }
            }
        }

        // --- Разрешение выделения: нода приоритетнее стрелки ---
        if response.clicked() {
            if clicked_node.is_some() {
                self.selected_node = clicked_node;
                self.selected_tx = None;
                self.node_selected_at = Some(Instant::now());
            } else if let Some(dk) = clicked_dir_key {
                self.selected_tx = Some(dk);
                self.selected_node = None;
                self.node_selected_at = None;
            } else {
                self.selected_tx = None;
                self.selected_node = None;
                self.node_selected_at = None;
            }
        }

        // --- Панель с информацией внизу ---
        let mut info_text: Option<String> = None;

        if let Some(idx) = self.selected_node {
            if let Ok(nodes) = self.state.nodes.lock() {
                if let Some(node) = nodes.get(idx) {
                    // --- Ширины колонок (в моноширинных символах) ---
                    const COL_ADDR:    usize = 6;
                    const COL_NAME:    usize = 16;
                    const COL_HOPS:    usize = 4;
                    const COL_RSSI:   usize = 4;
                    const COL_LAST:    usize = 8;
                    const COL_IDENT:   usize = 24;
                    const COL_ROLES:   usize = 24;
                    const COL_NEIGH:   usize = 20;

                    const COL_TARGET:  usize = 8;
                    const COL_GATEWAY: usize = 14;
                    const COL_RHOPS:   usize = 6;

                    // --- Заголовок ---
                    let mut text = format!("Node {}:", node.addr);

                    // ===== Routing table =====
                    text.push_str("\n\nRouting table:");
                    if node.routing.is_empty() {
                        text.push_str("\n  (empty)");
                    } else {
                        text.push_str(&format!(
                            "\n  {} {} {}",
                            pad_cell("target",  COL_TARGET),
                            pad_cell("gateway", COL_GATEWAY),
                            pad_cell("hops",    COL_RHOPS),
                        ));
                        text.push_str(&format!(
                            "\n  {} {} {}",
                            "-".repeat(COL_TARGET),
                            "-".repeat(COL_GATEWAY),
                            "-".repeat(COL_RHOPS),
                        ));
                        for r in &node.routing {
                            text.push_str(&format!(
                                "\n  {} {} {}",
                                pad_cell(&r.target.to_string(),  COL_TARGET),
                                pad_cell(&r.gateway.to_string(), COL_GATEWAY),
                                pad_cell(&r.hops.to_string(),    COL_RHOPS),
                            ));
                        }
                    }

                    // ===== Known nodes =====
                    text.push_str("\n\nKnown nodes:");
                    if node.known_nodes.is_empty() {
                        text.push_str("\n  (empty)");
                    } else {
                        text.push_str(&format!(
                            "\n  {} {} {} {} {} {} {} {}",
                            pad_cell("addr",       COL_ADDR),
                            pad_cell("name",       COL_NAME),
                            pad_cell("hops",       COL_HOPS),
                            pad_cell("hops",       COL_RSSI),
                            pad_cell("last",       COL_LAST),
                            pad_cell("identy",     COL_IDENT),
                            pad_cell("roles",      COL_ROLES),
                            pad_cell("neighbours", COL_NEIGH),
                        ));
                        text.push_str(&format!(
                            "\n  {} {} {} {} {} {} {} {}",
                            "-".repeat(COL_ADDR),
                            "-".repeat(COL_NAME),
                            "-".repeat(COL_HOPS),
                            "-".repeat(COL_RSSI),
                            "-".repeat(COL_LAST),
                            "-".repeat(COL_IDENT),
                            "-".repeat(COL_ROLES),
                            "-".repeat(COL_NEIGH),
                        ));

                        for kn in &node.known_nodes {
                            let name = kn
                                .name
                                .as_deref()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            // last_seen считаем от момента клика (снимок), а не от now().
                            let last_seen_secs = match self.node_selected_at {
                                Some(t) => t.saturating_duration_since(kn.last_seen).as_secs(),
                                None => 0,
                            };

                            // EnumSet'ы — через запятую именами вариантов.
                            let identy = {
                                let s = fmt_enumset(&kn.identy_type);
                                if s.is_empty() { "Unknown".to_string() } else { s }
                            };
                            let roles = match &kn.roles {
                                Some(r) => {
                                    let s = fmt_enumset(r);
                                    if s.is_empty() { "Unknown".to_string() } else { s }
                                }
                                None => "Unknown".to_string(),
                            };

                            let neighbours = match &kn.neighbour_nodes {
                                Some(v) if !v.is_empty() => v
                                    .iter()
                                    .map(|x| x.addr.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                _ => "Unknown".to_string(),
                            };
                            let last_str = format!("{}s", last_seen_secs);

                            text.push_str(&format!(
                                "\n  {} {} {} {} {} {} {} {}",
                                pad_cell(&kn.addr.to_string(), COL_ADDR),
                                pad_cell(&name,                COL_NAME),
                                pad_cell(&kn.hops.to_string(), COL_HOPS),
                                pad_cell(&kn.rssi.map(|v| v.to_string()).unwrap_or(" ".to_string()), COL_RSSI),
                                pad_cell(&last_str,            COL_LAST),
                                pad_cell(&identy,              COL_IDENT),
                                pad_cell(&roles,               COL_ROLES),
                                pad_cell(&neighbours,          COL_NEIGH),
                            ));
                        }
                    }

                    info_text = Some(text);
                }
            }
        } else if let Some(dk) = self.selected_tx {
            if let Ok(transactions) = self.state.transactions.lock() {
                // Собираем ВСЕ транзакции данного направления
                let mut matching: Vec<&RenderTransaction> = transactions
                    .iter()
                    .filter(|tx| {
                        tx.from_position[0].to_bits() == dk.0
                            && tx.from_position[1].to_bits() == dk.1
                            && tx.to_position[0].to_bits()   == dk.2
                            && tx.to_position[1].to_bits()   == dk.3
                    })
                    .collect();

                // Сортируем по времени
                matching.sort_by_key(|tx| tx.time);

                if !matching.is_empty() {
                    let total = matching.len();
                    let mut text = String::new();

                    for (i, tx) in matching.iter().enumerate() {
                        let packet_id = &tx.packet.id;
                        let packet_source = &tx.packet.source;
                        let packet_dest = &tx.packet.dest;
                        let packet_hops = &tx.packet.hops;

                        let (tp, body) = match &tx.packet.body {
                            Some(proto::packet::Body::Datagram(datagram)) => (
                                "DATAGRAM",
                                format!(
                                    "nounce: {}, data: {}",
                                    const_hex::encode_upper(datagram.nounce.clone()),
                                    const_hex::encode_upper(datagram.data.clone()),
                                )
                            ),
                            Some(proto::packet::Body::Preq(preq)) => (
                                "PREQ",
                                format!(
                                    "hops: {}, ttl: {}, dest: {}, source: {}",
                                    preq.hops, preq.ttl, preq.dest, preq.source
                                ),
                            ),
                            Some(proto::packet::Body::Prep(prep)) => (
                                "PREP",
                                format!(
                                    "hops: {}, dest: {}, source: {}",
                                    prep.hops, prep.dest, prep.source
                                ),
                            ),
                            Some(proto::packet::Body::Rerr(rerr)) => (
                                "RERR",
                                format!(
                                    "original_dest: {}",
                                    rerr.original_dest
                                ),
                            ),
                            Some(proto::packet::Body::Beacon(beacon)) => (
                                "BEACON",
                                format!(
                                    "name: {}, roles: {:?}, neighbour: {:?}",
                                    beacon.name, EnumSet::<DeviceRole>::from_u32(beacon.roles), beacon.neighbour
                                ),
                            ),
                            Some(proto::packet::Body::Hello(_)) => (
                                "HELLO",
                                "".to_owned(),
                            ),
                            None => ("UNKNOWN", "".to_string()),
                        };

                        if i > 0 {
                            text.push('\n');
                        }
                        // Нумерация 1..N
                        text.push_str(&format!(
                            "[{}/{}] <type: {}, id: {}, source: {}, dest: {}, hops: {}>: {}",
                            i + 1,
                            total,
                            tp,
                            packet_id,
                            packet_source,
                            packet_dest,
                            packet_hops,
                            body
                        ));
                    }
                    info_text = Some(text);
                }
            }
        }

        if let Some(text) = info_text {
            // Галей текстовой информации
            let galley = painter.layout(
                text,
                egui::FontId::monospace(13.0),
                egui::Color32::from_gray(235),
                rect.width() - 24.0,
            );
            let info_size = galley.size();

            // Прямоугольник под текст (прижат к низу).
            let info_bg = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + 12.0,
                    rect.bottom() - 12.0 - info_size.y - 12.0,
                ),
                info_size + egui::vec2(12.0, 12.0),
            );

            // Прямоугольник под мини-граф (над текстом).
            const GRAPH_W: f32 = 300.0;
            const GRAPH_H: f32 = 220.0;
            const GAP: f32     = 8.0;
            let graph_bg = egui::Rect::from_min_size(
                egui::pos2(info_bg.left(), info_bg.top() - GAP - GRAPH_H),
                egui::vec2(GRAPH_W, GRAPH_H),
            );

            // Фон и текст информационной панели.
            painter.rect_filled(info_bg, 4.0, egui::Color32::from_black_alpha(180));
            painter.galley(
                info_bg.min + egui::vec2(6.0, 6.0),
                galley,
                egui::Color32::from_gray(235),
            );

            // Мини-граф — только когда выбрана нода.
            if let Some(idx) = self.selected_node {
                if let Ok(nodes) = self.state.nodes.lock() {
                    if let Some(node) = nodes.get(idx) {
                        painter.rect_filled(graph_bg, 4.0, egui::Color32::from_black_alpha(180));

                        painter.text(
                            graph_bg.left_top() + egui::vec2(8.0, 5.0),
                            egui::Align2::LEFT_TOP,
                            format!("Known graph of node {}", node.addr),
                            egui::FontId::monospace(11.0),
                            egui::Color32::from_gray(180),
                        );

                        let inner = egui::Rect::from_min_max(
                            graph_bg.min + egui::vec2(18.0, 26.0),
                            graph_bg.max - egui::vec2(18.0, 10.0),
                        );
                        let layout = layout_known_nodes(node.addr, &node.known_nodes, inner);
                        let ego_pos = layout[&node.addr];

                        // --- Рёбра между known_nodes (без дублей A↔B) ---
                        let mut seen_edges: std::collections::HashSet<(u32, u32)> =
                            std::collections::HashSet::new();
                        let edge_color = egui::Color32::from_gray(110);
                        for kn in &node.known_nodes {
                            if let Some(neighs) = &kn.neighbour_nodes {
                                for nb in neighs {
                                    let key = if kn.addr <= nb.addr {
                                        (kn.addr, nb.addr)
                                    } else {
                                        (nb.addr, kn.addr)
                                    };
                                    if !seen_edges.insert(key) {
                                        continue;
                                    }
                                    if let (Some(&a), Some(&b)) =
                                        (layout.get(&kn.addr), layout.get(&nb.addr))
                                    {
                                        painter.line_segment(
                                            [a, b],
                                            egui::Stroke::new(1.0, edge_color),
                                        );
                                    }
                                }
                            }
                        }

                        // --- Рёбра от эго к соседям hops == 1 ---
                        let ego_edge_color = egui::Color32::from_rgb(255, 205, 110);
                        for kn in &node.known_nodes {
                            if kn.hops == 1 {
                                if let Some(&p) = layout.get(&kn.addr) {
                                    painter.line_segment(
                                        [ego_pos, p],
                                        egui::Stroke::new(1.4, ego_edge_color),
                                    );
                                }
                            }
                        }

                        // --- Обычные узлы ---
                        let dot_fill  = egui::Color32::from_rgb(200, 200, 200);
                        let dot_edge  = egui::Color32::from_gray(40);
                        let label_col = egui::Color32::from_gray(225);
                        for kn in &node.known_nodes {
                            if let Some(&p) = layout.get(&kn.addr) {
                                painter.circle_filled(p, 3.0, dot_fill);
                                painter.circle_stroke(p, 3.0, egui::Stroke::new(1.0, dot_edge));
                                painter.text(
                                    p + egui::vec2(5.0, -5.0),
                                    egui::Align2::LEFT_BOTTOM,
                                    format!("{}", kn.addr),
                                    egui::FontId::monospace(10.0),
                                    label_col,
                                );
                            }
                        }

                        // --- Эго-нода поверх всех, визуально выделена ---
                        painter.circle_filled(
                            ego_pos,
                            9.0,
                            egui::Color32::from_rgba_unmultiplied(255, 205, 110, 60),
                        );
                        painter.circle_stroke(
                            ego_pos,
                            6.0,
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(255, 205, 110)),
                        );
                        painter.circle_filled(ego_pos, 4.0, egui::Color32::from_rgb(255, 205, 110));
                        painter.circle_stroke(
                            ego_pos,
                            4.0,
                            egui::Stroke::new(1.5, egui::Color32::WHITE),
                        );
                        painter.text(
                            ego_pos + egui::vec2(8.0, -8.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("{}", node.addr),
                            egui::FontId::monospace(10.0),
                            egui::Color32::from_rgb(255, 235, 150),
                        );
                    }
                }
            }
        }

        // Подсказка
        painter.text(
            rect.left_top() + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!(
                "LMB — drag | wheel — zoom ({:.2}x) | nodes: {}",
                self.zoom,
                self.state.nodes.lock().unwrap().len()
            ),
            egui::FontId::monospace(12.0),
            egui::Color32::from_gray(180),
        );
    }
}

