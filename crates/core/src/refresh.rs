//! One background task per card: refresh it on its interval, send the view to the UI.
//!
//! The UI never awaits a card. It only reads whatever arrives on the channel.

use std::{sync::Arc, time::Duration};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};

use crate::card::{Card, CardView};

/// A refresh that takes longer than this counts as an error.
pub const REFRESH_TIMEOUT: Duration = Duration::from_secs(3);

/// Actions get longer: connecting headphones can take a few seconds.
pub const ACTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Guard against `Duration::ZERO` and busy loops.
const MIN_INTERVAL: Duration = Duration::from_millis(100);

/// A fresh view for the card at position `slot`.
#[derive(Debug)]
pub struct Update {
    pub slot: usize,
    pub view: CardView,
}

/// Starts one refresh task per card.
pub fn spawn_all(cards: &[Arc<dyn Card>], tx: mpsc::Sender<Update>) -> Vec<JoinHandle<()>> {
    cards.iter().enumerate().map(|(slot, card)| tokio::spawn(refresh_loop(slot, card.clone(), tx.clone()))).collect()
}

async fn refresh_loop(slot: usize, card: Arc<dyn Card>, tx: mpsc::Sender<Update>) {
    let Some(every) = card.refresh_interval() else {
        // Push-only card: one initial view, later updates come from elsewhere.
        let _ = tx.send(Update { slot, view: refresh_once(card.as_ref()).await }).await;
        return;
    };

    let mut ticker = time::interval(every.max(MIN_INTERVAL));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await; // the first tick completes immediately
        let view = refresh_once(card.as_ref()).await;
        if tx.send(Update { slot, view }).await.is_err() {
            return; // receiver gone: the UI has shut down
        }
    }
}

/// Runs one card action in the background. Afterwards the card refreshes right away,
/// so the result shows without waiting for the next tick. A failure becomes an error view.
pub fn spawn_action(slot: usize, card: Arc<dyn Card>, action: String, arg: Option<String>, tx: mpsc::Sender<Update>) {
    tokio::spawn(async move {
        let view = match time::timeout(ACTION_TIMEOUT, card.run(&action, arg.as_deref())).await {
            Ok(Ok(())) => refresh_once(card.as_ref()).await,
            Ok(Err(e)) => CardView::error(e.to_string()),
            Err(_) => CardView::error(format!("{action}: timed out")),
        };
        let _ = tx.send(Update { slot, view }).await; // UI gone: nobody to tell
    });
}

