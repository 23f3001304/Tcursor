use crate::events::model::{Button, EventKind, MouseEvent};

pub struct EventCollector {
    move_min_interval_ms: u32,
    events: Vec<MouseEvent>,
    last_move_t: Option<u32>,
    last_move_xy: Option<(i32, i32)>,
}

impl EventCollector {
    pub fn new(move_min_interval_ms: u32) -> Self {
        Self {
            move_min_interval_ms,
            events: Vec::new(),
            last_move_t: None,
            last_move_xy: None,
        }
    }

    pub fn push(&mut self, t_ms: u32, kind: EventKind, x: i32, y: i32, button: Option<Button>) {
        if matches!(kind, EventKind::Move) {
            if self.last_move_xy == Some((x, y)) {
                return;
            }
            if let Some(last) = self.last_move_t {
                if t_ms.saturating_sub(last) < self.move_min_interval_ms {
                    return;
                }
            }
            self.last_move_t = Some(t_ms);
            self.last_move_xy = Some((x, y));
        }
        self.events.push(MouseEvent {
            t: t_ms,
            kind,
            x,
            y,
            button,
        });
    }

    pub fn take(self) -> Vec<MouseEvent> {
        self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::model::{Button, EventKind};

    #[test]
    fn throttles_moves_but_keeps_clicks() {
        let mut c = EventCollector::new(10);
        c.push(0, EventKind::Move, 1, 1, None);
        c.push(3, EventKind::Move, 2, 2, None);
        c.push(12, EventKind::Move, 3, 3, None);
        c.push(13, EventKind::Down, 3, 3, Some(Button::Left));
        c.push(14, EventKind::Up, 3, 3, Some(Button::Left));
        let ev = c.take();
        assert_eq!(ev.len(), 4);
        assert_eq!(ev[0].t, 0);
        assert_eq!(ev[1].t, 12);
        assert_eq!(ev[2].kind, EventKind::Down);
    }

    #[test]
    fn drops_duplicate_move_coords() {
        let mut c = EventCollector::new(0);
        c.push(0, EventKind::Move, 5, 5, None);
        c.push(20, EventKind::Move, 5, 5, None);
        c.push(40, EventKind::Move, 6, 5, None);
        assert_eq!(c.take().len(), 2);
    }
}
