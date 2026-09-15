use super::*;
use crate::events::model::MouseEvent;
use crate::session::record::pause_totals::PauseTotals;

#[test]
fn paused_span_is_subtracted_before_reaching_the_collector() {
    let ledger = PauseTotals::new();
    ledger.pause(1000);
    ledger.resume(3000);

    let mut collector = EventCollector::new(0);
    let t = ledger.stamp(3100);
    collector.push(t, EventKind::Move, 5, 5, None);

    let events = collector.take();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].t, 1100);
}

#[test]
fn a_switched_display_is_remapped_before_reaching_the_collector() {
    let remap = Remap::for_switch((1920, 0), (1280, 800), (0, 0), (1920, 1080));

    let mut collector = EventCollector::new(0);
    let (x, y) = remapped(Some(&remap), 1920 + 640, 400);
    collector.push(0, EventKind::Move, x, y, None);
    let (x, y) = remapped(None, 1920 + 640, 400);
    collector.push(20, EventKind::Down, x, y, Some(Button::Left));

    let events = collector.take();
    assert_eq!(
        (events[0].x, events[0].y),
        (960, 540),
        "not fitted into the canvas"
    );
    assert_eq!(
        (events[1].x, events[1].y),
        (2560, 400),
        "cleared remap still mapped"
    );
}

#[test]
fn a_polled_position_lands_as_a_move_through_the_hook_stamp_site() {
    push_polled_move(1, 1);
    let ledger = Arc::new(PauseTotals::new());
    *SINK.lock().unwrap() = Some(Sink {
        start: Instant::now(),
        collector: EventCollector::new(0),
        ledger: ledger.clone(),
        remap: Some(Remap::for_switch(
            (1920, 0),
            (1280, 800),
            (0, 0),
            (1920, 1080),
        )),
    });
    push_polled_move(1920 + 640, 400);
    push_polled_move(1920 + 640, 400);
    let events: Vec<MouseEvent> = SINK
        .lock()
        .unwrap()
        .take()
        .map(|s| s.collector.take())
        .unwrap_or_default();
    assert_eq!(
        events.len(),
        1,
        "a repeated poll must not duplicate the sample"
    );
    assert_eq!(events[0].kind, EventKind::Move);
    assert_eq!(
        (events[0].x, events[0].y),
        (960, 540),
        "polled points take the display remap too"
    );
    push_polled_move(2, 2);
    assert!(SINK.lock().unwrap().is_none());
}
