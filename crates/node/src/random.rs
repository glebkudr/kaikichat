//! Single seam for scheduler-visible randomness (V1-C05): job ids and sync
//! nonces. Production installs nothing and reads the OS source; a managed-time
//! reproducer installs one deterministic stream shared by every logical node in
//! the process, so the same event order yields the same ids and the same trace.
use std::cell::RefCell;
use std::rc::Rc;

type Source = Rc<RefCell<dyn FnMut(&mut [u8])>>;

thread_local! {
    static SOURCE: RefCell<Option<Source>> = const { RefCell::new(None) };
    #[cfg(test)]
    static PREVIOUS: RefCell<Vec<Option<Source>>> = const { RefCell::new(Vec::new()) };
}

/// Fill `dest` from the installed source, or the OS RNG in production.
/// The error type matches the `getrandom::fill` calls this replaces.
pub(in crate::runtime) fn fill(dest: &mut [u8]) -> Result<(), getrandom::Error> {
    SOURCE.with(|slot| {
        if let Some(source) = slot.borrow().as_ref() {
            source.borrow_mut()(dest);
            return Ok(());
        }
        getrandom::fill(dest)
    })
}

/// Deterministic stream installed by a reproducer: `sha256(seed || counter)`
/// blocks, so every draw is a pure function of position. Drop restores the
/// previous source, including nested installs.
#[cfg(test)]
pub(in crate::runtime) struct Deterministic {
    _private: (),
}

#[cfg(test)]
impl Deterministic {
    pub(in crate::runtime) fn install(seed: [u8; 32]) -> Self {
        let state = Rc::new(RefCell::new((0u64, Vec::<u8>::new())));
        PREVIOUS.with(|stack| {
            stack.borrow_mut().push(SOURCE.with(|slot| {
                slot.replace(Some(Rc::new(RefCell::new(move |dest: &mut [u8]| {
                    let mut state = state.borrow_mut();
                    for chunk in dest.iter_mut() {
                        if state.1.is_empty() {
                            use sha2::{Digest, Sha256};
                            let mut block = Sha256::new();
                            block.update(seed);
                            block.update(state.0.to_be_bytes());
                            state.0 += 1;
                            state.1 = block.finalize().to_vec();
                        }
                        *chunk = state.1.remove(0);
                    }
                }))))
            }));
        });
        Self { _private: () }
    }
}

#[cfg(test)]
impl Drop for Deterministic {
    fn drop(&mut self) {
        PREVIOUS.with(|stack| {
            if let Some(previous) = stack.borrow_mut().pop() {
                SOURCE.with(|slot| slot.replace(previous));
            }
        });
    }
}
