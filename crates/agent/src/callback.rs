// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
// SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call>

//! Outbound callback routing: idle control WS → `place_call` → session tracking.
//!
//! Phone idle control client (`AgentControlClient`) sends hello with `mode=CONTROL_IDLE`.
//! Fulfillment (`ReplyMethod::Phone`) pushes `callback_requested`, then dispatches
//! `place_call` only when: user confirmed + number on `callback_whitelist` + idle
//! control client connected (never auto-dials; empty whitelist = fail-closed).

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::tasks::TaskStore;

pub type WsSink = Arc<Mutex<SplitSink<WebSocket, Message>>>;

/// Outbound callback session phases (agent-side state machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackPhase {
    /// Task queued; `place_call` not yet sent.
    Queued,
    /// `place_call` sent; waiting for phone dial / OFFHOOK.
    Dialing,
    /// Remote party ringing (optional telemetry from phone).
    Ringing,
    /// OFFHOOK + uplink path ready (`outbound_ready` from phone).
    Connected,
    /// Dial or bridge failed.
    Failed,
    /// Call ended normally.
    Done,
}

impl CallbackPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Dialing => "dialing",
            Self::Ringing => "ringing",
            Self::Connected => "connected",
            Self::Failed => "failed",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallbackSession {
    pub task_id: String,
    pub number: String,
    pub call_id: String,
    pub serial: String,
    pub phase: CallbackPhase,
    /// The SIM (Android subscription id) of the call the task came from, when the
    /// call-back goes out on that same phone.
    pub sub_id: Option<i64>,
}

impl CallbackSession {
    pub fn new(task_id: impl Into<String>, number: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            number: number.into(),
            call_id: String::new(),
            serial: String::new(),
            phase: CallbackPhase::Queued,
            sub_id: None,
        }
    }

    pub fn advance(&mut self, phase: CallbackPhase) {
        self.phase = phase;
    }
}

/// Build the WS JSON command payload for the phone to place an outbound callback.
///
/// Returns `{"type":"place_call","number":"…","call_id":"…","task_id":"…"}`, plus
/// `"sub_id"` when the SIM the call came in on is known (older apps ignore it and
/// dial on their default SIM).
pub fn start_callback_request(session: &mut CallbackSession, call_id: Option<&str>) -> Value {
    let call_id = call_id
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    session.call_id = call_id.clone();
    session.advance(CallbackPhase::Dialing);
    let mut payload = json!({
        "type": "place_call",
        "number": session.number,
        "call_id": call_id,
        "task_id": session.task_id,
    });
    if let Some(sub) = session.sub_id {
        payload["sub_id"] = json!(sub);
    }
    payload
}

/// Phone acknowledged dial intent (`place_call_ack` or `place_call_failed`).
pub fn on_place_call_result(session: &mut CallbackSession, ok: bool) {
    session.advance(if ok {
        CallbackPhase::Dialing
    } else {
        CallbackPhase::Failed
    });
}

/// Phone reached OFFHOOK and reused the uplink inject path (`outbound_ready`).
pub fn on_outbound_ready(session: &mut CallbackSession) {
    session.advance(CallbackPhase::Connected);
}

/// Cellular session ended (`call_end` or WS drop while Connected).
pub fn on_call_end(session: &mut CallbackSession, failed: bool) {
    session.advance(if failed {
        CallbackPhase::Failed
    } else {
        CallbackPhase::Done
    });
}

/// Tasks whose call's SIM is remembered for a call-back: the newest this many.
const RECENT_MAX: usize = 256;

/// A map that keeps only its newest [`RECENT_MAX`] keys.
#[derive(Debug)]
struct Recent<V> {
    entries: HashMap<String, V>,
    order: VecDeque<String>,
}

impl<V> Default for Recent<V> {
    fn default() -> Self {
        Self { entries: HashMap::new(), order: VecDeque::new() }
    }
}

impl<V> Recent<V> {
    fn get(&self, key: &str) -> Option<&V> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: &str, value: V) {
        if self.entries.insert(key.to_string(), value).is_none() {
            self.order.push_back(key.to_string());
        }
        while self.order.len() > RECENT_MAX {
            if let Some(old) = self.order.pop_front() {
                self.entries.remove(&old);
            }
        }
    }
}

/// The control serials a call's phone may be registered under: its own, and the
/// main app's for a work-profile twin copy (`pixel-101NNNN` → `pixel-1NNNN`).
fn control_serials(serial: &str) -> Vec<String> {
    let s = serial.trim();
    let mut out = vec![s.to_string()];
    if crate::calls::is_twin_serial(s) {
        if let Some(app) = s.strip_prefix("pixel-").and_then(|r| r.get(2..)) {
            out.push(format!("pixel-{app}"));
        }
    }
    out
}

