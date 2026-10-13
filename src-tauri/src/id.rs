//! 雪花 ID（64-bit），应用层按 i64 使用。

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Twitter 风格雪花：41bit 时间 + 10bit worker + 12bit 序列
const EPOCH_MS: u64 = 1_704_067_200_000; // 2024-01-01 UTC，缩短时间位空闲
const WORKER_BITS: u64 = 10;
const SEQUENCE_BITS: u64 = 12;
const MAX_WORKER: u64 = (1 << WORKER_BITS) - 1;
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

struct State {
    worker_id: u64,
    last_ms: u64,
    sequence: u64,
}

pub struct Snowflake {
    inner: Mutex<State>,
}

impl Snowflake {
    pub fn new(worker_id: u64) -> Self {
        assert!(worker_id <= MAX_WORKER, "worker_id out of range");
        Self {
            inner: Mutex::new(State {
                worker_id,
                last_ms: 0,
                sequence: 0,
            }),
        }
    }

    pub fn next_id(&self) -> i64 {
        let mut g = self.inner.lock().expect("snowflake lock");
        let mut now = current_millis();
        if now < g.last_ms {
            // 时钟回拨：等到追上
            while now < g.last_ms {
                std::thread::yield_now();
                now = current_millis();
            }
        }
        if now == g.last_ms {
            g.sequence = (g.sequence + 1) & MAX_SEQUENCE;
            if g.sequence == 0 {
                while now <= g.last_ms {
                    std::thread::yield_now();
                    now = current_millis();
                }
            }
        } else {
            g.sequence = 0;
        }
        g.last_ms = now;
        let ts = now.saturating_sub(EPOCH_MS);
        let id = (ts << (WORKER_BITS + SEQUENCE_BITS))
            | (g.worker_id << SEQUENCE_BITS)
            | g.sequence;
        id as i64
    }
}

fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time before epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_unique_and_positive() {
        let gen = Snowflake::new(1);
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            let id = gen.next_id();
            assert!(id > 0);
            assert!(set.insert(id));
        }
    }
}
