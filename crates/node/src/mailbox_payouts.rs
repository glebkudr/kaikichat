//! An operator's prizes on its node (Docs/V1_OPERATOR_PAYOUTS_2026_09_29.md).
//! The holder service keeps a ticket for every paid stamp naming its unit;
//! this lane reads the pool's terms, the node's registry unit and the seeds
//! of the days books were bought on, draws the tickets, fixes a day's seed
//! when its turn comes, and withdraws only when the operator asks. Reads and
//! transactions run as tasks; their answers are applied on the next pump.
use super::chain::{ChainError, PoolTerms, RegistryUnit, SeedState, TicketClaim};
use super::mailbox_holder::{DrawTerms, TicketState, claim_slot};
use super::*;
use agentic_mailbox_swarm::address::PERIOD_SECONDS;
use std::collections::{BTreeMap, BTreeSet};

/// How long a failed read of the terms, the unit or the account waits.
const RETRY: Duration = Duration::from_secs(60);
/// How often the gas and what the pool owes are read again.
const ACCOUNT_EVERY: Duration = Duration::from_secs(600);
/// How often tickets drawing are looked over.
const DRAW_EVERY: Duration = Duration::from_secs(30);
/// How often a day's missing seed is read again.
const SEED_EVERY: Duration = Duration::from_secs(60);
/// How soon a seed is read again after this node armed or captured it.
const SEED_SOON: Duration = Duration::from_secs(12);
/// How often ended tickets are dropped.
const FORGET_EVERY: Duration = Duration::from_secs(3_600);
/// Units take turns fixing a day's seed over its first ten minutes.
const KEEPER_SPREAD: u64 = 600;
/// A seed's target block is readable this many blocks.
const SEED_WINDOW: u64 = 256;
/// Tickets in one claim: below the transaction size limit.
pub(super) const MAX_CLAIM: usize = 150;

type Read<T> = std::result::Result<T, ChainError>;

enum Answer {
    Terms(Read<(PoolTerms, u64)>),
    Unit([u8; 32], Read<Option<RegistryUnit>>),
    Account(Option<u128>, Option<u128>),
    Seed(u64, Read<SeedState>),
    /// Whether the pool's shop sold a book ending at `valid_until`.
    Book([u8; 32], u64, Read<Option<super::chain::BookRecord>>),
    /// This node's arm or capture of a day's seed went out or failed.
    Kept(u64, Read<[u8; 32]>),
    Withdrawn(Withdrawn),
}

/// What a withdrawal did.
#[derive(Default)]
struct Withdrawn {
    claimed: Vec<[u8; 32]>,
    refused: Vec<[u8; 32]>,
    transactions: Vec<[u8; 32]>,
    owed: Option<u128>,
    error: Option<&'static str>,
}

/// A withdrawal's chain, the pool's terms as drawn and the node's unit.
type Ready = (Arc<dyn chain::Chain>, PoolTerms, DrawTerms, RegistryUnit);

/// The last withdrawal, as `operator_earnings` shows it.
struct Withdrawal {
    state: &'static str,
    tickets: usize,
    error: Option<&'static str>,
    transactions: Vec<[u8; 32]>,
}

/// What the pool's shop says of a ticket's book.
enum BookCheck {
    Reading,
    Sold,
    /// Never sold by this shop: its tickets are dropped.
    Unsold,
    /// Unanswered, or not sold yet while its purchase day lasts: read again.
    Retry(Instant),
}

/// A day whose seed a ticket waits for.
struct Day {
    due: Instant,
    reading: bool,
}

pub(super) struct Lane {
    sender: mpsc::UnboundedSender<Answer>,
    answers: mpsc::UnboundedReceiver<Answer>,
    /// The pool's terms and the shop's book validity.
    terms: Option<(PoolTerms, u64)>,
    terms_due: Option<Instant>,
    /// The registry unit of the commitment this node holds as.
    unit: Option<([u8; 32], RegistryUnit)>,
    unit_due: Option<Instant>,
    gas: Option<u128>,
    owed: Option<u128>,
    account_due: Option<Instant>,
    seeds: BTreeMap<u64, [u8; 32]>,
    days: BTreeMap<u64, Day>,
    books: BTreeMap<[u8; 32], BookCheck>,
    draw_due: Option<Instant>,
    forget_due: Option<Instant>,
    withdrawal: Option<Withdrawal>,
}