const CALLBACK_SESSIONS_MAX: usize = 512;

/// Connected idle-control WebSocket clients and in-flight callback sessions.
#[derive(Clone, Default)]
pub struct ControlRegistry {
    clients: Arc<Mutex<HashMap<String, WsSink>>>,
    sessions: Arc<Mutex<HashMap<String, CallbackSession>>>,
    call_index: Arc<Mutex<HashMap<String, String>>>,
    session_order: Arc<Mutex<VecDeque<String>>>,
    /// Phones switched to "normal phone" (their owner answers): call-backs skip them.
    normal_mode: Arc<Mutex<HashSet<String>>>,
    /// Phones whose app can refuse a ringing call (`supports_reject` in the
    /// control hello); older apps pick up whatever `answer_plan` says.
    reject_capable: Arc<Mutex<HashSet<String>>>,
    /// The phone and SIM (subscription id) of the call each recent task came
    /// from, so its call-back dials out on the SIM the caller called.
    task_sims: Arc<Mutex<Recent<(String, i64)>>>,
    /// SIM book (from `errands`) so call-back phone selection knows each phone's
    /// SIM countries.
    sim_book: Arc<std::sync::Mutex<Option<Arc<crate::errands::SimBook>>>>,
    /// Per-phone SIM countries (ISO 3166 alpha-2) registered directly (e.g. tests).
    phone_sims: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl ControlRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Attach the shared [`crate::errands::SimBook`] so call-backs can prefer a
    /// phone with a SIM in the destination number's country.
    pub fn set_sim_book(&self, book: Arc<crate::errands::SimBook>) {
        *self.sim_book.lock().unwrap_or_else(|e| e.into_inner()) = Some(book);
    }

