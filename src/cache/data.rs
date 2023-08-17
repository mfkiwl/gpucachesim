use super::{base, event};
use crate::{address, cache, config, interconn as ic, mem_fetch, tag_array, Cycle};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// First level data cache in Fermi.
///
/// The cache uses a write-evict (global) or write-back (local) policy
/// at the granularity of individual blocks.
/// (the policy used in fermi according to the CUDA manual)
#[derive(Debug)]
pub struct Data<I> {
    pub inner: base::Base<I>,
    /// Specifies type of write allocate request (e.g., L1 or L2)
    write_alloc_type: mem_fetch::AccessKind,
    /// Specifies type of writeback request (e.g., L1 or L2)
    write_back_type: mem_fetch::AccessKind,
}

impl<I> Data<I>
where
    I: ic::MemFetchInterface,
{
    pub fn new(
        name: String,
        core_id: usize,
        cluster_id: usize,
        cycle: Cycle,
        mem_port: Arc<I>,
        stats: Arc<Mutex<stats::Cache>>,
        config: Arc<config::GPU>,
        cache_config: Arc<config::Cache>,
        write_alloc_type: mem_fetch::AccessKind,
        write_back_type: mem_fetch::AccessKind,
    ) -> Self {
        let inner = super::base::Base::new(
            name,
            core_id,
            cluster_id,
            cycle,
            mem_port,
            stats,
            config,
            cache_config,
        );
        Self {
            inner,
            write_alloc_type,
            write_back_type,
        }
    }

    #[must_use]
    pub fn cache_config(&self) -> &Arc<config::Cache> {
        &self.inner.cache_config
    }

    /// Write-back hit: mark block as modified.
    fn write_hit_write_back(
        &mut self,
        addr: address,
        cache_index: Option<usize>,
        fetch: &mem_fetch::MemFetch,
        time: u64,
        _events: &mut [event::Event],
        _probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        debug_assert_eq!(addr, fetch.addr());

        let block_addr = self.inner.cache_config.block_addr(addr);
        log::debug!(
            "handling WRITE HIT WRITE BACK for {} (block_addr={}, cache_idx={:?})",
            fetch,
            block_addr,
            cache_index,
        );

        // update LRU state
        let tag_array::AccessStatus { index, .. } =
            self.inner.tag_array.access(block_addr, fetch, time);
        let cache_index = index.unwrap();
        let block = self.inner.tag_array.get_block_mut(cache_index);
        let was_modified_before = block.is_modified();
        block.set_status(cache::block::Status::MODIFIED, fetch.access_sector_mask());
        block.set_byte_mask(fetch.access_byte_mask());
        if !was_modified_before {
            self.inner.tag_array.num_dirty += 1;
        }
        self.update_readable(fetch, cache_index);

        cache::RequestStatus::HIT
    }

    fn update_readable(&mut self, fetch: &mem_fetch::MemFetch, cache_index: usize) {
        use crate::mem_sub_partition::{SECTOR_CHUNCK_SIZE, SECTOR_SIZE};
        let block = self.inner.tag_array.get_block_mut(cache_index);
        for i in 0..SECTOR_CHUNCK_SIZE as usize {
            let sector_mask = fetch.access_sector_mask();
            if sector_mask[i] {
                let mut all_set = true;
                for k in (i * SECTOR_SIZE as usize)..((i + 1) * SECTOR_SIZE as usize) {
                    // If any bit in the byte mask (within the sector) is not set,
                    // the sector is unreadble
                    if !block.dirty_byte_mask()[k] {
                        all_set = false;
                        break;
                    }
                }
                if all_set {
                    block.set_readable(true, fetch.access_sector_mask());
                }
            }
        }
    }

    fn read_hit(
        &mut self,
        addr: address,
        _cache_index: Option<usize>,
        // cache_index: usize,
        fetch: &mem_fetch::MemFetch,
        time: u64,
        _events: &mut [event::Event],
        _probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        let super::base::Base {
            ref mut tag_array,
            ref cache_config,
            ..
        } = self.inner;
        let block_addr = cache_config.block_addr(addr);
        let access_status = tag_array.access(block_addr, fetch, time);
        let block_index = access_status.index.expect("read hit has index");

        // Atomics treated as global read/write requests:
        // Perform read, mark line as MODIFIED
        if fetch.is_atomic() {
            debug_assert_eq!(*fetch.access_kind(), mem_fetch::AccessKind::GLOBAL_ACC_R);
            let block = tag_array.get_block_mut(block_index);
            let was_modified_before = block.is_modified();
            block.set_status(cache::block::Status::MODIFIED, fetch.access_sector_mask());
            block.set_byte_mask(fetch.access_byte_mask());
            if !was_modified_before {
                tag_array.num_dirty += 1;
            }
        }
        cache::RequestStatus::HIT
    }

    /// Sends write request to lower level memory (write or writeback)
    pub fn send_write_request(
        &mut self,
        mut fetch: mem_fetch::MemFetch,
        request: event::Event,
        time: u64,
        events: &mut Vec<event::Event>,
    ) {
        log::debug!("data_cache::send_write_request({})", fetch);
        events.push(request);
        fetch.set_status(self.inner.miss_queue_status, time);
        self.inner.miss_queue.push_back(fetch);
    }

    /// Baseline read miss
    ///
    /// Send read request to lower level memory and perform
    /// write-back as necessary.
    fn read_miss(
        &mut self,
        addr: address,
        cache_index: Option<usize>,
        fetch: &mem_fetch::MemFetch,
        time: u64,
        events: &mut Vec<event::Event>,
        _probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        if !self.inner.miss_queue_can_fit(1) {
            // cannot handle request this cycle
            // (might need to generate two requests)
            let mut stats = self.inner.stats.lock().unwrap();
            stats.inc(
                *fetch.access_kind(),
                cache::AccessStat::ReservationFailure(cache::ReservationFailure::MISS_QUEUE_FULL),
                1,
            );
            return cache::RequestStatus::RESERVATION_FAIL;
        }

        let block_addr = self.inner.cache_config.block_addr(addr);
        let (should_miss, writeback, evicted) = self.inner.send_read_request(
            addr,
            block_addr,
            cache_index.unwrap(),
            fetch.clone(),
            time,
            events,
            false,
            false,
        );

        let writeback_policy = self.inner.cache_config.write_policy;
        log::debug!(
            "handling READ MISS for {} (should miss={}, writeback={}, writeback policy={:?})",
            fetch,
            should_miss,
            writeback,
            writeback_policy,
        );

        if should_miss {
            // If evicted block is modified and not a write-through
            // (already modified lower level)
            if writeback && writeback_policy != config::CacheWritePolicy::WRITE_THROUGH {
                if let Some(evicted) = evicted {
                    let is_write = true;
                    let writeback_access = mem_fetch::MemAccess::new(
                        self.write_back_type,
                        evicted.block_addr,
                        evicted.allocation.clone(),
                        evicted.modified_size,
                        is_write,
                        *fetch.access_warp_mask(),
                        evicted.byte_mask,
                        evicted.sector_mask,
                    );

                    let mut writeback_fetch = mem_fetch::MemFetch::new(
                        fetch.instr.clone(),
                        writeback_access,
                        &self.inner.config,
                        if is_write {
                            mem_fetch::WRITE_PACKET_SIZE
                        } else {
                            mem_fetch::READ_PACKET_SIZE
                        }
                        .into(),
                        0,
                        0,
                        0,
                    );

                    // the evicted block may have wrong chip id when
                    // advanced L2 hashing is used, so set the right chip
                    // address from the original mf
                    writeback_fetch.tlx_addr.chip = fetch.tlx_addr.chip;
                    writeback_fetch.tlx_addr.sub_partition = fetch.tlx_addr.sub_partition;
                    let event = event::Event::WriteBackRequestSent {
                        evicted_block: None,
                    };

                    log::trace!(
                        "handling READ MISS for {}: => sending writeback {}",
                        fetch,
                        writeback_fetch
                    );

                    self.send_write_request(writeback_fetch, event, time, events);
                }
            }
            return cache::RequestStatus::MISS;
        }

        cache::RequestStatus::RESERVATION_FAIL
    }

    fn write_miss_no_write_allocate(
        &mut self,
        addr: address,
        _cache_index: Option<usize>,
        fetch: mem_fetch::MemFetch,
        time: u64,
        events: &mut Vec<event::Event>,
        _probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        debug_assert_eq!(addr, fetch.addr());
        log::debug!(
            "handling WRITE MISS NO WRITE ALLOCATE for {} (miss_queue_full={})",
            fetch,
            self.inner.miss_queue_full()
        );

        if self.inner.miss_queue_full() {
            let mut stats = self.inner.stats.lock().unwrap();
            stats.inc(
                *fetch.access_kind(),
                cache::AccessStat::ReservationFailure(cache::ReservationFailure::MISS_QUEUE_FULL),
                1,
            );
            // cannot handle request this cycle
            return cache::RequestStatus::RESERVATION_FAIL;
        }

        // on miss, generate write through
        let event = event::Event::WriteRequestSent;
        self.send_write_request(fetch, event, time, events);
        cache::RequestStatus::MISS
    }

    #[allow(clippy::needless_pass_by_value)]
    fn write_miss_write_allocate_naive(
        &mut self,
        addr: address,
        cache_index: Option<usize>,
        fetch: mem_fetch::MemFetch,
        time: u64,
        events: &mut Vec<event::Event>,
        probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        // what exactly is the difference between the addr and the fetch addr?
        debug_assert_eq!(addr, fetch.addr());

        let block_addr = self.inner.cache_config.block_addr(addr);
        let mshr_addr = self.inner.cache_config.mshr_addr(fetch.addr());

        // Write allocate, maximum 3 requests:
        //  (write miss, read request, write back request)
        //
        //  Conservatively ensure the worst-case request can be handled this cycle
        let mshr_hit = self.inner.mshrs.probe(mshr_addr);
        let mshr_free = !self.inner.mshrs.full(mshr_addr);
        let mshr_miss_but_free = !mshr_hit && mshr_free && !self.inner.miss_queue_full();

        log::debug!("handling write miss for {} (block addr={}, mshr addr={}, mshr hit={} mshr avail={}, miss queue full={})", &fetch, block_addr, mshr_addr, mshr_hit, mshr_free, self.inner.miss_queue_can_fit(2));

        if !self.inner.miss_queue_can_fit(2) || !(mshr_miss_but_free || mshr_hit && mshr_free) {
            // check what is the exact failure reason
            let failure = if !self.inner.miss_queue_can_fit(2) {
                cache::ReservationFailure::MISS_QUEUE_FULL
            } else if mshr_hit && !mshr_free {
                cache::ReservationFailure::MSHR_MERGE_ENTRY_FAIL
            } else if !mshr_hit && !mshr_free {
                cache::ReservationFailure::MSHR_ENTRY_FAIL
            } else {
                panic!("write_miss_write_allocate_naive bad reason");
            };

            let mut stats = self.inner.stats.lock().unwrap();
            stats.inc(
                *fetch.access_kind(),
                cache::AccessStat::ReservationFailure(failure),
                1,
            );
            log::debug!("handling write miss for {}: RESERVATION FAIL", &fetch);
            return cache::RequestStatus::RESERVATION_FAIL;
        }

        let event = event::Event::WriteRequestSent;
        self.send_write_request(fetch.clone(), event, time, events);

        let is_write = false;
        let new_access = mem_fetch::MemAccess::new(
            self.write_alloc_type,
            fetch.addr(),
            fetch.access.allocation.clone(),
            self.cache_config().atom_size(),
            is_write, // Now performing a read
            *fetch.access_warp_mask(),
            *fetch.access_byte_mask(),
            *fetch.access_sector_mask(),
        );

        let new_fetch = mem_fetch::MemFetch::new(
            None,
            new_access,
            &self.inner.config,
            fetch.control_size(),
            fetch.warp_id,
            fetch.core_id,
            fetch.cluster_id,
        );

        // Send read request resulting from write miss
        let is_read_only = false;
        let is_write_allocate = true;
        let (should_miss, writeback, evicted) = self.inner.send_read_request(
            addr,
            block_addr,
            cache_index.unwrap(),
            new_fetch,
            time,
            events,
            is_read_only,
            is_write_allocate,
        );

        events.push(event::Event::WriteAllocateSent);

        if should_miss {
            // If evicted block is modified and not a write-through
            // (already modified lower level)
            // log::debug!(
            //     "evicted block: {:?}",
            //     evicted.as_ref().map(|e| e.block_addr)
            // );
            let not_write_through =
                self.cache_config().write_policy != config::CacheWritePolicy::WRITE_THROUGH;

            if writeback && not_write_through {
                if let Some(evicted) = evicted {
                    log::debug!("evicted block: {:?}", evicted.block_addr);

                    // SECTOR_MISS and HIT_RESERVED should not send write back
                    debug_assert_eq!(probe_status, cache::RequestStatus::MISS);

                    let is_write = true;
                    let writeback_access = mem_fetch::MemAccess::new(
                        self.write_back_type,
                        evicted.block_addr,
                        evicted.allocation.clone(),
                        evicted.modified_size,
                        is_write,
                        *fetch.access_warp_mask(),
                        evicted.byte_mask,
                        evicted.sector_mask,
                    );
                    let control_size = writeback_access.control_size();
                    let mut writeback_fetch = mem_fetch::MemFetch::new(
                        None,
                        writeback_access,
                        &self.inner.config,
                        control_size,
                        0, // warp id
                        0, // self.core_id,
                        0, // self.cluster_id,
                    );

                    // the evicted block may have wrong chip id when advanced L2 hashing
                    // is used, so set the right chip address from the original mf
                    writeback_fetch.tlx_addr.chip = fetch.tlx_addr.chip;
                    writeback_fetch.tlx_addr.sub_partition = fetch.tlx_addr.sub_partition;
                    let event = event::Event::WriteBackRequestSent {
                        evicted_block: Some(evicted),
                    };

                    self.send_write_request(writeback_fetch, event, time, events);
                }
            }
            return cache::RequestStatus::MISS;
        }

        cache::RequestStatus::RESERVATION_FAIL
    }

    fn write_miss(
        &mut self,
        addr: address,
        cache_index: Option<usize>,
        fetch: mem_fetch::MemFetch,
        time: u64,
        events: &mut Vec<event::Event>,
        probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        let func = match self.inner.cache_config.write_allocate_policy {
            config::CacheWriteAllocatePolicy::NO_WRITE_ALLOCATE => {
                Self::write_miss_no_write_allocate
            }
            config::CacheWriteAllocatePolicy::WRITE_ALLOCATE => {
                Self::write_miss_write_allocate_naive
            }
            config::CacheWriteAllocatePolicy::FETCH_ON_WRITE => {
                // Self::write_miss_write_allocate_fetch_on_write
                unimplemented!("fetch on write")
            }
            config::CacheWriteAllocatePolicy::LAZY_FETCH_ON_READ => {
                // Self::write_miss_write_allocate_lazy_fetch_on_read
                unimplemented!("fetch on read")
            }
        };
        (func)(self, addr, cache_index, fetch, time, events, probe_status)
    }

    fn write_hit(
        &mut self,
        addr: address,
        cache_index: Option<usize>,
        fetch: &mem_fetch::MemFetch,
        time: u64,
        events: &mut [event::Event],
        probe_status: cache::RequestStatus,
    ) -> cache::RequestStatus {
        let func = match self.inner.cache_config.write_policy {
            // TODO: make read only policy deprecated
            // READ_ONLY is now a separate cache class, config is deprecated
            config::CacheWritePolicy::READ_ONLY => unimplemented!("todo: remove the read only cache write policy / writable data cache set as READ_ONLY"),
            config::CacheWritePolicy::WRITE_BACK => Self::write_hit_write_back,
            config::CacheWritePolicy::WRITE_THROUGH => unimplemented!("Self::wr_hit_wt"),
            config::CacheWritePolicy::WRITE_EVICT => unimplemented!("Self::wr_hit_we"),
            config::CacheWritePolicy::LOCAL_WB_GLOBAL_WT => unimplemented!("Self::wr_hit_global_we_local_wb"),
        };
        (func)(self, addr, cache_index, fetch, time, events, probe_status)
    }

    // A general function that takes the result of a tag_array probe.
    //
    // It performs the correspding functions based on the
    // cache configuration.
    fn process_tag_probe(
        &mut self,
        is_write: bool,
        probe_status: cache::RequestStatus,
        addr: address,
        cache_index: Option<usize>,
        fetch: mem_fetch::MemFetch,
        events: &mut Vec<event::Event>,
        time: u64,
    ) -> cache::RequestStatus {
        // dbg!(cache_index, probe_status);
        // Each function pointer ( m_[rd/wr]_[hit/miss] ) is set in the
        // data_cache constructor to reflect the corresponding cache
        // configuration options.
        //
        // Function pointers were used to avoid many long conditional
        // branches resulting from many cache configuration options.
        // let time = self.inner.cycle.get();
        let mut access_status = probe_status;
        let data_size = fetch.data_size();

        if is_write {
            if probe_status == cache::RequestStatus::HIT {
                // let cache_index = cache_index.expect("hit has cache idx");
                access_status =
                    self.write_hit(addr, cache_index, &fetch, time, events, probe_status);
            } else if probe_status != cache::RequestStatus::RESERVATION_FAIL
                || (probe_status == cache::RequestStatus::RESERVATION_FAIL
                    && self.inner.cache_config.write_allocate_policy
                        == config::CacheWriteAllocatePolicy::NO_WRITE_ALLOCATE)
            {
                access_status =
                    self.write_miss(addr, cache_index, fetch, time, events, probe_status);
            } else {
                // the only reason for reservation fail here is LINE_ALLOC_FAIL
                // (i.e all lines are reserved)
                let mut stats = self.inner.stats.lock().unwrap();
                stats.inc(
                    *fetch.access_kind(),
                    cache::AccessStat::ReservationFailure(
                        cache::ReservationFailure::LINE_ALLOC_FAIL,
                    ),
                    1,
                );
            }
        } else if probe_status == cache::RequestStatus::HIT {
            access_status = self.read_hit(addr, cache_index, &fetch, time, events, probe_status);
        } else if probe_status != cache::RequestStatus::RESERVATION_FAIL {
            access_status = self.read_miss(addr, cache_index, &fetch, time, events, probe_status);
        } else {
            // the only reason for reservation fail here is LINE_ALLOC_FAIL
            // (i.e all lines are reserved)
            let mut stats = self.inner.stats.lock().unwrap();
            stats.inc(
                *fetch.access_kind(),
                cache::AccessStat::ReservationFailure(cache::ReservationFailure::LINE_ALLOC_FAIL),
                1,
            );
        }

        self.inner
            .bandwidth
            .use_data_port(data_size, access_status, events);

        access_status
    }
}