impl Default for Lane {
    fn default() -> Self {
        let (sender, answers) = mpsc::unbounded_channel();
        Self {
            sender,
            answers,
            terms: None,
            terms_due: None,
            unit: None,
            unit_due: None,
            gas: None,
            owed: None,
            account_due: None,
            seeds: BTreeMap::new(),
            days: BTreeMap::new(),
            books: BTreeMap::new(),
            draw_due: None,
            forget_due: None,
            withdrawal: None,
        }
    }
}

impl Lane {
    /// The earliest read, draw or seed step due; nothing while no pool.
    pub(super) fn next_due(&self) -> Option<Instant> {
        [
            self.terms_due,
            self.unit_due,
            self.account_due,
            self.draw_due,
            self.forget_due,
        ]
        .into_iter()
        .flatten()
        .chain(self.days.values().filter(|d| !d.reading).map(|d| d.due))
        .min()
    }

    fn spawn(&self, work: impl std::future::Future<Output = Answer> + Send + 'static) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = sender.send(work.await);
        });
    }

    /// The books the pool's shop is known to have sold.
    fn sold(&self) -> BTreeSet<[u8; 32]> {
        self.books
            .iter()
            .filter(|(_, check)| matches!(check, BookCheck::Sold))
            .map(|(book, _)| *book)
            .collect()
    }

    fn draw_terms(&self) -> Option<DrawTerms> {
        self.terms.map(|(pool, validity)| DrawTerms {
            validity,
            lifetime: pool.ticket_lifetime,
            threshold: pool.win_threshold,
        })
    }
}

/// The share of the first ten minutes after a day ends at which `unit`
/// takes its turn to fix the day's seed.
fn keeper_turn(unit: &[u8; 32], day: u64) -> u64 {
    let mut bytes = [0; 8];
    bytes.copy_from_slice(
        &alloy_primitives::keccak256([&unit[..], &day.to_be_bytes()].concat())[..8],
    );
    u64::from_be_bytes(bytes) % KEEPER_SPREAD
}

fn after(instant: Instant, seconds: u64) -> Instant {
    instant + Duration::from_secs(seconds)
}

fn error_code(error: &ChainError) -> &'static str {
    match error {
        ChainError::NoGas => "no_gas",
        ChainError::Reverted => "refused",
        _ => "chain_unavailable",
    }
}

fn hex0x(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes))
}

impl Runtime {
    /// Tell core the registry's units, as the directory lists them, so its
    /// stamps name the holders of their mailboxes.
    pub(super) fn sync_swarm_units(&mut self) {
        let units: Vec<[u8; 32]> = self.mailbox_client.directory.keys().copied().collect();
        if units.is_empty() || units == self.swarm_units {
            return;
        }
        if self.core.set_swarm_units(units.clone()).is_ok() {
            self.swarm_units = units;
        }
    }

