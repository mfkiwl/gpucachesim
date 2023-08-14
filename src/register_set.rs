use super::instruction::WarpInstruction;
use std::sync::{Arc, Mutex};

// pub type RegisterSetRef = Rc<RefCell<RegisterSet>>;
pub type Ref = Arc<Mutex<RegisterSet>>;

/// Register that can hold multiple instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterSet {
    pub stage: super::PipelineStage,
    pub regs: Vec<Option<WarpInstruction>>,
    pub id: usize,
}

impl RegisterSet {
    #[must_use]
    pub fn new(stage: super::PipelineStage, size: usize, id: usize) -> Self {
        let regs = (0..size).map(|_| None).collect();
        Self { stage, regs, id }
    }

    pub fn has_free(&self) -> bool {
        self.regs.iter().any(Option::is_none)
        // self.regs.iter().any(|r| match r {
        //     Some(r) => r.empty(),
        //     None => true,
        // })
    }

    // pub fn has_free_sub_core(&self, sub_core_model: bool, reg_id: usize) -> bool {
    #[must_use]
    pub fn has_free_sub_core(&self, reg_id: usize) -> bool {
        // in subcore model, each sched has a one specific
        // reg to use (based on sched id)
        // if !sub_core_model {
        //     return self.has_free();
        // }

        debug_assert!(reg_id < self.regs.len());
        // self.regs[reg_id].is_none()
        let Some(reg) = self.regs.get(reg_id) else {
            return false;
        };

        reg.as_ref()
            // .and_then(Option::as_ref)
            // .flatten()
            .is_none()
        // .map(|r| r.empty())
        // .unwrap_or(false)
    }

    // pub fn has_ready(&self) -> bool {
    //     self.regs.iter().any(Option::is_some)
    //     // self.regs.iter().any(|r| match r {
    //     //     Some(r) => !r.empty(),
    //     //     None => false,
    //     // })
    // }

    // pub fn has_ready_sub_core(&self, sub_core_model: bool, reg_id: usize) -> bool {
    // pub fn has_ready_sub_core(&self, reg_id: usize) -> bool {
    //     // if !sub_core_model {
    //     //     return self.has_ready();
    //     // }
    //
    //     debug_assert!(reg_id < self.regs.len());
    //     match self.get_ready_sub_core(reg_id) {
    //         Some(ready) => !ready.empty(),
    //         None => true,
    //     }
    // }

    pub fn scheduler_id(&self, reg_id: usize) -> Option<usize> {
        match self.regs.get(reg_id).and_then(Option::as_ref) {
            Some(r) => {
                // debug_assert!(!r.empty());
                r.scheduler_id
            }
            None => None,
        }
    }

    // pub fn take_ready(&mut self) -> Option<WarpInstruction> {
    //     let mut ready: &Option<WarpInstruction> = &None;
    //     for free in self.regs.iter().filter_map(|&r| r) {
    //         if let Some(ref mut r) = ready {
    //             if free.uid < r.uid {
    //                 // free is older
    //                 *r = free;
    //             }
    //         } else {
    //             ready = &Some(free);
    //         }
    //     }
    //     ready.take()
    // }

    pub fn has_ready(&self) -> bool {
        self.regs.iter().any(Option::is_some)
    }

    #[must_use]
    pub fn get_ready(&self) -> Option<(usize, &Option<WarpInstruction>)> {
        let mut ready: Option<(usize, &Option<WarpInstruction>)> = None;
        for free in self.iter_occupied() {
            match (&ready, free) {
                (Some((_, Some(ref r))), (_, Some(ref f))) if f.uid < r.uid => {
                    // free is older
                    ready = Some(free);
                }
                (None, free) => ready = Some(free),
                _ => {}
            }
        }
        ready
    }

    // pub fn get_ready_reg_id(&self) -> &Option<WarpInstruction> {
    //     // let mut ready: &Option<WarpInstruction> = &None;
    //     for (reg_id, free) in self.iter_occupied() {
    //         match (ready, free) {
    //             (Some(ref r), Some(ref f)) if f.uid < r.uid => {
    //                 // free is older
    //                 ready = free;
    //             }
    //             (None, free) => ready = free,
    //             _ => {}
    //         }
    //         // if let (Some(ref mut r), Some(ref mut f)) = (ready, free) {
    //         //     if f.uid < r.uid {
    //         //         // free is older
    //         //         ready = free;
    //         //     }
    //         // } else if ready.is_none() {
    //         //     ready = free;
    //         // }
    //     }
    //     ready
    // }

