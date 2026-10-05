use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, Weak},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

static LOCKS: OnceLock<Mutex<HashMap<i64, Weak<RwLock<()>>>>> = OnceLock::new();
pub type ReadLease = Arc<OwnedRwLockReadGuard<()>>;

fn lock(id: i64) -> Arc<RwLock<()>> {
    let mut locks = LOCKS.get_or_init(Default::default).lock();
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&id).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(RwLock::new(()));
    locks.insert(id, Arc::downgrade(&lock));
    lock
}

pub fn read(id: i64) -> Result<ReadLease> {
    lock(id)
        .try_read_owned()
        .map(Arc::new)
        .map_err(|_| anyhow!("正在删除此漫画，请稍后再试"))
}

pub fn delete(id: i64) -> Result<OwnedRwLockWriteGuard<()>> {
    lock(id)
        .try_write_owned()
        .map_err(|_| anyhow!("此漫画有下载、导出或删除操作，请完成或取消任务后再删除"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_waits_for_last_download_image_or_export_lease() {
        let first = read(-90001).unwrap();
        let image = first.clone();
        let second = read(-90001).unwrap();
        assert!(delete(-90001).is_err());
        drop(first);
        drop(second);
        assert!(delete(-90001).is_err());
        drop(image);
        let deleting = delete(-90001).unwrap();
        assert!(read(-90001).is_err());
        assert!(delete(-90001).is_err());
        drop(deleting);
        assert!(read(-90001).is_ok());
    }
}