    pub(super) fn maintain_payouts(&mut self) {
        let Some(source) = self.chain.source.clone() else {
            return;
        };
        if !source.has_pool() {
            return;
        }
        let instant = clock::instant();
        let Ok(now) = clock::wall() else {
            return;
        };
        while let Ok(answer) = self.payouts.answers.try_recv() {
            self.payout_answer(answer, instant, now);
        }
        let lane = &mut self.payouts;
        if lane.terms.is_none() && lane.terms_due.is_none_or(|due| instant >= due) {
            lane.terms_due = Some(after(instant, 3_600));
            let source = source.clone();
            lane.spawn(async move {
                let read = async {
                    let pool = source.pool().await?;
                    let shop = source.shop().await?;
                    Ok((pool, shop.validity))
                };
                Answer::Terms(read.await)
            });
        }
        let Some(terms) = lane.draw_terms() else {
            return;
        };
        if let Some(unit) = self.mailbox_holder.unit()
            && lane.unit.is_none_or(|(held, _)| held != unit)
            && lane.unit_due.is_none_or(|due| instant >= due)
        {
            lane.unit_due = Some(after(instant, 3_600));
            let source = source.clone();
            lane.spawn(async move { Answer::Unit(unit, source.registry_unit(unit).await) });
        }
        if lane.account_due.is_none_or(|due| instant >= due) {
            lane.account_due = Some(instant + ACCOUNT_EVERY);
            let account = self.mailbox_holder.account();
            let index = lane.unit.map(|(_, unit)| unit.index);
            let source = source.clone();
            lane.spawn(async move {
                let gas = source.gas_balance(account).await.ok();
                let owed = match index {
                    Some(index) => source.owed(index).await.ok(),
                    None => None,
                };
                Answer::Account(gas, owed)
            });
        }
        if lane.draw_due.is_none_or(|due| instant >= due) {
            lane.draw_due = Some(instant + DRAW_EVERY);
            let drawing: Vec<_> = self
                .mailbox_holder
                .tickets()
                .unwrap_or_default()
                .into_iter()
                .filter(|ticket| ticket.state == TicketState::Drawing)
                .collect();
            // Only books the pool's shop sold are drawn: each is read once.
            let mut books = BTreeMap::new();
            for ticket in &drawing {
                books.insert(ticket.claim.book, ticket.valid_until);
            }
            for (book, valid_until) in &books {
                match lane.books.get(book) {
                    Some(BookCheck::Unsold) => {
                        let _ = self.mailbox_holder.forget_book_tickets(book);
                    }
                    Some(BookCheck::Sold | BookCheck::Reading) => {}
                    Some(BookCheck::Retry(due)) if instant < *due => {}
                    _ => {
                        lane.books.insert(*book, BookCheck::Reading);
                        let (book, valid_until, source) = (*book, *valid_until, source.clone());
                        lane.spawn(async move {
                            Answer::Book(book, valid_until, source.book(book).await)
                        });
                    }
                }
            }
            let sold = lane.sold();
            let days: BTreeSet<u64> = drawing
                .iter()
                .filter(|ticket| sold.contains(&ticket.claim.book))
                .map(|ticket| terms.purchase_day(ticket.valid_until))
                .collect();
            if days.iter().any(|day| lane.seeds.contains_key(day)) {
                let _ = self
                    .mailbox_holder
                    .draw_books(&terms, &lane.seeds, Some(&sold));
            }
            for day in days {
                if !lane.seeds.contains_key(&day) {
                    // Nothing to read before the day is over.
                    let over = (day + 1) * PERIOD_SECONDS;
                    lane.days.entry(day).or_insert(Day {
                        due: after(instant, over.saturating_sub(now)),
                        reading: false,
                    });
                }
            }
        }
        let due: Vec<u64> = lane
            .days
            .iter()
            .filter(|(_, day)| !day.reading && instant >= day.due)
            .map(|(day, _)| *day)
            .collect();
        for day in due {
            if let Some(state) = lane.days.get_mut(&day) {
                state.reading = true;
            }
            let source = source.clone();
            lane.spawn(async move { Answer::Seed(day, source.seed_state(day).await) });
        }
        if lane.forget_due.is_none_or(|due| instant >= due) {
            lane.forget_due = Some(instant + FORGET_EVERY);
            let _ = self.mailbox_holder.forget_ended(now, &terms);
        }
    }

