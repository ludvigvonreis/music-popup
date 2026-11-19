use slotmap::{SlotMap, new_key_type};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::VecDeque,
    marker::PhantomData,
    rc::Rc,
};

new_key_type! {
    pub struct SignalId;
    pub struct EffectId;
    pub struct ScopeId;
}

struct SignalSlot {
    value: Rc<RefCell<dyn Any>>,
    subs: Vec<EffectId>,
}

struct EffectSlot {
    f: Rc<RefCell<dyn FnMut()>>,
    deps: Vec<SignalId>,
    queued: bool,
}

#[derive(Default)]
struct ScopeSlot {
    parent: Option<ScopeId>,
    children: Vec<ScopeId>,
    signals: Vec<SignalId>,
    effects: Vec<EffectId>,
}

#[derive(Default)]
struct Runtime {
    signals: RefCell<SlotMap<SignalId, SignalSlot>>,
    effects: RefCell<SlotMap<EffectId, EffectSlot>>,
    scopes: RefCell<SlotMap<ScopeId, ScopeSlot>>,
    /// Effect currently collecting dependencies.
    observer: Cell<Option<EffectId>>,
    /// Scope that newly created signals/effects/scopes attach to.
    scope: Cell<Option<ScopeId>>,
    pending: RefCell<VecDeque<EffectId>>,
    batch_depth: Cell<u32>,
    flushing: Cell<bool>,
}

thread_local! {
    static RT: Runtime = Runtime::default();
}

// ---------------------------------------------------------------------------
// Signals
// ---------------------------------------------------------------------------

pub struct RwSignal<T> {
    id: SignalId,
    _p: PhantomData<(fn() -> T, *const ())>,
}

impl<T> Clone for RwSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for RwSignal<T> {}

pub fn create_signal<T: 'static>(value: T) -> RwSignal<T> {
    RT.with(|rt| {
        let id = rt.signals.borrow_mut().insert(SignalSlot {
            value: Rc::new(RefCell::new(value)),
            subs: Vec::new(),
        });
        register(rt, |s| s.signals.push(id));
        RwSignal {
            id,
            _p: PhantomData,
        }
    })
}

impl<T: 'static> RwSignal<T> {
    fn value_rc(&self) -> Rc<RefCell<dyn Any>> {
        RT.with(|rt| {
            rt.signals
                .borrow()
                .get(self.id)
                .map(|s| s.value.clone())
                .expect("signal used after its scope was disposed")
        })
    }

    /// Read without subscribing the current effect.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let rc = self.value_rc();
        let b = rc.borrow();
        f(b.downcast_ref::<T>().expect("signal type mismatch"))
    }

    /// Read and subscribe the current effect (if any).
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        track(self.id);
        self.with_untracked(f)
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(|v| v.clone())
    }

    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.with_untracked(|v| v.clone())
    }

    /// Mutate in place and notify subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let rc = self.value_rc();
            let mut b = rc.borrow_mut();
            f(b.downcast_mut::<T>().expect("signal type mismatch"));
        }
        notify(self.id);
    }

    pub fn set(&self, value: T) {
        self.update(|v| *v = value);
    }

    /// Like `set`, but does nothing (and wakes nobody) if the value is equal.
    pub fn set_if_changed(&self, value: T)
    where
        T: PartialEq,
    {
        if !self.with_untracked(|cur| *cur == value) {
            self.set(value);
        }
    }

    pub fn is_alive(&self) -> bool {
        RT.with(|rt| rt.signals.borrow().contains_key(self.id))
    }
}

fn track(sig: SignalId) {
    RT.with(|rt| {
        let Some(e) = rt.observer.get() else { return };
        let mut effects = rt.effects.borrow_mut();
        let Some(eff) = effects.get_mut(e) else {
            return;
        };
        let mut sigs = rt.signals.borrow_mut();
        if let Some(s) = sigs.get_mut(sig) {
            if !s.subs.contains(&e) {
                s.subs.push(e);
                eff.deps.push(sig);
            }
        }
    });
}

