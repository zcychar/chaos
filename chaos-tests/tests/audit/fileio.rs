use chaos_tests::*;

fn rw_opt() -> FdOpt {
    FdOpt {
        rd: true,
        wr: true,
        ap: false,
        nb: false,
    }
}

fn append_opt() -> FdOpt {
    FdOpt {
        rd: true,
        wr: true,
        ap: true,
        nb: false,
    }
}

fn ep_event(tag: u64) -> EpEvent {
    EpEvent {
        events: EpEvent::IN,
        data: EpData { ptr: tag },
    }
}

#[test]
fn audit_pipe_clone_drop_keeps_original_writer_open() {
    let (rd, wr) = PipeNode::pair();
    let extra_writer = wr.clone();

    drop(extra_writer);

    assert!(wr.can_write());
    let mut byte = [0u8; 1];
    assert_eq!(rd.read_at(&mut byte), Err("again"));
}

#[test]
fn audit_pipe_write_after_reader_drop_errors() {
    let (rd, wr) = PipeNode::pair();

    drop(rd);

    assert!(!wr.can_write());
    assert!(wr.write_at(b"x").is_err());
}

#[test]
fn audit_fhandle_append_write_updates_offset_to_new_end() {
    let file = FHandle::with_data("append.log", append_opt(), b"abc".to_vec());

    assert_eq!(file.write(b"de"), Ok(2));
    assert_eq!(file.seek(FSeek::Cur(0)).unwrap(), 5);

    file.seek(FSeek::Start(0)).unwrap();
    let mut buf = [0u8; 5];
    assert_eq!(file.read(&mut buf), Ok(5));
    assert_eq!(&buf, b"abcde");
}

#[test]
fn audit_fhandle_negative_seek_is_rejected() {
    let file = FHandle::with_data("seek.txt", rw_opt(), b"abc".to_vec());

    assert!(file.seek(FSeek::Cur(-1)).is_err());
    assert!(file.seek(FSeek::End(-4)).is_err());
}

#[test]
fn audit_fhandle_write_at_overflow_does_not_panic() {
    let file = FHandle::new("overflow.bin", rw_opt(), false, false);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        file.write_at(usize::MAX, b"x")
    }));

    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn audit_flike_mmap_overflow_does_not_panic() {
    let file = FLike::File(FHandle::new("map.bin", rw_opt(), false, false));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        file.mmap_fl(0, usize::MAX, 0)
    }));

    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn audit_epoll_add_existing_fd_is_rejected() {
    let mut ep = EpInst::new();
    let ev = ep_event(1);

    assert_eq!(ep.control(EpCtlOp::ADD, 7, &ev), Ok(()));
    assert!(ep.control(EpCtlOp::ADD, 7, &ev).is_err());
}

#[test]
fn audit_epoll_del_clears_ready_and_ctl_state() {
    let mut ep = EpInst::new();
    let ev = ep_event(1);

    ep.control(EpCtlOp::ADD, 9, &ev).unwrap();
    ep.ready.lock().unwrap().insert(9);

    assert_eq!(ep.control(EpCtlOp::DEL, 9, &ev), Ok(()));
    assert!(!ep.ready.lock().unwrap().contains(&9));
    assert!(!ep.new_ctl.lock().unwrap().contains(&9));
}

#[test]
fn audit_epoll_dup_shares_registration_state() {
    let mut ep = EpInst::new();
    ep.control(EpCtlOp::ADD, 1, &ep_event(1)).unwrap();

    let original = FLike::Ep(ep);
    let mut duplicate = match original.dup(false) {
        FLike::Ep(ep) => ep,
        _ => unreachable!(),
    };

    duplicate.control(EpCtlOp::ADD, 2, &ep_event(2)).unwrap();

    match &original {
        FLike::Ep(ep) => assert!(ep.events.contains_key(&2)),
        _ => unreachable!(),
    }
}