    fn payout_answer(&mut self, answer: Answer, instant: Instant, now: u64) {
        let lane = &mut self.payouts;
        match answer {
            Answer::Terms(Ok(terms)) => {
                lane.terms = Some(terms);
                lane.terms_due = None;
                lane.draw_due = None;
            }
            Answer::Terms(Err(_)) => lane.terms_due = Some(instant + RETRY),
            Answer::Unit(unit, Ok(Some(found))) => {
                lane.unit = Some((unit, found));
                lane.unit_due = None;
                lane.account_due = None;
            }
            Answer::Unit(_, _) => lane.unit_due = Some(instant + RETRY),
            Answer::Account(gas, owed) => {
                lane.gas = gas.or(lane.gas);
                lane.owed = owed.or(lane.owed);
            }
            Answer::Seed(day, Ok(state)) => self.seed_read(day, state, instant, now),
            Answer::Seed(day, Err(_)) => {
                if let Some(day) = lane.days.get_mut(&day) {
                    day.reading = false;
                    day.due = instant + SEED_EVERY;
                }
            }
            Answer::Kept(day, outcome) => {
                if let Some(day) = lane.days.get_mut(&day) {
                    day.reading = false;
                    day.due = match outcome {
                        Ok(_) => instant + SEED_SOON,
                        Err(ChainError::NoGas) => instant + ACCOUNT_EVERY,
                        Err(_) => instant + SEED_EVERY,
                    };
                }
            }
            Answer::Book(book, _, Ok(Some(_))) => {
                lane.books.insert(book, BookCheck::Sold);
                lane.draw_due = None;
            }
            Answer::Book(book, valid_until, Ok(None)) => {
                // A lagging RPC may not show a purchase of today yet.
                let bought_on = lane
                    .draw_terms()
                    .map(|terms| terms.purchase_day(valid_until));
                let check = match bought_on {
                    Some(day) if now >= (day + 1) * PERIOD_SECONDS => BookCheck::Unsold,
                    _ => BookCheck::Retry(instant + SEED_EVERY),
                };
                lane.books.insert(book, check);
                lane.draw_due = None;
            }
            Answer::Book(book, _, Err(_)) => {
                lane.books
                    .insert(book, BookCheck::Retry(instant + SEED_EVERY));
            }
            Answer::Withdrawn(done) => self.withdrawn(done),
        }
    }

    /// A day's seed as read: draw by it once captured; otherwise, in this
    /// unit's turn, arm it or capture it past its target block.
    fn seed_read(&mut self, day: u64, state: SeedState, instant: Instant, now: u64) {
        let lane = &mut self.payouts;
        if let Some(seed) = state.seed {
            lane.days.remove(&day);
            lane.seeds.insert(day, seed);
            if let Some(terms) = lane.draw_terms() {
                let sold = lane.sold();
                let _ = self
                    .mailbox_holder
                    .draw_books(&terms, &lane.seeds, Some(&sold));
            }
            return;
        }
        let (Some(unit), Some((pool, _)), Some(source)) = (
            self.mailbox_holder.unit(),
            lane.terms,
            self.chain.source.clone(),
        ) else {
            if let Some(entry) = lane.days.get_mut(&day) {
                entry.reading = false;
                entry.due = instant + SEED_EVERY;
            }
            return;
        };
        let turn = (day + 1) * PERIOD_SECONDS + keeper_turn(&unit, day);
        let data = if state.target == 0 || state.head > state.target + SEED_WINDOW {
            (now >= turn).then(|| super::chain::arm_seed_calldata(day))
        } else if state.head > state.target {
            (now >= turn).then(|| super::chain::capture_seed_calldata(day))
        } else {
            None
        };
        let Some(entry) = lane.days.get_mut(&day) else {
            return;
        };
        match data {
            Some(data) => {
                let key = self.mailbox_holder.transaction_key();
                lane.spawn(async move {
                    Answer::Kept(day, source.transact(&key, pool.address, data).await)
                });
            }
            None => {
                entry.reading = false;
                entry.due = if now < turn {
                    after(instant, (turn - now).min(SEED_EVERY.as_secs()))
                } else {
                    instant + SEED_SOON
                };
            }
        }
    }

