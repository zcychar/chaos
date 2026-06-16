use chaos_tests::*;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn audit_channel_send_after_close_is_rejected() {
    let channel = Channel::new(4);

    channel.close();

    assert!(!channel.send(1));
    assert_eq!(channel.depth(), 0);
}

#[test]
fn audit_channel_send_batch_after_close_is_rejected() {
    let channel = Channel::new(4);

    channel.close();

    assert_eq!(channel.send_batch(&[1, 2, 3]), 0);
    assert_eq!(channel.depth(), 0);
}

#[test]
fn audit_channel_send_batch_wakes_each_waiting_receiver() {
    let channel = Arc::new(Channel::new(4));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::new();

    for _ in 0..2 {
        let channel = channel.clone();
        let tx = tx.clone();
        handles.push(std::thread::spawn(move || {
            let _ = tx.send(channel.recv());
        }));
    }
    drop(tx);

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(channel.send_batch(&[10, 20]), 2);

    let first = rx.recv_timeout(Duration::from_millis(200)).ok();
    let second = rx.recv_timeout(Duration::from_millis(200)).ok();
    let both_woke = first.is_some() && second.is_some();

    channel.close();
    for handle in handles {
        handle.join().unwrap();
    }

    assert!(both_woke);
}
