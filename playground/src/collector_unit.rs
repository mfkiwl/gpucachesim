use playground_sys::operand_collector::collector_unit_t;

#[derive(Clone)]
pub struct CollectorUnit<'a> {
    pub(crate) set_id: u32,
    pub(crate) unit: &'a collector_unit_t,
}

impl<'a> CollectorUnit<'a> {
    #[must_use]
    pub fn set_id(&self) -> u32 {
        self.set_id
    }

    #[must_use]
    pub fn warp_id(&self) -> Option<usize> {
        if self.unit.is_free() {
            None
        } else {
            Some(self.unit.get_warp_id() as usize)
        }
    }

    #[must_use]
    // pub fn reg_id(&self) -> Option<usize> {
    pub fn reg_id(&self) -> usize {
        self.unit.get_reg_id() as usize
        // if self.unit.is_free() {
        //     None
        // } else {
        //     Some(self.unit.get_reg_id() as usize)
        // }
    }

    #[must_use]
    pub fn warp_instruction(&self) -> Option<super::warp_inst::WarpInstr<'a>> {
        if self.unit.is_free() {
            None
        } else {
            Some(unsafe { super::warp_inst::WarpInstr::wrap_ptr(self.unit.get_warp_instruction()) })
        }
    }

    #[must_use]
    pub fn output_register(&self) -> Option<super::register_set::RegisterSet<'a>> {
        if self.unit.is_free() {
            None
        } else {
            let reg = self.unit.get_output_register();
            Some(unsafe { super::register_set::RegisterSet::wrap_ptr(reg) })
        }
    }

    #[must_use]
    pub fn not_ready_mask(&self) -> String {
        self.unit.get_not_ready_mask().to_string()
    }
}

impl<'a> std::ops::Deref for CollectorUnit<'a> {
    type Target = collector_unit_t;

    fn deref(&self) -> &'a Self::Target {
        self.unit
    }
}