fn notify(sig: SignalId) {
    RT.with(|rt| {
        let subs = rt
            .signals
            .borrow()
            .get(sig)
            .map(|s| s.subs.clone())
            .unwrap_or_default();
        {
            let mut effects = rt.effects.borrow_mut();
            let mut pending = rt.pending.borrow_mut();
            for e in subs {
                if let Some(eff) = effects.get_mut(e) {
                    if !eff.queued {
                        eff.queued = true;
                        pending.push_back(e);
                    }
                }
            }
        }
        if rt.batch_depth.get() == 0 {
            flush(rt);
        }
    });
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

/// Runs `f` immediately, then again whenever a signal it read changes.
/// Dependencies are re-collected on every run, so conditional reads work.
pub fn create_effect(f: impl FnMut() + 'static) -> EffectId {
    RT.with(|rt| {
        let id = rt.effects.borrow_mut().insert(EffectSlot {
            f: Rc::new(RefCell::new(f)),
            deps: Vec::new(),
            queued: false,
        });
        register(rt, |s| s.effects.push(id));
        run_effect(rt, id);
        id
    })
}

fn run_effect(rt: &Runtime, id: EffectId) {
    let (f, deps) = {
        let mut effects = rt.effects.borrow_mut();
        let Some(e) = effects.get_mut(id) else { return };
        e.queued = false;
        (e.f.clone(), std::mem::take(&mut e.deps))
    };
    {
        let mut sigs = rt.signals.borrow_mut();
        for d in deps {
            if let Some(s) = sigs.get_mut(d) {
                s.subs.retain(|x| *x != id);
            }
        }
    }
    let prev = rt.observer.replace(Some(id));
    (f.borrow_mut())();
    rt.observer.set(prev);
}

fn flush(rt: &Runtime) {
    if rt.flushing.replace(true) {
        return; // already flushing: `set` inside an effect just enqueues
    }
    let mut runs = 0u32;
    loop {
        let next = rt.pending.borrow_mut().pop_front();
        let Some(id) = next else { break };
        runs += 1;
        debug_assert!(
            runs < 100_000,
            "reactive loop: an effect keeps re-triggering itself"
        );
        run_effect(rt, id);
    }
    rt.flushing.set(false);
}

/// Defer effect execution until `f` returns, so several `set`s cause one pass.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    RT.with(|rt| rt.batch_depth.set(rt.batch_depth.get() + 1));
    let r = f();
    RT.with(|rt| {
        let d = rt.batch_depth.get() - 1;
        rt.batch_depth.set(d);
        if d == 0 {
            flush(rt);
        }
    });
    r
}

/// Run `f` without subscribing the current effect to anything it reads.
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    let prev = RT.with(|rt| rt.observer.replace(None));
    let r = f();
    RT.with(|rt| rt.observer.set(prev));
    r
}

// ---------------------------------------------------------------------------
// Memo
// ---------------------------------------------------------------------------

pub struct Memo<T>(RwSignal<Option<T>>);

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Memo<T> {}

impl<T: Clone + 'static> Memo<T> {
    pub fn get(&self) -> T {
        self.0.with(|v| v.clone().expect("memo not initialised"))
    }
    pub fn get_untracked(&self) -> T {
        self.0
            .with_untracked(|v| v.clone().expect("memo not initialised"))
    }
}

/// Derived value that only notifies its readers when the result actually changes.
pub fn create_memo<T: PartialEq + Clone + 'static>(f: impl Fn() -> T + 'static) -> Memo<T> {
    let sig = create_signal(None::<T>);
    create_effect(move || {
        let v = f();
        if sig.with_untracked(|cur| cur.as_ref() != Some(&v)) {
            sig.set(Some(v));
        }
    });
    Memo(sig)
}

