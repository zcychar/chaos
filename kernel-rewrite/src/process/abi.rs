#![allow(unused_imports)]

use std::collections::BTreeMap;

/// Initial process stack layout inputs. It describes the init data for a new process to put on its stack when starting execution.
/// Its only function here is to compute the total size of the stack.
///
/// `args`, `envs`, and `auxv` are used to compute where argv/envp/auxv data
/// would be placed below a supplied stack top.
pub struct ProcInit {
    pub args: Vec<String>,
    pub envs: Vec<String>,
    pub auxv: BTreeMap<u8, usize>,
}

impl ProcInit {
    fn reserve_stack_bytes(stack_pointer: &mut usize, byte_count: usize) -> bool {
        match stack_pointer.checked_sub(byte_count) {
            Some(next_stack_pointer) => {
                *stack_pointer = next_stack_pointer;
                true
            }
            None => false,
        }
    }

    pub fn push_at(&self, top: usize) -> usize {
        let word_size = std::mem::size_of::<usize>();
        let mut stack_pointer = top;

        for env in self.envs.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, env.len().saturating_add(1)) {
                return 0;
            }
        }

        for arg in self.args.iter() {
            if !Self::reserve_stack_bytes(&mut stack_pointer, arg.len().saturating_add(1)) {
                return 0;
            }
        }

        let aux_pairs = self.auxv.len();
        let aux_bytes = match aux_pairs
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, aux_bytes) {
            return 0;
        }

        let env_ptrs_bytes = match self
            .envs
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, env_ptrs_bytes) {
            return 0;
        }

        let arg_ptrs_bytes = match self
            .args
            .len()
            .checked_add(1)
            .and_then(|value| value.checked_mul(word_size))
        {
            Some(value) => value,
            None => return 0,
        };
        if !Self::reserve_stack_bytes(&mut stack_pointer, arg_ptrs_bytes) {
            return 0;
        }

        if !Self::reserve_stack_bytes(&mut stack_pointer, word_size) {
            return 0;
        }

        let alignment_offset = stack_pointer & 0xF;
        if alignment_offset != 0 {
            // Debug fix: every downward adjustment uses checked_sub to avoid underflow.
            if !Self::reserve_stack_bytes(&mut stack_pointer, alignment_offset) {
                return 0;
            }
        }
        stack_pointer
    }

    pub fn total_size(&self) -> usize {
        let mut size = 0usize;
        for arg in &self.args {
            size += arg.len() + 1;
        }
        for env in &self.envs {
            size += env.len() + 1;
        }
        size += (self.auxv.len() * 2 + 2 + self.args.len() + 1 + self.envs.len() + 1 + 1)
            * std::mem::size_of::<usize>();
        size
    }
}
