use chaos_tests::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock, Weak};

fn sem_store() -> RwLock<BTreeMap<u32, Weak<SemArr>>> {
    RwLock::new(BTreeMap::new())
}

fn shm_store() -> RwLock<BTreeMap<usize, Weak<Mutex<Vec<usize>>>>> {
    RwLock::new(BTreeMap::new())
}

#[test]
fn audit_semarr_rejects_zero_semaphore_create() {
    let store = sem_store();

    assert!(SemArr::get_or_create(1, 0, 1 << 9, &store).is_err());
}

#[test]
fn audit_semarr_existing_key_rejects_larger_nsems_request() {
    let store = sem_store();
    let first = SemArr::get_or_create(7, 1, 1 << 9, &store).unwrap();

    let second = SemArr::get_or_create(7, 2, 0, &store);

    assert!(second.is_err());
    assert_eq!(first.ds.lock().unwrap().nsems, 1);
}

#[test]
fn audit_semctx_remove_clears_stale_undo_before_id_reuse() {
    let store = sem_store();
    let first = SemArr::get_or_create(10, 1, 1 << 9, &store).unwrap();
    let second = SemArr::get_or_create(11, 1, 1 << 9, &store).unwrap();

    {
        let mut ctx = SemCtx::default();
        let id = ctx.add(first);
        ctx.add_undo(id, 0, -1);
        ctx.remove(id);
        assert_eq!(ctx.add(second.clone()), id);
    }

    assert_eq!(second[0].get_val(), 0);
}

#[test]
fn audit_semctx_drop_replays_full_undo_magnitude() {
    let store = sem_store();
    let arr = SemArr::get_or_create(12, 1, 1 << 9, &store).unwrap();

    {
        let mut ctx = SemCtx::default();
        let id = ctx.add(arr.clone());
        ctx.add_undo(id, 0, -3);
    }

    assert_eq!(arr[0].get_val(), 3);
}

#[test]
fn audit_shm_private_key_creates_unique_segments() {
    let store = shm_store();

    let first = shm_get_or_create(0, 1, &store);
    let second = shm_get_or_create(0, 1, &store);

    assert!(!Arc::ptr_eq(&first, &second));
}

#[test]
fn audit_shm_existing_key_rejects_larger_size_request() {
    let store = shm_store();
    let _first = shm_get_or_create(5, 1, &store);
    let second = shm_get_or_create(5, 2, &store);

    assert!(second.lock().unwrap().len() >= 2);
}