    /// Record the SIM countries (`iso` codes) available on `serial` (used when
    /// no [`crate::errands::SimBook`] entry exists yet).
    pub async fn set_phone_sim_countries(&self, serial: &str, countries: Vec<String>) {
        let mut clean: Vec<String> = countries
            .into_iter()
            .map(|c| c.trim().to_ascii_lowercase())
            .filter(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_lowercase()))
            .collect();
        clean.sort();
        clean.dedup();
        self.phone_sims
            .lock()
            .await
            .insert(serial.trim().to_string(), clean);
    }

    async fn sim_countries_for(&self, serial: &str) -> Vec<String> {
        let canonical = crate::canonical_device_serial(serial);
        let mut out: Vec<String> = Vec::new();
        if let Some(book) = self.sim_book.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            for (_, c) in book.of(&canonical) {
                let c = c.trim().to_ascii_lowercase();
                if !c.is_empty() && !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        if out.is_empty() {
            let map = self.phone_sims.lock().await;
            for key in [serial.trim(), canonical.as_str()] {
                if let Some(list) = map.get(key) {
                    for c in list {
                        if !c.is_empty() && !out.contains(c) {
                            out.push(c.clone());
                        }
                    }
                }
            }
        }
        if out.is_empty() {
            if let Some(last) = crate::phone_region::last_sim(&canonical) {
                out.push(last);
            }
        }
        out
    }

    /// Record whether `serial` is a normal phone (from its control hello and status).
    pub async fn set_normal_mode(&self, serial: &str, normal: bool) {
        let mut set = self.normal_mode.lock().await;
        if normal {
            set.insert(serial.to_string());
        } else {
            set.remove(serial);
        }
    }

    pub async fn register_client(&self, serial: String, sink: WsSink) {
        let mut clients = self.clients.lock().await;
        if let Some(prev) = clients.insert(serial.clone(), sink) {
            // Drop stale sink for same serial (reconnect).
            let _ = prev;
        }
        info!(
            serial,
            count = clients.len(),
            "control_idle client registered"
        );
    }

    pub async fn unregister_client(&self, serial: &str) {
        let mut clients = self.clients.lock().await;
        clients.remove(serial);
        info!(
            serial,
            count = clients.len(),
            "control_idle client unregistered"
        );
        drop(clients);
        self.forget_phone(serial).await;
    }

    /// Remove `serial` only if it still maps to `sink` (reconnect-safe).
    pub async fn unregister_client_sink(&self, serial: &str, sink: &WsSink) {
        let mut clients = self.clients.lock().await;
        let ours = clients
            .get(serial)
            .map(|cur| Arc::ptr_eq(cur, sink))
            .unwrap_or(false);
        if ours {
            clients.remove(serial);
            info!(serial, count = clients.len(), "control_idle client unregistered");
            drop(clients);
            self.forget_phone(serial).await;
        } else {
            info!(
                serial,
                count = clients.len(),
                "control_idle stale socket closed (newer client kept)"
            );
        }
    }

    /// Record whether the phone's app can refuse a ringing call (from its control hello).
    pub async fn set_supports_reject(&self, serial: &str, on: bool) {
        let mut set = self.reject_capable.lock().await;
        if on {
            set.insert(serial.to_string());
        } else {
            set.remove(serial);
        }
    }

    /// The phone's app said it can refuse a ringing call (`answer_plan` `reject`).
    pub async fn supports_reject(&self, serial: &str) -> bool {
        let set = self.reject_capable.lock().await;
        control_serials(serial).iter().any(|s| set.contains(s))
    }

    /// A phone went away: its abilities are forgotten until its next hello.
    async fn forget_phone(&self, serial: &str) {
        self.reject_capable.lock().await.remove(serial);
    }

    /// The call task `task_id` came from was taken by `serial` on the SIM
    /// `sub_id`: a call-back from that phone dials out on that SIM.
    pub async fn remember_task_sim(&self, task_id: &str, serial: &str, sub_id: i64) {
        if task_id.trim().is_empty() {
            return;
        }
        self.task_sims
            .lock()
            .await
            .insert(task_id, (serial.trim().to_string(), sub_id));
    }

    /// The SIM for `task_id`'s call-back on `dialing_serial`: the one its call came
    /// in on, when the call-back goes out on that same phone.
    async fn task_sim_on(&self, task_id: &str, dialing_serial: &str) -> Option<i64> {
        let sims = self.task_sims.lock().await;
        let (serial, sub) = sims.get(task_id)?;
        control_serials(serial)
            .iter()
            .any(|s| s.eq_ignore_ascii_case(dialing_serial.trim()))
            .then_some(*sub)
    }

    /// Idle phones the owner actually has (work-profile twin copies excluded), sorted.
    pub async fn connected_serials(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .clients
            .lock()
            .await
            .keys()
            .filter(|s| !crate::calls::is_twin_serial(s))
            .cloned()
            .collect();
        v.sort();
        v
    }

    /// Push current blocklist to one idle control client (after hello or reconnect).
    pub async fn send_blocklist_sync(&self, serial: &str, numbers: &[String]) -> Result<()> {
        let clients = self.clients.lock().await;
        let Some(sink) = clients.get(serial) else {
            return Ok(());
        };
        let sink = sink.clone();
        drop(clients);
        send_blocklist_payload(&sink, numbers).await
    }

    /// Push blocklist to all connected idle control clients. Returns send count.
    pub async fn broadcast_blocklist_sync(&self, numbers: &[String]) -> usize {
        let clients = self.clients.lock().await;
        if clients.is_empty() {
            return 0;
        }
        let sinks: Vec<(String, WsSink)> = clients
            .iter()
            .map(|(s, sink)| (s.clone(), sink.clone()))
            .collect();
        drop(clients);

        let payload = blocklist_sync_payload(numbers);
        let text = match serde_json::to_string(&payload) {
            Ok(t) => t,
            Err(e) => {
                warn!("blocklist_sync serialize failed: {e}");
                return 0;
            }
        };
        let mut sent = 0usize;
        for (serial, sink) in sinks {
            if sink
                .lock()
                .await
                .send(Message::Text(text.clone().into()))
                .await
                .is_ok()
            {
                sent += 1;
            } else {
                warn!(serial, "blocklist_sync send failed");
            }
        }
        if sent > 0 {
            info!(
                sent,
                numbers_len = numbers.len(),
                "blocklist_sync broadcast"
            );
        }
        sent
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// The call-back session of `task_id` (empty: the task of `call_id`, the
    /// call the agent asked a phone to place), as it stands now. Read-only.
    pub async fn callback_session_for(&self, task_id: &str, call_id: &str) -> Option<CallbackSession> {
        let task_id = match task_id.trim() {
            "" => self.call_index.lock().await.get(call_id.trim())?.clone(),
            t => t.to_string(),
        };
        self.sessions.lock().await.get(&task_id).cloned()
    }

    /// Dispatch `place_call` to a connected control client. Returns `true` when sent.
    ///
    /// Gates (fail-closed):
    /// - `number` non-empty
    /// - `callback_whitelist_allows(number, whitelist)` — empty list denies
    /// - at least one `CONTROL_IDLE` client (`Ok(false)` when none)
    ///
    /// Prefers `prefer_serial` when that phone is connected; otherwise first available.
    pub async fn try_dispatch_phone_callback(
        &self,
        task_id: &str,
        number: &str,
        prefer_serial: &str,
        whitelist: &[String],
    ) -> Result<bool> {
        let number = number.trim();
        if number.is_empty() {
            anyhow::bail!("callback number empty");
        }
        if !callback_whitelist_allows(number, whitelist) {
            anyhow::bail!("callback number not on whitelist");
        }

        let serials: Vec<String> = {
            let clients = self.clients.lock().await;
            if clients.is_empty() {
                return Ok(false);
            }
            clients.keys().cloned().collect()
        };
        let mut sim_map: HashMap<String, Vec<String>> = HashMap::with_capacity(serials.len());
        for s in &serials {
            sim_map.insert(s.clone(), self.sim_countries_for(s).await);
        }
        let home = crate::phone_region::country_of_code(&crate::locale::country_code());
        let target_country = crate::phone_region::normalize(number, home).map(|n| n.country.to_string());
        let normal = self.normal_mode.lock().await.clone();
        let clients = self.clients.lock().await;
        if clients.is_empty() {
            return Ok(false);
        }
        let picked = pick_callback_phone(
            clients.keys(),
            prefer_serial,
            &normal,
            target_country.as_deref(),
            |serial, target| {
                sim_map
                    .get(serial)
                    .is_some_and(|list| list.iter().any(|c| c.eq_ignore_ascii_case(target)))
            },
        )
        .and_then(|serial| clients.get(&serial).map(|sink| (serial, sink.clone())));
        let Some((serial, sink)) = picked else {
            info!(task_id, "callback: every connected phone is a normal phone");
            return Ok(false);
        };
        drop(clients);

        let mut session = CallbackSession::new(task_id, number);
        session.serial = serial.clone();
        session.sub_id = self.task_sim_on(task_id, &serial).await;
        let sub_id = session.sub_id;
        let payload = start_callback_request(&mut session, None);

        send_ws_json(&sink, payload)
            .await
            .with_context(|| format!("send place_call serial={serial} task_id={task_id}"))?;

        let call_id = session.call_id.clone();
        self.register_callback_session(task_id, call_id, session).await;

        info!(
            task_id,
            serial,
            sub_id = ?sub_id,
            number_len = number.len(),
            "callback place_call dispatched"
        );
        Ok(true)
    }

    /// Dispatch `place_call` to exactly the phone `serial` (from the SIM
    /// `sub_id`; None: its default), never another one: an errand's phone and
    /// SIM were picked for the number's country. `Ok(false)` when that phone
    /// is not connected or is a normal phone.
    pub async fn try_dispatch_on(&self, task_id: &str, number: &str, serial: &str, sub_id: Option<i64>) -> Result<bool> {
        let number = number.trim();
        if number.is_empty() {
            anyhow::bail!("number empty");
        }
        let sink = {
            let clients = self.clients.lock().await;
            if self.normal_mode.lock().await.contains(serial) {
                info!(task_id, serial, "place_call: the phone is a normal phone");
                return Ok(false);
            }
            match clients.get(serial) {
                Some(sink) => sink.clone(),
                None => {
                    info!(task_id, serial, "place_call: the phone is not connected");
                    return Ok(false);
                }
            }
        };
        let mut session = CallbackSession::new(task_id, number);
        session.serial = serial.to_string();
        session.sub_id = sub_id;
        let payload = start_callback_request(&mut session, None);
        send_ws_json(&sink, payload)
            .await
            .with_context(|| format!("send place_call serial={serial} task_id={task_id}"))?;
        let call_id = session.call_id.clone();
        self.register_callback_session(task_id, call_id, session).await;
        info!(task_id, serial, sub_id = ?sub_id, number_len = number.len(), "place_call dispatched on its phone");
        Ok(true)
    }

    async fn register_callback_session(&self, task_id: &str, call_id: String, session: CallbackSession) {
        let mut sessions = self.sessions.lock().await;
        let mut call_index = self.call_index.lock().await;
        let mut order = self.session_order.lock().await;
        if let Some(prev) = sessions.insert(task_id.to_string(), session) {
            if !prev.call_id.is_empty() && prev.call_id != call_id {
                call_index.remove(&prev.call_id);
            }
            order.retain(|id| id != task_id);
        }
        order.push_back(task_id.to_string());
        call_index.insert(call_id, task_id.to_string());
        // Bound in-memory callback history by pruning the oldest finished sessions.
        if sessions.len() > CALLBACK_SESSIONS_MAX {
            let mut keep = VecDeque::with_capacity(order.len());
            while let Some(id) = order.pop_front() {
                let over = sessions.len() > CALLBACK_SESSIONS_MAX;
                let done = sessions
                    .get(&id)
                    .is_some_and(|s| matches!(s.phase, CallbackPhase::Done | CallbackPhase::Failed));
                if over && done {
                    if let Some(old) = sessions.remove(&id) {
                        if !old.call_id.is_empty() {
                            call_index.remove(&old.call_id);
                        }
                    }
                } else if sessions.contains_key(&id) {
                    keep.push_back(id);
                }
            }
            *order = keep;
        }
    }

    /// Advance session from phone → agent control WS telemetry.
    pub async fn handle_phone_event(&self, msg: &Value, store: &TaskStore) {
        let mtype = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let call_id = msg.get("call_id").and_then(|v| v.as_str()).unwrap_or("").trim();
        let task_id = msg
            .get("task_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let task_id = if let Some(tid) = task_id {
            tid
        } else if !call_id.is_empty() {
            match self.call_index.lock().await.get(call_id) {
                Some(tid) => tid.clone(),
                None => {
                    info!(mtype, call_id, "callback event: unknown call_id");
                    return;
                }
            }
        } else {
            info!(mtype, "callback event: missing task_id/call_id");
            return;
        };

        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&task_id) else {
            info!(mtype, task_id, "callback event: no session");
            return;
        };
        if !call_id.is_empty() && !session.call_id.is_empty() && session.call_id != call_id {
            info!(mtype, task_id, call_id, "callback event: stale call_id ignored");
            return;
        }
        let write_task = !task_id.starts_with("errand-");

        match mtype {
            "place_call_ack" => {
                on_place_call_result(session, true);
                drop(sessions);
                if write_task {
                    let _ = store.push_event(&task_id, "callback_place_call_ack").await;
                    let _ = store
                        .patch(
                            &task_id,
                            crate::tasks::TaskPatch {
                                fulfill_error: Some(None),
                                ..Default::default()
                            },
                        )
                        .await;
                }
                info!(task_id, "callback place_call_ack");
            }
            "place_call_failed" => {
                on_place_call_result(session, false);
                let reason = msg
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("dial failed");
                drop(sessions);
                if write_task {
                    let _ = store
                        .push_event(&task_id, "callback_place_call_failed")
                        .await;
                    let _ = store
                        .patch(
                            &task_id,
                            crate::tasks::TaskPatch {
                                fulfill_error: Some(Some(format!("callback: {reason}"))),
                                ..Default::default()
                            },
                        )
                        .await;
                }
                warn!(task_id, reason, "callback place_call_failed");
            }
            "outbound_ready" => {
                on_outbound_ready(session);
                drop(sessions);
                if write_task {
                    let _ = store.push_event(&task_id, "callback_outbound_ready").await;
                }
                info!(task_id, "callback outbound_ready");
            }
            "call_end" => {
                let failed = msg
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .is_some_and(|r| !r.is_empty() && r != "normal");
                on_call_end(session, failed);
                drop(sessions);
                if write_task {
                    let evt = if failed {
                        "callback_failed"
                    } else {
                        "callback_done"
                    };
                    let _ = store.push_event(&task_id, evt).await;
                }
                info!(task_id, failed, "callback call_end");
            }
            _ => {}
        }
    }
}

