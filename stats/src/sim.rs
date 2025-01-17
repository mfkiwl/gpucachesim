use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sim {
    pub kernel_name: String,
    pub kernel_name_mangled: String,
    pub kernel_launch_id: usize,
    pub cycles: u64,
    pub instructions: u64,
    pub num_blocks: u64,
    pub elapsed_millis: u128,
    pub is_release_build: bool,
}

impl std::ops::AddAssign for Sim {
    fn add_assign(&mut self, other: Self) {
        self.cycles += other.cycles;
        self.instructions += other.instructions;
        self.num_blocks += other.num_blocks;
        self.elapsed_millis += other.elapsed_millis;
        self.is_release_build |= other.is_release_build;
    }
}