    // pub fn ready_reg_id(&self) -> Option<usize> {
    //     // for sub core model we need to figure which reg_id has
    //     // the ready warp this function should only be called
    //     // if has_ready() was true
    //     // debug_assert!(self.has_ready());
    //     let mut non_empty = self
    //         .regs
    //         .iter()
    //         .map(Option::as_ref)
    //         .filter_map(|r| r)
    //         .filter(|r| !r.empty());
    //
    //     let mut ready: Option<&WarpInstruction> = None;
    //     let mut reg_id = None;
    //     for (i, reg) in non_empty.enumerate() {
    //         match ready {
    //             Some(ready) if ready.warp_id < reg.warp_id => {
    //                 // ready is oldest
    //             }
    //             _ => {
    //                 ready.insert(reg);
    //                 reg_id = Some(i);
    //             }
    //         }
    //     }
    //     reg_id
    // }

    pub fn get_ready_mut(&mut self) -> Option<(usize, &mut Option<WarpInstruction>)> {
        let mut oldest: Option<(usize, &mut Option<WarpInstruction>)> = None;
        for ready in self.iter_occupied_mut() {
            if let (Some((_, Some(o))), (_, Some(r))) = (&oldest, &ready) {
                log::trace!(
                    "oldest={} uid = {}  <  ready={} uid = {}",
                    o,
                    o.uid,
                    r,
                    r.uid
                );
            }
            match (&oldest, &ready) {
                (Some((_, Some(o))), (_, Some(r))) if o.uid < r.uid => {
                    // ready is older
                    // ready is newer, so nothing to do here
                    // oldest = Some(ready);
                }
                // (None, _) => oldest = Some(ready),
                _ => oldest = Some(ready),
                // _ => {}
            }
        }
        oldest
    }

    // pub fn get_instruction(&self) -> Option<&WarpInstruction> {
    //     self.get_ready().map(Option::as_ref).flatten().flatten()
    // }

    pub fn get_instruction_mut(&mut self) -> Option<&mut WarpInstruction> {
        self.get_ready_mut()
            .map(|(_, r)| r)
            .and_then(Option::as_mut)
    }

    // pub fn get_ready_mut(&mut self) -> Option<&mut WarpInstruction> {
    //     let mut ready: Option<&mut WarpInstruction> = None;
    //     for free in self.iter_instructions_mut() {
    //         if let Some(ref mut r) = ready {
    //             if free.uid < r.uid {
    //                 // free is older
    //                 *r = free;
    //             }
    //         } else {
    //             ready = Some(free);
    //         }
    //     }
    //     ready
    // }

    #[must_use]
    pub fn get_ready_sub_core(&self, reg_id: usize) -> Option<&Option<WarpInstruction>> {
        debug_assert!(reg_id < self.regs.len());
        self.regs.get(reg_id)
    }

    pub fn get_ready_sub_core_mut(
        &mut self,
        reg_id: usize,
    ) -> Option<&mut Option<WarpInstruction>> {
        debug_assert!(reg_id < self.regs.len());
        self.regs.get_mut(reg_id)
    }

    pub fn get_instruction_sub_core(&self, reg_id: usize) -> Option<&WarpInstruction> {
        debug_assert!(reg_id < self.regs.len());
        self.regs.get(reg_id).and_then(Option::as_ref)
    }

    pub fn get_instruction_sub_core_mut(&mut self, reg_id: usize) -> Option<&mut WarpInstruction> {
        debug_assert!(reg_id < self.regs.len());
        self.regs.get_mut(reg_id).and_then(Option::as_mut)
    }

    pub fn iter_occupied(&self) -> impl Iterator<Item = (usize, &Option<WarpInstruction>)> {
        self.regs.iter().enumerate().filter(|(_, r)| r.is_some())
    }

    pub fn iter_occupied_mut(
        &mut self,
    ) -> impl Iterator<Item = (usize, &mut Option<WarpInstruction>)> {
        self.regs
            .iter_mut()
            .enumerate()
            .filter(|(_, r)| r.is_some())
    }

    pub fn iter_instructions(&self) -> impl Iterator<Item = &WarpInstruction> {
        self.regs.iter().filter_map(Option::as_ref)
    }

    pub fn iter_instructions_mut(&mut self) -> impl Iterator<Item = &mut WarpInstruction> {
        self.regs.iter_mut().filter_map(Option::as_mut)
    }

    pub fn iter_free(&self) -> impl Iterator<Item = &Option<WarpInstruction>> {
        self.regs.iter().filter(|r| r.is_none())
    }

    pub fn iter_free_mut(&mut self) -> impl Iterator<Item = (usize, &mut Option<WarpInstruction>)> {
        self.regs
            .iter_mut()
            .enumerate()
            .filter(|(_i, r)| r.is_none())
    }