/// The phone for a call-back: skips normal-mode phones, prefers connected AI-mode
/// phones whose SIM country matches `target_country` (preferring `prefer` among
/// matching phones, else first by serial), and falls back to `prefer` / first by
/// serial when none is known to match. None: no phone can call.
fn pick_callback_phone<'a>(
    connected: impl Iterator<Item = &'a String>,
    prefer: &str,
    normal: &HashSet<String>,
    target_country: Option<&str>,
    has_sim: impl Fn(&str, &str) -> bool,
) -> Option<String> {
    let mut usable: Vec<&String> = connected.filter(|s| !normal.contains(s.as_str())).collect();
    if let Some(target) = target_country.map(str::trim).filter(|t| !t.is_empty()) {
        let mut matching: Vec<&String> = usable
            .iter()
            .copied()
            .filter(|s| has_sim(s.as_str(), target))
            .collect();
        if !matching.is_empty() {
            if matching.iter().any(|s| s.as_str() == prefer) {
                return Some(prefer.to_string());
            }
            matching.sort();
            return matching.first().map(|s| (*s).clone());
        }
    }
    if usable.iter().any(|s| s.as_str() == prefer) {
        return Some(prefer.to_string());
    }
    usable.sort();
    usable.first().map(|s| (*s).clone())
}