    fn withdrawn(&mut self, done: Withdrawn) {
        let prize = self.payouts.terms.map_or(0, |(pool, _)| pool.prize_usdc);
        let _ = self
            .mailbox_holder
            .settle_tickets(&done.claimed, TicketState::Claimed, prize);
        let _ = self
            .mailbox_holder
            .settle_tickets(&done.refused, TicketState::Refused, prize);
        let lane = &mut self.payouts;
        lane.owed = done.owed.or(lane.owed);
        if let Some(withdrawal) = &mut lane.withdrawal {
            withdrawal.state = if done.error.is_some() {
                "failed"
            } else {
                "done"
            };
            withdrawal.error = done.error;
            withdrawal.transactions = done.transactions;
        }
        lane.account_due = None;
    }

    fn pool_ready(&self) -> std::result::Result<Ready, (&'static str, String)> {
        let source = self
            .chain
            .source
            .clone()
            .filter(|source| source.has_pool())
            .ok_or_else(|| {
                (
                    "pool_not_configured",
                    "This node knows no operator pool: start it with --operator-pool".into(),
                )
            })?;
        let (pool, terms) = self
            .payouts
            .terms
            .zip(self.payouts.draw_terms())
            .map(|((pool, _), terms)| (pool, terms))
            .ok_or_else(|| ("pool_pending", "Reading the pool's terms; ask again".into()))?;
        let (_, unit) = self
            .payouts
            .unit
            .filter(|(held, _)| Some(*held) == self.mailbox_holder.unit())
            .ok_or_else(|| {
                (
                    "not_a_unit",
                    "This node holds as no registry unit the pool knows yet".into(),
                )
            })?;
        Ok((source, pool, terms, unit))
    }

