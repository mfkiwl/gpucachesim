use crate::sync::{Arc, Mutex};
use crate::{config, func_unit as fu, instruction::WarpInstruction, opcodes, register_set};

#[allow(clippy::module_name_repetitions)]
pub struct IntUnit {
    config: Arc<config::GPU>,
    inner: fu::PipelinedSimdUnit,
}

impl IntUnit {
    pub fn new(
        id: usize,
        result_port: register_set::Ref,
        config: Arc<config::GPU>,
        _stats: &Arc<Mutex<stats::PerKernel>>,
        issue_reg_id: usize,
    ) -> Self {
        let pipeline_depth = config.max_int_latency;
        let inner = fu::PipelinedSimdUnit::new(
            id,
            "IntUnit".to_string(),
            Some(result_port),
            pipeline_depth,
            config.clone(),
            issue_reg_id,
        );

        Self { config, inner }
    }
}

impl std::fmt::Display for IntUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IntUnit")
    }
}

impl std::fmt::Debug for IntUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntUnit").finish()
    }
}

impl fu::SimdFunctionUnit for IntUnit {
    fn can_issue(&self, instr: &WarpInstruction) -> bool {
        use opcodes::ArchOp;
        match instr.opcode.category {
            ArchOp::SFU_OP
            | ArchOp::LOAD_OP
            | ArchOp::TENSOR_CORE_LOAD_OP
            | ArchOp::STORE_OP
            | ArchOp::TENSOR_CORE_STORE_OP
            | ArchOp::MEMORY_BARRIER_OP
            | ArchOp::DP_OP => false,
            _ => self.inner.can_issue(instr),
        }
    }

    fn pipeline(&self) -> &Vec<Option<WarpInstruction>> {
        &self.inner.pipeline_reg
    }

    fn occupied(&self) -> &fu::OccupiedSlots {
        &self.inner.occupied
    }

    fn id(&self) -> &str {
        &self.inner.name
    }

    fn is_issue_partitioned(&self) -> bool {
        true
    }

    fn active_lanes_in_pipeline(&self) -> usize {
        let active = self.inner.active_lanes_in_pipeline();
        debug_assert!(active <= self.config.warp_size);
        // m_core->incspactivelanes_stat(active_count);
        // m_core->incfuactivelanes_stat(active_count);
        // m_core->incfumemactivelanes_stat(active_count);
        active
    }

    // fn issue(&mut self, source_reg: &mut RegisterSet) {
    fn issue(&mut self, source_reg: WarpInstruction) {
        // let ready_reg = source_reg.get_ready(self.config.sub_core_model, self.issue_reg_id);
        // m_core->incexecstat((*ready_reg));
        // ready_reg.op_pipe = SP__OP;
        // m_core->incsp_stat(m_core->get_config()->warp_size, (*ready_reg)->latency);

        self.inner.issue(source_reg);
    }

    fn issue_reg_id(&self) -> usize {
        self.inner.issue_reg_id
    }

    fn stallable(&self) -> bool {
        false
    }

    fn clock_multiplier(&self) -> usize {
        1
    }
}

impl crate::engine::cycle::Component for IntUnit {
    fn cycle(&mut self, cycle: u64) {
        self.inner.cycle(cycle);
    }
}