pub async fn send_ws_json(sink: &WsSink, payload: Value) -> Result<()> {
    let text = serde_json::to_string(&payload)?;
    sink.lock()
        .await
        .send(Message::Text(text.into()))
        .await
        .context("ws send")?;
    Ok(())
}

/// A real control connection for tests: the agent's end ([`WsSink`], as a
/// phone's hello registers it) and the phone's end, which reads what the agent sends.
#[cfg(test)]
pub(crate) mod test_socket {
    use super::*;
    use futures_util::StreamExt;
    use tokio::sync::oneshot;

    pub(crate) type PhoneEnd =
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

    pub(crate) async fn phone_socket() -> (WsSink, PhoneEnd) {
        use axum::extract::ws::WebSocketUpgrade;
        let (tx, rx) = oneshot::channel::<WsSink>();
        let tx = Arc::new(std::sync::Mutex::new(Some(tx)));
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(move |ws: WebSocketUpgrade| {
                let tx = tx.clone();
                async move {
                    ws.on_upgrade(move |socket| async move {
                        let (sink, mut stream) = socket.split();
                        if let Some(tx) = tx.lock().unwrap().take() {
                            let _ = tx.send(Arc::new(Mutex::new(sink)));
                        }
                        while let Some(Ok(_)) = stream.next().await {}
                    })
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let (phone, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/")).await.unwrap();
        (sink_of(rx).await, phone)
    }

    async fn sink_of(rx: oneshot::Receiver<WsSink>) -> WsSink {
        tokio::time::timeout(Duration::from_secs(5), rx).await.unwrap().unwrap()
    }

    /// The next JSON message the phone gets, or None when nothing comes within `wait`.
    pub(crate) async fn next_json(phone: &mut PhoneEnd, wait: Duration) -> Option<Value> {
        use tokio_tungstenite::tungstenite::Message as WsMessage;
        loop {
            match tokio::time::timeout(wait, phone.next()).await {
                Ok(Some(Ok(WsMessage::Text(t)))) => return serde_json::from_str(t.as_str()).ok(),
                Ok(Some(Ok(_))) => continue,
                _ => return None,
            }
        }
    }
}

fn blocklist_sync_payload(numbers: &[String]) -> Value {
    json!({
        "type": "blocklist_sync",
        "numbers": numbers,
    })
}

async fn send_blocklist_payload(sink: &WsSink, numbers: &[String]) -> Result<()> {
    send_ws_json(sink, blocklist_sync_payload(numbers)).await
}

/// Resolve E.164 / caller-id number for outbound callback.
pub fn callback_number(reply_contact: Option<&str>, caller_id: &str) -> String {
    reply_contact
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let c = caller_id.trim();
            if c.is_empty() {
                None
            } else {
                Some(c)
            }
        })
        .unwrap_or("")
        .to_string()
}

/// Digits-only normalize for whitelist compare.
pub fn normalize_callback_digits(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn strip_tw_mobile_core(digits: &str) -> String {
    let d = digits.trim_start_matches('0');
    if let Some(rest) = d.strip_prefix("886") {
        rest.trim_start_matches('0').to_string()
    } else {
        d.to_string()
    }
}

/// Minimum normalized digit length required for a suffix match between two
/// numbers so short codes like `"123"` cannot collide with full numbers.
const MIN_EQUIV_SUFFIX_DIGITS: usize = 7;

fn numbers_equivalent(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let ca = strip_tw_mobile_core(a);
    let cb = strip_tw_mobile_core(b);
    !ca.is_empty()
        && !cb.is_empty()
        && (ca == cb
            || (ca.len().min(cb.len()) >= MIN_EQUIV_SUFFIX_DIGITS
                && (ca.ends_with(&cb) || cb.ends_with(&ca))))
}

/// Outbound dial allow-list. **Fail-closed:** empty `whitelist` denies every number.
///
/// Entries ending in `*` are digit-prefix matches (after normalize). Other entries
/// use digit-exact / TW-mobile-equivalent match (`09…` ↔ `8869…`).
pub fn callback_whitelist_allows(number: &str, whitelist: &[String]) -> bool {
    if whitelist.is_empty() {
        return false;
    }
    let num = normalize_callback_digits(number);
    if num.is_empty() {
        return false;
    }
    for entry in whitelist {
        let e = entry.trim();
        if e.is_empty() {
            continue;
        }
        let prefix = e.ends_with('*');
        let pat_raw = if prefix {
            e.trim_end_matches('*').trim()
        } else {
            e
        };
        let pat = normalize_callback_digits(pat_raw);
        if pat.is_empty() {
            continue;
        }
        if prefix {
            if num.starts_with(&pat)
                || strip_tw_mobile_core(&num).starts_with(&strip_tw_mobile_core(&pat))
            {
                return true;
            }
        } else if numbers_equivalent(&num, &pat) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_skips_normal_phones() {
        let phones = vec!["pixel-1".to_string(), "pixel-2".to_string(), "pixel-3".to_string()];
        let normal: HashSet<String> = ["pixel-2".to_string()].into_iter().collect();
        assert_eq!(pick_callback_phone(phones.iter(), "pixel-3", &normal, None, |_, _| false).as_deref(), Some("pixel-3"));
        // The preferred phone is a normal phone: another one calls.
        assert_eq!(pick_callback_phone(phones.iter(), "pixel-2", &normal, None, |_, _| false).as_deref(), Some("pixel-1"));
        let all: HashSet<String> = phones.iter().cloned().collect();
        assert_eq!(pick_callback_phone(phones.iter(), "pixel-1", &all, None, |_, _| false), None);
        assert_eq!(pick_callback_phone(std::iter::empty::<&String>(), "", &normal, None, |_, _| false), None);
    }

    #[test]
    fn pick_callback_phone_prefers_matching_sim_country() {
        let phones = vec![
            "pixel-1".to_string(),
            "pixel-2".to_string(),
            "pixel-3".to_string(),
            "pixel-4".to_string(),
        ];
        let normal: HashSet<String> = ["pixel-4".to_string()].into_iter().collect();
        let has_sim = |serial: &str, country: &str| match serial {
            "pixel-1" => country == "us",
            "pixel-2" => country == "tw",
            "pixel-3" => country == "jp",
            "pixel-4" => country == "tw", // normal phone must stay skipped
            _ => false,
        };
        // Even when prefer is pixel-1 (US), a TW destination prefers pixel-2 (TW).
        assert_eq!(
            pick_callback_phone(phones.iter(), "pixel-1", &normal, Some("tw"), has_sim).as_deref(),
            Some("pixel-2")
        );
        // JP destination picks pixel-3 (JP).
        assert_eq!(
            pick_callback_phone(phones.iter(), "pixel-1", &normal, Some("jp"), has_sim).as_deref(),
            Some("pixel-3")
        );
        // Unknown target country (or no matching SIM) falls back to prefer, then sorted usable.
        assert_eq!(
            pick_callback_phone(phones.iter(), "pixel-1", &normal, Some("de"), has_sim).as_deref(),
            Some("pixel-1")
        );
        assert_eq!(
            pick_callback_phone(phones.iter(), "pixel-4", &normal, Some("de"), has_sim).as_deref(),
            Some("pixel-1")
        );
    }

    #[tokio::test]
    async fn try_dispatch_switches_phone_by_sim_and_drops_old_sub_id() {
        let reg = ControlRegistry::default();
        let (s1, mut phone1) = test_socket::phone_socket().await;
        let (s2, mut phone2) = test_socket::phone_socket().await;
        reg.register_client("pixel-us-1".into(), s1).await;
        reg.register_client("pixel-tw-2".into(), s2).await;
        reg.set_phone_sim_countries("pixel-us-1", vec!["us".into()]).await;
        reg.set_phone_sim_countries("pixel-tw-2", vec!["tw".into()]).await;
        reg.remember_task_sim("task-tw", "pixel-us-1", 7).await;
        reg.remember_task_sim("task-us", "pixel-us-1", 7).await;

        let wl = vec!["+886912345678".to_string(), "+12025550143".to_string()];
        assert!(reg
            .try_dispatch_phone_callback("task-tw", "+886912345678", "pixel-us-1", &wl)
            .await
            .unwrap());
        let msg2 = test_socket::next_json(&mut phone2, Duration::from_secs(2))
            .await
            .expect("tw phone should receive place_call");
        assert_eq!(msg2["task_id"], "task-tw");
        assert!(msg2.get("sub_id").is_none(), "sub_id from pixel-us-1 must not be reused on pixel-tw-2");
        assert!(test_socket::next_json(&mut phone1, Duration::from_millis(80)).await.is_none());

        // Same phone as original: sub_id IS preserved.
        assert!(reg
            .try_dispatch_phone_callback("task-us", "+12025550143", "pixel-us-1", &wl)
            .await
            .unwrap());
        let msg1 = test_socket::next_json(&mut phone1, Duration::from_secs(2))
            .await
            .expect("us phone should receive place_call");
        assert_eq!(msg1["task_id"], "task-us");
        assert_eq!(msg1["sub_id"], 7);
    }

    #[test]
    fn numbers_equivalent_requires_min_suffix_len() {
        assert!(!numbers_equivalent("0912345123", "123"));
        assert!(!numbers_equivalent("123", "0912345123"));
        assert!(!numbers_equivalent("+886912345911", "911"));
        assert!(!numbers_equivalent("0912123456", "123456"));
        assert!(!callback_whitelist_allows("0912345123", &["123".to_string()]));
        // Exact short code still matches itself.
        assert!(numbers_equivalent("123", "123"));
        // Full numbers with >=7 core digits match across formats.
        assert!(numbers_equivalent("0912345678", "886912345678"));
        assert!(numbers_equivalent("2025550143", "12025550143"));
        assert!(numbers_equivalent("0312345678", "81312345678"));
    }

    #[test]
    fn start_callback_request_payload() {
        let mut s = CallbackSession::new("task-1", "+15551234567");
        let v = start_callback_request(&mut s, Some("call-abc"));
        assert_eq!(v["type"], "place_call");
        assert_eq!(v["number"], "+15551234567");
        assert_eq!(v["call_id"], "call-abc");
        assert_eq!(v["task_id"], "task-1");
        assert_eq!(s.phase, CallbackPhase::Dialing);
        assert!(v.get("sub_id").is_none());
        // The SIM of the call the task came from goes with the dial command.
        let mut s = CallbackSession::new("task-2", "0912345678");
        s.sub_id = Some(3);
        assert_eq!(start_callback_request(&mut s, None)["sub_id"], 3);
    }

    #[tokio::test]
    async fn call_back_uses_the_calls_sim_only_on_the_same_phone() {
        let reg = ControlRegistry::default();
        // The call came in on the work-profile twin; its control connection is the main app's.
        reg.remember_task_sim("task-1", "pixel-1010341", 4).await;
        assert_eq!(reg.task_sim_on("task-1", "pixel-10341").await, Some(4));
        assert_eq!(reg.task_sim_on("task-1", "pixel-10342").await, None);
        assert_eq!(reg.task_sim_on("task-2", "pixel-10341").await, None);
        // Only the newest tasks are kept.
        for i in 0..RECENT_MAX {
            reg.remember_task_sim(&format!("t{i}"), "pixel-10341", 1).await;
        }
        assert_eq!(reg.task_sim_on("task-1", "pixel-10341").await, None);
        assert_eq!(reg.task_sim_on("t0", "pixel-10341").await, Some(1));
    }

    #[test]
    fn callback_number_prefers_reply_contact() {
        assert_eq!(callback_number(Some("+886911"), "+886900"), "+886911");
        assert_eq!(callback_number(None, "+886900"), "+886900");
        assert_eq!(callback_number(Some(""), ""), "");
    }

    #[test]
    fn whitelist_fail_closed_empty() {
        assert!(!callback_whitelist_allows("+886911222333", &[]));
        assert!(!callback_whitelist_allows("0911222333", &[String::new()]));
    }

    #[test]
    fn whitelist_exact_and_tw_equiv() {
        let list = vec!["+886 911-222-333".to_string()];
        assert!(callback_whitelist_allows("0911222333", &list));
        assert!(callback_whitelist_allows("+886911222333", &list));
        assert!(!callback_whitelist_allows("0911222334", &list));
    }

    #[test]
    fn whitelist_prefix_star() {
        let list = vec!["+8869*".to_string()];
        assert!(callback_whitelist_allows("0911222333", &list));
        assert!(!callback_whitelist_allows("0212345678", &list));
    }

    #[test]
    fn a_twin_copy_reaches_the_main_apps_connection() {
        assert_eq!(control_serials("pixel-1010341"), vec!["pixel-1010341", "pixel-10341"]);
        assert_eq!(control_serials("pixel-10341"), vec!["pixel-10341"]);
    }

    #[tokio::test]
    async fn only_apps_that_say_so_can_decline() {
        let reg = ControlRegistry::default();
        assert!(!reg.supports_reject("pixel-10341").await);
        reg.set_supports_reject("pixel-10341", true).await;
        assert!(reg.supports_reject("pixel-10341").await);
        // The work-profile twin's call rings on the main app's connection.
        assert!(reg.supports_reject("pixel-1010341").await);
        // A reconnect with an older app (no field) takes it back; so does a drop.
        reg.set_supports_reject("pixel-10341", false).await;
        assert!(!reg.supports_reject("pixel-10341").await);
        reg.set_supports_reject("pixel-10341", true).await;
        reg.unregister_client("pixel-10341").await;
        assert!(!reg.supports_reject("pixel-10341").await);
    }

    #[tokio::test]
    async fn stale_call_id_and_errand_events_do_not_clobber_session_or_tasks() {
        let reg = ControlRegistry::default();
        let root = std::env::temp_dir().join(format!("bageo-cb-stale-{}", Uuid::new_v4()));
        let tasks = TaskStore::new(root.clone());

        let mut s1 = CallbackSession::new("errand-1", "0912345678");
        s1.call_id = "call-old".into();
        s1.phase = CallbackPhase::Dialing;
        reg.register_callback_session("errand-1", "call-old".into(), s1).await;

        let mut s2 = CallbackSession::new("errand-1", "0912345678");
        s2.call_id = "call-new".into();
        s2.phase = CallbackPhase::Dialing;
        reg.register_callback_session("errand-1", "call-new".into(), s2).await;

        assert!(reg.call_index.lock().await.get("call-old").is_none());
        // Late call_end from call-old must NOT mark call-new Done/Failed.
        reg.handle_phone_event(&json!({"type": "call_end", "task_id": "errand-1", "call_id": "call-old"}), &tasks).await;
        assert_eq!(reg.callback_session_for("errand-1", "").await.unwrap().phase, CallbackPhase::Dialing);

        let _ = std::fs::remove_dir_all(root);
    }
}