    /// `operator_withdraw`: one claim of every won ticket still valid, the
    /// ones the pool refuses left out; with none won, a payout of what the
    /// pool owes. It runs in the background: `operator_earnings` shows how
    /// it ended.
    pub(super) fn operator_withdraw(
        &mut self,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        let (source, pool, terms, unit) = self.pool_ready()?;
        if self
            .payouts
            .withdrawal
            .as_ref()
            .is_some_and(|w| w.state == "sending")
        {
            return Err(("withdrawal_running", "A withdrawal is under way".into()));
        }
        let claims: Vec<TicketClaim> = self
            .mailbox_holder
            .tickets()
            .map_err(|refusal| ("storage", refusal.code().to_owned()))?
            .into_iter()
            .filter(|t| t.state == TicketState::Won && terms.ends_at(t.valid_until) > now)
            .map(|t| t.claim)
            .collect();
        let key = self.mailbox_holder.transaction_key();
        let transport = self.own_transport_key();
        let domain = NETWORK_DOMAIN;
        let owed = self.payouts.owed.unwrap_or(0);
        if claims.is_empty() && owed == 0 {
            return Err((
                "nothing_to_withdraw",
                "No won ticket and nothing owed".into(),
            ));
        }
        let answer = if claims.is_empty() {
            json!({ "owed": owed.to_string() })
        } else {
            json!({
                "tickets": claims.len(),
                "usdc": (pool.prize_usdc * claims.len() as u128).to_string(),
            })
        };
        self.payouts.withdrawal = Some(Withdrawal {
            state: "sending",
            tickets: claims.len(),
            error: None,
            transactions: vec![],
        });
        self.payouts.spawn(async move {
            let mut done = Withdrawn::default();
            if claims.is_empty() {
                let data = super::chain::pay_out_calldata(unit.index, &transport);
                match source.transact(&key, pool.address, data).await {
                    Ok(hash) => done.transactions.push(hash),
                    Err(error) => done.error = Some(error_code(&error)),
                }
            }
            'batches: for batch in claims.chunks(MAX_CLAIM) {
                let data = super::chain::claim_calldata(unit.index, &transport, batch);
                let batch = match source.transact(&key, pool.address, data).await {
                    Ok(hash) => {
                        done.transactions.push(hash);
                        done.claimed
                            .extend(batch.iter().map(|c| claim_slot(&domain, c)));
                        continue;
                    }
                    Err(ChainError::Reverted) => batch,
                    Err(error) => {
                        done.error = Some(error_code(&error));
                        break;
                    }
                };
                // The pool refuses one of them: find which, claim the rest.
                let mut good = Vec::new();
                for claim in batch {
                    let data = super::chain::claim_calldata(
                        unit.index,
                        &transport,
                        std::slice::from_ref(claim),
                    );
                    match source.check(key.account(), pool.address, data).await {
                        Ok(()) => good.push(claim.clone()),
                        Err(ChainError::Reverted) => done.refused.push(claim_slot(&domain, claim)),
                        Err(error) => {
                            done.error = Some(error_code(&error));
                            break 'batches;
                        }
                    }
                }
                if good.is_empty() {
                    continue;
                }
                let data = super::chain::claim_calldata(unit.index, &transport, &good);
                match source.transact(&key, pool.address, data).await {
                    Ok(hash) => {
                        done.transactions.push(hash);
                        done.claimed
                            .extend(good.iter().map(|c| claim_slot(&domain, c)));
                    }
                    Err(error) => {
                        done.error = Some(error_code(&error));
                        break;
                    }
                }
            }
            done.owed = source.owed(unit.index).await.ok();
            Answer::Withdrawn(done)
        });
        Ok(answer)
    }

    /// `operator_earnings`: the operator's prizes in USDC units and what
    /// the node needs to withdraw them.
    pub(super) fn operator_earnings(
        &mut self,
        now: u64,
    ) -> std::result::Result<Value, (&'static str, String)> {
        if !self
            .chain
            .source
            .as_ref()
            .is_some_and(|source| source.has_pool())
        {
            return Err((
                "pool_not_configured",
                "This node knows no operator pool: start it with --operator-pool".into(),
            ));
        }
        let terms = self.payouts.draw_terms();
        let prize = self.payouts.terms.map(|(pool, _)| pool.prize_usdc);
        let usdc = |tickets: u64| prize.map(|prize| (prize * u128::from(tickets)).to_string());
        let tickets = self
            .mailbox_holder
            .tickets()
            .map_err(|refusal| ("storage", refusal.code().to_owned()))?;
        let count = |state: TicketState| {
            tickets
                .iter()
                .filter(|t| {
                    t.state == state && terms.is_none_or(|terms| terms.ends_at(t.valid_until) > now)
                })
                .count() as u64
        };
        let won = count(TicketState::Won);
        let claimed = self.mailbox_holder.claimed();
        let held = self.mailbox_holder.held();
        let ending = terms.and_then(|terms| self.mailbox_holder.ending(now, &terms));
        Ok(json!({
            "unit": self.mailbox_holder.unit().map(hex::encode),
            "unitIndex": self.payouts.unit.map(|(_, unit)| unit.index),
            "account": hex0x(self.mailbox_holder.account()),
            "gasWei": self.payouts.gas.map(|gas| gas.to_string()),
            "prizeUsdc": prize.map(|prize| prize.to_string()),
            "won": {"tickets": won, "usdc": usdc(won)},
            "drawing": {"tickets": count(TicketState::Drawing)},
            "claimed": {"tickets": claimed.tickets, "usdc": claimed.usdc.to_string()},
            "refused": {"tickets": count(TicketState::Refused)},
            "owed": self.payouts.owed.map(|owed| owed.to_string()),
            "ending": ending.map(|e| json!({
                "tickets": e.tickets,
                "usdc": usdc(e.tickets),
                "inDays": e.in_days,
            })),
            "held": {"messages": held.messages, "paid": held.paid},
            "withdrawal": self.payouts.withdrawal.as_ref().map(|w| json!({
                "state": w.state,
                "tickets": w.tickets,
                "error": w.error,
                "transactions": w.transactions.iter().map(hex0x).collect::<Vec<_>>(),
            })),
        }))
    }

    /// Diagnostics for `node_info`.
    pub(super) fn payouts_info(&self) -> Value {
        json!({
            "configured": self.chain.source.as_ref().is_some_and(|s| s.has_pool()),
            "terms": self.payouts.terms.is_some(),
            "unitIndex": self.payouts.unit.map(|(_, unit)| unit.index),
            "seeds": self.payouts.seeds.len(),
            "waitingDays": self.payouts.days.keys().collect::<Vec<_>>(),
        })
    }
}