// ---------------------------------------------------------------------------
// Scopes
// ---------------------------------------------------------------------------

fn register(rt: &Runtime, f: impl FnOnce(&mut ScopeSlot)) {
    if let Some(id) = rt.scope.get() {
        if let Some(slot) = rt.scopes.borrow_mut().get_mut(id) {
            f(slot);
        }
    }
}

/// New scope, child of the current one (or a root scope if none is active).
pub fn create_scope() -> ScopeId {
    RT.with(|rt| {
        let parent = rt.scope.get();
        let id = rt.scopes.borrow_mut().insert(ScopeSlot {
            parent,
            ..Default::default()
        });
        if let Some(p) = parent {
            if let Some(ps) = rt.scopes.borrow_mut().get_mut(p) {
                ps.children.push(id);
            }
        }
        id
    })
}

/// Make `scope` current while `f` runs; things created inside belong to it.
pub fn with_scope<R>(scope: ScopeId, f: impl FnOnce() -> R) -> R {
    let prev = RT.with(|rt| rt.scope.replace(Some(scope)));
    let r = f();
    RT.with(|rt| rt.scope.set(prev));
    r
}

/// Free a scope, its children, and every signal/effect they own.
/// Effects are unsubscribed first so nothing keeps firing into freed views.
pub fn dispose_scope(id: ScopeId) {
    RT.with(|rt| {
        let slot = rt.scopes.borrow_mut().remove(id);
        let Some(slot) = slot else { return };

        if let Some(parent) = slot.parent {
            if let Some(p) = rt.scopes.borrow_mut().get_mut(parent) {
                p.children.retain(|c| *c != id);
            }
        }
        for child in slot.children {
            dispose_scope(child);
        }
        for e in slot.effects {
            let eff = rt.effects.borrow_mut().remove(e);
            if let Some(eff) = eff {
                let mut sigs = rt.signals.borrow_mut();
                for d in eff.deps {
                    if let Some(s) = sigs.get_mut(d) {
                        s.subs.retain(|x| *x != e);
                    }
                }
            }
        }
        for s in slot.signals {
            rt.signals.borrow_mut().remove(s);
        }
    });
}

// ---------------------------------------------------------------------------
// Tests (each test runs on its own thread, so each gets a fresh runtime)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_tracks_and_reruns() {
        let a = create_signal(1);
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        create_effect(move || l.borrow_mut().push(a.get()));
        a.set(2);
        a.set(3);
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn batch_runs_effect_once() {
        let a = create_signal(1);
        let b = create_signal(10);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        create_effect(move || {
            a.get();
            b.get();
            r.set(r.get() + 1);
        });
        assert_eq!(runs.get(), 1);
        batch(|| {
            a.set(2);
            b.set(20);
        });
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn conditional_dependencies_are_retracked() {
        let flag = create_signal(true);
        let a = create_signal(0);
        let b = create_signal(0);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        create_effect(move || {
            if flag.get() {
                a.get();
            } else {
                b.get();
            }
            r.set(r.get() + 1);
        });
        flag.set(false); // run 2, now depends on b, not a
        a.set(1); // must not re-run
        assert_eq!(runs.get(), 2);
        b.set(1);
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn memo_suppresses_equal_results() {
        let a = create_signal(1);
        let parity = create_memo(move || a.get() % 2);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        create_effect(move || {
            parity.get();
            r.set(r.get() + 1);
        });
        a.set(3); // parity unchanged
        assert_eq!(runs.get(), 1);
        a.set(4);
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn dispose_scope_stops_effects() {
        let a = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let scope = create_scope();
        with_scope(scope, || {
            create_effect(move || {
                a.get();
                c.set(c.get() + 1);
            });
        });
        a.set(1);
        assert_eq!(count.get(), 2);
        dispose_scope(scope);
        a.set(2);
        assert_eq!(count.get(), 2);
    }
}