/// Never fails: errors and timeouts become an error view.
async fn refresh_once(card: &dyn Card) -> CardView {
    match time::timeout(REFRESH_TIMEOUT, card.refresh()).await {
        Ok(Ok(view)) => view,
        Ok(Err(e)) => CardView::error(e.to_string()),
        Err(_) => CardView::error("timed out"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use super::*;
    use crate::card::{Line, State};

    enum Behaviour {
        Count,
        Fail,
        Hang,
    }

    struct Fake {
        interval: Option<Duration>,
        behaviour: Behaviour,
        calls: AtomicUsize,
    }

    fn fake(interval: Option<Duration>, behaviour: Behaviour) -> Arc<dyn Card> {
        Arc::new(Fake { interval, behaviour, calls: AtomicUsize::new(0) })
    }

    #[async_trait]
    impl Card for Fake {
        fn id(&self) -> &str {
            "fake"
        }
        fn title(&self) -> &str {
            "Fake"
        }
        fn refresh_interval(&self) -> Option<Duration> {
            self.interval
        }

        async fn refresh(&self) -> anyhow::Result<CardView> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            match self.behaviour {
                Behaviour::Count => Ok(CardView::new(State::Ok).line(Line::new(n.to_string()))),
                Behaviour::Fail => anyhow::bail!("boom"),
                Behaviour::Hang => {
                    time::sleep(Duration::from_secs(60)).await;
                    unreachable!("should have timed out")
                }
            }
        }

        async fn run(&self, action: &str, arg: Option<&str>) -> anyhow::Result<()> {
            match action {
                "ok" => Ok(()),
                "echo" => anyhow::bail!("got {arg:?}"),
                "hang" => {
                    time::sleep(Duration::from_secs(60)).await;
                    unreachable!("should have timed out")
                }
                other => anyhow::bail!("no action `{other}`"),
            }
        }
    }

    fn text(view: &CardView) -> &str {
        &view.lines[0].text
    }

    // `start_paused`: tokio's clock is frozen and jumps ahead whenever all tasks
    // are waiting, so a 60 s sleep finishes instantly. No real waiting in tests.

    #[tokio::test(start_paused = true)]
    async fn refreshes_on_its_interval() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_all(&[fake(Some(Duration::from_secs(1)), Behaviour::Count)], tx);

        for expected in ["1", "2", "3"] {
            let update = rx.recv().await.unwrap();
            assert_eq!(update.slot, 0);
            assert_eq!(text(&update.view), expected);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn slots_match_card_positions() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_all(&[fake(None, Behaviour::Count), fake(None, Behaviour::Count)], tx);

        let mut slots = vec![rx.recv().await.unwrap().slot, rx.recv().await.unwrap().slot];
        slots.sort();
        assert_eq!(slots, vec![0, 1]);
    }

    #[tokio::test(start_paused = true)]
    async fn push_only_card_refreshes_once() {
        let (tx, mut rx) = mpsc::channel(8);
        let tasks = spawn_all(&[fake(None, Behaviour::Count)], tx);

        assert_eq!(text(&rx.recv().await.unwrap().view), "1");
        assert!(rx.recv().await.is_none(), "channel should close: task is done");
        assert!(tasks[0].is_finished());
    }

    #[tokio::test(start_paused = true)]
    async fn error_becomes_error_view() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_all(&[fake(None, Behaviour::Fail)], tx);

        let view = rx.recv().await.unwrap().view;
        assert_eq!(view.state, State::Error);
        assert_eq!(text(&view), "boom");
    }

    #[tokio::test(start_paused = true)]
    async fn slow_refresh_times_out() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_all(&[fake(None, Behaviour::Hang)], tx);

        let view = rx.recv().await.unwrap().view;
        assert_eq!(view.state, State::Error);
        assert_eq!(text(&view), "timed out");
    }

    #[tokio::test(start_paused = true)]
    async fn task_stops_when_ui_is_gone() {
        let (tx, rx) = mpsc::channel(8);
        let tasks = spawn_all(&[fake(Some(Duration::from_secs(1)), Behaviour::Count)], tx);
        drop(rx);

        tasks.into_iter().next().unwrap().await.unwrap(); // returns instead of looping forever
    }

    #[tokio::test(start_paused = true)]
    async fn action_then_fresh_view() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_action(3, fake(None, Behaviour::Count), "ok".to_string(), None, tx);

        let update = rx.recv().await.unwrap();
        assert_eq!(update.slot, 3);
        assert_eq!(text(&update.view), "1"); // refreshed right after the action
    }

    #[tokio::test(start_paused = true)]
    async fn failed_action_becomes_error_view() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_action(0, fake(None, Behaviour::Count), "explode".to_string(), None, tx);

        let view = rx.recv().await.unwrap().view;
        assert_eq!(view.state, State::Error);
        assert_eq!(text(&view), "no action `explode`");
    }

    #[tokio::test(start_paused = true)]
    async fn slow_action_times_out() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_action(0, fake(None, Behaviour::Count), "hang".to_string(), None, tx);

        let view = rx.recv().await.unwrap().view;
        assert_eq!(text(&view), "hang: timed out");
    }

    #[tokio::test(start_paused = true)]
    async fn action_gets_its_argument() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_action(0, fake(None, Behaviour::Count), "echo".to_string(), Some("AirPods Pro".to_string()), tx);

        let view = rx.recv().await.unwrap().view;
        assert_eq!(text(&view), r#"got Some("AirPods Pro")"#);
    }

    #[tokio::test(start_paused = true)]
    async fn zero_interval_does_not_panic() {
        let (tx, mut rx) = mpsc::channel(8);
        spawn_all(&[fake(Some(Duration::ZERO), Behaviour::Count)], tx);

        assert_eq!(text(&rx.recv().await.unwrap().view), "1");
        assert_eq!(text(&rx.recv().await.unwrap().view), "2");
    }
}