impl<I> cache::Component for Data<I>
where
    I: ic::MemFetchInterface,
{
    fn cycle(&mut self) {
        self.inner.cycle();
    }
}

impl<I> cache::Cache for Data<I>
where
    I: ic::MemFetchInterface + 'static,
{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn stats(&self) -> &Arc<Mutex<stats::Cache>> {
        &self.inner.stats
    }

    fn access(
        &mut self,
        addr: address,
        fetch: mem_fetch::MemFetch,
        events: &mut Vec<event::Event>,
        time: u64,
    ) -> cache::RequestStatus {
        let super::base::Base {
            ref cache_config, ..
        } = self.inner;

        debug_assert_eq!(&fetch.access.addr, &addr);
        debug_assert!(fetch.data_size() <= cache_config.atom_size());

        let is_write = fetch.is_write();
        let access_kind = *fetch.access_kind();
        let block_addr = cache_config.block_addr(addr);

        log::debug!(
            "{}::data_cache::access({fetch}, write = {is_write}, size = {}, block = {block_addr}, time = {})",
            self.inner.name,
            fetch.data_size(), time,
        );

        let dbg_fetch = fetch.clone();

        let (cache_index, probe_status) = self
            .inner
            .tag_array
            .probe(block_addr, &fetch, is_write, true);
        // dbg!((cache_index, probe_status));

        let access_status = self.process_tag_probe(
            is_write,
            probe_status,
            addr,
            cache_index,
            fetch,
            events,
            time,
        );
        // dbg!(&access_status);

        log::debug!(
            "{}::access({}) => probe status={:?} access status={:?}",
            self.inner.name,
            &dbg_fetch,
            probe_status,
            access_status
        );

        {
            let mut stats = self.inner.stats.lock().unwrap();
            let stat_cache_request_status = match probe_status {
                cache::RequestStatus::HIT_RESERVED
                    if access_status != cache::RequestStatus::RESERVATION_FAIL =>
                {
                    probe_status
                }
                cache::RequestStatus::SECTOR_MISS
                    if access_status != cache::RequestStatus::MISS =>
                {
                    probe_status
                }
                _status => access_status,
            };
            stats.inc(
                access_kind,
                cache::AccessStat::Status(stat_cache_request_status),
                1,
            );
        }
        // m_stats.inc_stats_pw(
        // mf->get_access_type(),
        // m_stats.select_stats_status(probe_status, access_status));
        access_status
    }

    fn write_allocate_policy(&self) -> config::CacheWriteAllocatePolicy {
        self.cache_config().write_allocate_policy
    }

    fn next_access(&mut self) -> Option<mem_fetch::MemFetch> {
        self.inner.next_access()
    }

    fn ready_accesses(&self) -> Option<&VecDeque<mem_fetch::MemFetch>> {
        self.inner.ready_accesses()
    }

    fn has_ready_accesses(&self) -> bool {
        self.inner.has_ready_accesses()
    }

    fn fill(&mut self, fetch: mem_fetch::MemFetch, time: u64) {
        self.inner.fill(fetch, time);
    }

    fn waiting_for_fill(&self, fetch: &mem_fetch::MemFetch) -> bool {
        self.inner.waiting_for_fill(fetch)
    }
}

impl<I> cache::Bandwidth for Data<I> {
    fn has_free_data_port(&self) -> bool {
        self.inner.has_free_data_port()
    }

    fn has_free_fill_port(&self) -> bool {
        self.inner.has_free_fill_port()
    }
}
