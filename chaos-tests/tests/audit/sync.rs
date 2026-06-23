use chaos_tests::*;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn wait_for_message<T: Send + 'static>(rx: std::sync::mpsc::Receiver<T>, ms: u64) -> Option<T> {
    rx.recv_timeout(Duration::from_millis(ms)).ok()
}

#[test]
fn audit_futex_wake_one_unparks_waiter() {
    let futexes = Arc::new(FutexTable::new());
    let value = Arc::new(AtomicU32::new(7));
    let (tx, rx) = std::sync::mpsc::channel();

    {
        let futexes = futexes.clone();
        let value = value.clone();
        std::thread::spawn(move || {
            let result = futexes.ftx_wait(0x1000, 7, &value);
            let _ = tx.send(result);
        });
    }

    std::thread::sleep(Duration::from_millis(50));

    assert_eq!(futexes.ftx_wake(0x1000, 1), 1);
    assert_eq!(wait_for_message(rx, 200), Some(true));
}

#[test]
fn audit_futex_wake_zero_wakes_none() {
    let futexes = Arc::new(FutexTable::new());
    let value = Arc::new(AtomicU32::new(3));
    let (tx, rx) = std::sync::mpsc::channel();

    {
        let futexes = futexes.clone();
        let value = value.clone();
        std::thread::spawn(move || {
            let result = futexes.ftx_wait(0x2000, 3, &value);
            let _ = tx.send(result);
        });
    }

    std::thread::sleep(Duration::from_millis(50));

    let woken = futexes.ftx_wake(0x2000, 0);
    let _ = futexes.ftx_wake(0x2000, 2);

    assert_eq!(woken, 0);
    assert_eq!(wait_for_message(rx, 200), Some(true));
}

#[test]
fn audit_syncqueue_timeout_removes_waiter() {
    let q = SyncQueue::new();
    let guard = Mutex::new(());

    let _ = q.wait_timeout(&guard, Duration::from_millis(5));

    assert_eq!(q.pending(), 0);
}

#[test]
fn audit_syncqueue_wait_events_removes_stale_waiters() {
    let q1 = Arc::new(SyncQueue::new());
    let q2 = Arc::new(SyncQueue::new());
    let state = Arc::new(Mutex::new(false));
    let (tx, rx) = std::sync::mpsc::channel();

    {
        let q1 = q1.clone();
        let q2 = q2.clone();
        let state = state.clone();
        std::thread::spawn(move || {
            let result =
                SyncQueue::wait_events(
                    &[&q1, &q2],
                    &state,
                    |ready| {
                        if *ready {
                            Some(true)
                        } else {
                            None
                        }
                    },
                );
            let _ = tx.send(result);
        });
    }

    std::thread::sleep(Duration::from_millis(50));
    *state.lock().unwrap() = true;
    q1.broadcast();

    assert_eq!(wait_for_message(rx, 200), Some(true));
    assert_eq!(q1.pending(), 0);
    assert_eq!(q2.pending(), 0);
}
