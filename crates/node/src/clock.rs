//! Single seam for the node's two software clocks (V1-C05). Four time domains
//! stay distinct:
//!
//! - protocol wall time (`wall`): issued_at, lease and expiry inside signed
//!   documents;
//! - scheduler monotonic time (`instant`): due, backoff and admission windows;
//! - chain time: external adapter (Anvil block timestamps), never this module;
//! - real execution time: watchdog, IPC deadlines, elapsed_us telemetry —
//!   deliberately not sewn.
//!
//! Production installs nothing and reads the OS clocks. A simulation installs
//! one controller per process; every logical node in a managed-time reproducer
//! shares it, so an event can never observe a clock behind an already queued
//! `due`. The seam only supplies readings: creating a deadline is
//! `instant() + d`, checking it is `due <= instant()`, and wakeup ordering
//! belongs to the driver that advances the controller.
#[cfg(test)]
use std::time::Duration;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

type InstantSource = Rc<dyn Fn() -> Instant>;
type WallSource = Rc<dyn Fn() -> u64>;

thread_local! {
    static INSTANT: RefCell<Option<InstantSource>> = const { RefCell::new(None) };
    static WALL: RefCell<Option<WallSource>> = const { RefCell::new(None) };
}

/// Scheduler monotonic time: due, backoff and admission windows.
pub(in crate::runtime) fn instant() -> Instant {
    INSTANT.with(|slot| slot.borrow().as_ref().map_or_else(Instant::now, |f| f()))
}

/// Protocol wall time in seconds: issued_at, lease and expiry in signed
/// documents. Matches `SystemTime` error behaviour of the replaced `now()`.
pub(in crate::runtime) fn wall() -> crate::Result<u64> {
    WALL.with(|slot| {
        slot.borrow().as_ref().map_or_else(
            || Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
            |f| Ok(f()),
        )
    })
}

#[cfg(test)]
type Suspended = (Option<InstantSource>, Option<WallSource>);

#[cfg(test)]
thread_local! {
    static PREVIOUS: RefCell<Vec<Suspended>> = const { RefCell::new(Vec::new()) };
}

/// One managed-time controller shared by every logical node in the process.
/// Monotonic time only moves forward through `advance`; wall time is set
/// independently because protocol leases are expressed in chain-side seconds.
#[cfg(test)]
pub(in crate::runtime) struct Virtual {
    instant: Rc<std::cell::Cell<Instant>>,
    wall: Rc<std::cell::Cell<u64>>,
}

#[cfg(test)]
impl Virtual {
    pub(in crate::runtime) fn install(wall: u64) -> Self {
        let clock = Self {
            instant: Rc::new(std::cell::Cell::new(Instant::now())),
            wall: Rc::new(std::cell::Cell::new(wall)),
        };
        let instant = Rc::clone(&clock.instant);
        let wall_source = Rc::clone(&clock.wall);
        PREVIOUS.with(|stack| {
            let previous = INSTANT.with(|slot| slot.replace(Some(Rc::new(move || instant.get()))));
            let previous_wall =
                WALL.with(|slot| slot.replace(Some(Rc::new(move || wall_source.get()))));
            stack.borrow_mut().push((previous, previous_wall));
        });
        clock
    }

    pub(in crate::runtime) fn instant(&self) -> Instant {
        self.instant.get()
    }

    pub(in crate::runtime) fn wall(&self) -> u64 {
        self.wall.get()
    }

    /// Jump to the next due event: callers pass the earliest deadline across
    /// lanes so every event ordered before it is observed first.
    pub(in crate::runtime) fn advance(&self, by: Duration) {
        self.instant.set(self.instant.get() + by);
        self.wall.set(self.wall.get() + by.as_secs());
    }

    pub(in crate::runtime) fn advance_to(&self, due: Instant) {
        if let Some(by) = due.checked_duration_since(self.instant.get()) {
            self.advance(by);
        }
    }

    pub(in crate::runtime) fn set_wall(&self, wall: u64) {
        self.wall.set(wall);
    }
}

#[cfg(test)]
impl Drop for Virtual {
    fn drop(&mut self) {
        PREVIOUS.with(|stack| {
            if let Some((instant, wall)) = stack.borrow_mut().pop() {
                INSTANT.with(|slot| slot.replace(instant));
                WALL.with(|slot| slot.replace(wall));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_controller_drives_both_clocks() {
        let clock = Virtual::install(1_700_000_000);
        let started = instant();
        assert_eq!(wall().ok(), Some(1_700_000_000));
        clock.advance(Duration::from_secs(30));
        assert_eq!(instant() - started, Duration::from_secs(30));
        assert_eq!(wall().ok(), Some(1_700_000_030));
        clock.set_wall(1_800_000_000);
        assert_eq!(wall().ok(), Some(1_800_000_000));
        assert_eq!(instant() - started, Duration::from_secs(30));
    }

    #[test]
    fn runtime_wall_delegates_to_the_installed_controller() {
        let _clock = Virtual::install(1_700_000_000);
        assert_eq!(crate::runtime::now().ok(), Some(1_700_000_000));
    }

    #[test]
    fn dropped_controller_restores_the_previous_source() {
        {
            let _clock = Virtual::install(1_700_000_000);
            assert_eq!(wall().ok(), Some(1_700_000_000));
        }
        assert!(wall().is_ok_and(|wall| wall > 1_700_000_000));
    }
}
