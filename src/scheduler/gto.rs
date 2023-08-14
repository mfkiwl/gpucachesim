use super::{BaseSchedulerUnit, SchedulerUnit, WarpRef};
use crate::{config::GPUConfig, core::WarpIssuer, scoreboard::Scoreboard};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug)]
pub struct Scheduler {
    inner: BaseSchedulerUnit,
}

impl Scheduler {
    pub fn new(
        id: usize,
        cluster_id: usize,
        core_id: usize,
        warps: Vec<WarpRef>,
        scoreboard: Arc<RwLock<Scoreboard>>,
        stats: Arc<Mutex<stats::scheduler::Scheduler>>,
        config: Arc<GPUConfig>,
    ) -> Self {
        let inner =
            BaseSchedulerUnit::new(id, cluster_id, core_id, warps, scoreboard, stats, config);
        Self { inner }
    }
}

impl Scheduler {
    fn debug_warp_ids(&self) -> Vec<usize> {
        self.inner
            .next_cycle_prioritized_warps
            .iter()
            // .map(|w| w.borrow().warp_id)
            .map(|(_idx, w)| w.try_lock().unwrap().warp_id)
            .collect()
    }

    fn debug_dynamic_warp_ids(&self) -> Vec<usize> {
        self.inner
            .next_cycle_prioritized_warps
            .iter()
            // .map(|w| w.borrow().dynamic_warp_id())
            .map(|(_idx, w)| w.try_lock().unwrap().dynamic_warp_id())
            .collect()
    }
}

impl SchedulerUnit for Scheduler {
    fn order_warps(&mut self) {
        self.inner.order_by_priority(
            super::ordering::Ordering::GREEDY_THEN_PRIORITY_FUNC,
            super::ordering::sort_warps_by_oldest_dynamic_id,
        );
    }

    fn add_supervised_warp(&mut self, warp: WarpRef) {
        self.inner.supervised_warps.push_back(warp);
    }

    fn prioritized_warps(&self) -> &VecDeque<(usize, WarpRef)> {
        self.inner.prioritized_warps()
    }

    fn cycle(&mut self, issuer: &mut dyn WarpIssuer) {
        log::debug!(
            "gto scheduler[{}]: BEFORE: prioritized warp ids: {:?}",
            self.inner.id,
            self.debug_warp_ids()
        );
        log::debug!(
            "gto scheduler[{}]: BEFORE: prioritized dynamic warp ids: {:?}",
            self.inner.id,
            self.debug_dynamic_warp_ids()
        );

        self.order_warps();

        log::debug!(
            "gto scheduler[{}]: AFTER: prioritized warp ids: {:?}",
            self.inner.id,
            self.debug_warp_ids()
        );
        log::debug!(
            "gto scheduler[{}]: AFTER: prioritized dynamic warp ids: {:?}",
            self.inner.id,
            self.debug_dynamic_warp_ids()
        );

        self.inner.cycle(issuer);
    }
}