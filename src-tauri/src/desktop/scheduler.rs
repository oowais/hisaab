//! One thread, one timer (Befikr's scheduler): sleeps until the earliest
//! reminder, asks the core what to show and re-arms.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hisaab_core::Reminder;
use tauri::{AppHandle, Manager};

/// Wake at least this often: wall-clock time moves on while the machine
/// sleeps, but a timed wait doesn't.
const MAX_SLEEP: Duration = Duration::from_secs(30);

/// Next reminder per subscription.
type Alarms = HashMap<String, Reminder>;

#[derive(Clone, Default)]
pub struct Scheduler(Arc<(Mutex<Alarms>, Condvar)>);

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl Scheduler {
    pub fn start(app: AppHandle) -> Self {
        let s = Self::default();
        let me = s.clone();
        std::thread::Builder::new()
            .name("reminders".into())
            .spawn(move || me.run(&app))
            .expect("spawn reminders thread");
        s
    }

    fn alarms(&self) -> MutexGuard<'_, Alarms> {
        self.0 .0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Arm exactly these reminders.
    pub fn replace_all(&self, reminders: Vec<Reminder>) {
        *self.alarms() = reminders.into_iter().map(|r| (r.subscription_id.clone(), r)).collect();
        self.0 .1.notify_one();
    }

    fn run(&self, app: &AppHandle) {
        let mut alarms = self.alarms();
        loop {
            let now = now_ms();
            let due: Vec<String> = alarms.values().filter(|a| a.at_ms <= now).map(|a| a.subscription_id.clone()).collect();
            if !due.is_empty() {
                for id in &due {
                    alarms.remove(id);
                }
                drop(alarms);
                for id in due {
                    self.fire(app, &id);
                }
                alarms = self.alarms();
                continue;
            }
            let wait = alarms
                .values()
                .map(|a| Duration::from_millis((a.at_ms - now).max(0) as u64))
                .min()
                .unwrap_or(MAX_SLEEP)
                .min(MAX_SLEEP);
            alarms = self.0 .1.wait_timeout(alarms, wait).map_or_else(|e| e.into_inner().0, |(g, _)| g);
        }
    }

    /// Same as the Android receiver: the core decides what to show.
    fn fire(&self, app: &AppHandle, id: &str) {
        let db = app.state::<crate::Db>();
        let out = match db.store.lock() {
            Ok(mut s) => s.on_fired(id, now_ms(), &super::local_tz()),
            Err(_) => return,
        };
        match out {
            Ok(o) => {
                if let Some(r) = &o.show {
                    super::notify::show(app, r);
                }
                if let Some(next) = o.next {
                    self.alarms().insert(next.subscription_id.clone(), next);
                    self.0 .1.notify_one();
                }
            }
            Err(e) => eprintln!("reminder {id}: {e}"),
        }
    }
}