    pub fn get_free_mut(&mut self) -> Option<(usize, &mut Option<WarpInstruction>)> {
        // let mut free = self
        //     .regs
        //     .iter_mut()
        //     // .map(Option::as_mut)
        //     .filter(|r| r.is_none());
        // .filter(Option::is_none);
        // .filter_map(|r| r.as_ref())
        // .filter_map(|r| r.as_ref())
        // .filter(|r| r.empty());
        self.iter_free_mut().next()
    }

    pub fn get_free_sub_core_mut(
        &mut self,
        reg_id: usize,
    ) -> Option<(usize, &mut Option<WarpInstruction>)> {
        // in subcore model, each sched has a one specific reg
        // to use (based on sched id)
        debug_assert!(reg_id < self.regs.len());
        self.regs.get_mut(reg_id).map(|r| (reg_id, r))
        // .and_then(Option::as_ref) .filter(|r| r.empty())
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.regs.len()
    }

    #[must_use]
    pub fn empty(&self) -> bool {
        todo!("RegisterSet::empty")
    }

    // pub fn move_in_from(&mut self, src: Option<WarpInstruction>, msg: impl AsRef<str>) {
    pub fn move_in_from(&mut self, src: Option<WarpInstruction>) {
        // panic!("move {:?} in {}", src, self);
        let (_, free) = self.get_free_mut().unwrap();
        // let msg = format!(
        //     "register set moving in from {:?} to free={:?}",
        //     src.as_ref().map(ToString::to_string),
        //     free.as_ref().map(ToString::to_string)
        // );
        move_warp(src, free);
    }

    pub fn move_in_from_sub_core(
        &mut self,
        reg_id: usize,
        src: Option<WarpInstruction>,
        // msg: impl AsRef<str>,
    ) {
        //     assert(reg_id < regs.size());
        let (_, free) = self.get_free_sub_core_mut(reg_id).unwrap();
        // let msg = format!(
        //     "register set moving in from sub core {:?} to free={:?}",
        //     src.as_ref().map(ToString::to_string),
        //     free.as_ref().map(ToString::to_string),
        // );
        move_warp(src, free);
    }

    // pub fn move_out_to(&mut self, dest: &mut Option<WarpInstruction>, msg: impl AsRef<str>) {
    pub fn move_out_to(&mut self, dest: &mut Option<WarpInstruction>) {
        let ready: Option<WarpInstruction> =
            self.get_ready_mut().map(|(_, r)| r).and_then(Option::take);
        // let msg = format!(
        //     "register set moving out from ready={:?} to {:?}",
        //     ready.as_ref().map(ToString::to_string),
        //     dest.as_ref().map(ToString::to_string)
        // );

        move_warp(ready, dest);
    }

    pub fn move_out_to_sub_core(
        &mut self,
        reg_id: usize,
        dest: &mut Option<WarpInstruction>,
        // msg: impl AsRef<str>,
    ) {
        let ready: Option<WarpInstruction> =
            self.get_ready_sub_core_mut(reg_id).and_then(Option::take);
        // let msg = format!(
        //     "register set moving out to sub core from ready={:?} to {:?}",
        //     ready.as_ref().map(ToString::to_string),
        //     dest.as_ref().map(ToString::to_string),
        // );
        move_warp(ready, dest);
    }
}

impl std::fmt::Display for RegisterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let instructions = self
            .regs
            .iter()
            .map(|inst| inst.as_ref().map(std::string::ToString::to_string));
        f.debug_list().entries(instructions).finish()
    }
}

// fn swap<T>(x: &mut [T], i: usize, j: usize) {
//     let (lo, hi) = match i.cmp(&j) {
//         // no swapping necessary
//         std::cmp::Ordering::Equal => return,
//
//         // get the smallest and largest of the two indices
//         std::cmp::Ordering::Less => (i, j),
//         std::cmp::Ordering::Greater => (j, i),
//     };
//
//     let (init, tail) = x.split_at_mut(hi);
//     std::mem::swap(&mut init[lo], &mut tail[0]);
// }

// pub fn move_warp<T: std::fmt::Display>(from: Option<T>, to: &mut Option<T>, msg: impl AsRef<str>) {
#[inline(always)]
pub fn move_warp<T: std::fmt::Display>(from: Option<T>, to: &mut Option<T>) {
    // log::debug!(
    //     "{}",
    //     style(format!(
    //         "MOVING {:?} to {:?}: {}",
    //         from.as_ref().map(|i| i.to_string()),
    //         to.as_ref().map(|i| i.to_string()),
    //         msg.as_ref(),
    //     ))
    //     .white()
    //     .bold()
    // );
    // debug_assert!(to.is_none());
    // debug_assert!(from.is_some());
    *to = from;
}