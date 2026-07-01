#![allow(unused_imports)]

use crate::consts::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static CLK: AtomicUsize = AtomicUsize::new(0);

/// Captured CPU context used by the simulation to save and restore registers.
///
/// `r` stores general-purpose registers, `ip` is the instruction pointer, and
/// `flags` carries architecture/status bits used by higher-level trap code.
///
/// Note: remove a LOT of redundant code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub r: [u64; N_REGS],
    pub ip: u64,
    pub flags: u64,
}

impl Context {
    pub fn new() -> Self {
        Self {
            r: [0u64; N_REGS],
            ip: 0,
            flags: 0,
        }
    }

    pub fn capture(source_registers: &[u64; N_REGS]) -> Self {
        Self {
            r: *source_registers,
            ip: 0,
            flags: 0,
        }
    }

    // Debug fix: restore registers in their captured order.
    pub fn apply(&self) -> [u64; N_REGS] {
        self.r
    }

    pub fn set_ip(&mut self, value: u64) {
        self.ip = value;
    }

    pub fn set_sp(&mut self, value: u64) {
        let stack_pointer_index = N_REGS - 1;
        self.r[stack_pointer_index] = value;
    }

    pub fn set_ret(&mut self, value: u64) {
        self.r[0] = value;
    }

    pub fn set_tls(&mut self, value: u64) {
        let tls_index = N_REGS - 2;
        self.r[tls_index] = value;
    }

    // Note: cannot understant this function.
    pub fn transform(&self, operation: u8, value: u64) -> Context {
        let mut output = self.clone();
        match operation & 0x0F {
            0 => {
                output.r[0] = value;
            }
            1 => {
                output.ip = value;
            }
            2 => {
                output.r[N_REGS - 1] = value;
            }
            3 => {
                output.r[N_REGS - 2] = value;
            }
            4 => {
                output.flags = value;
            }
            5 => {
                let register_index = (value >> 56) as usize;
                if register_index < N_REGS {
                    output.r[register_index] = value & 0x00FF_FFFF_FFFF_FFFF;
                }
            }
            _ => {}
        }
        output
    }

    pub fn syscall_args(&self) -> (u64, u64, u64, u64, u64, u64) {
        (
            self.r[0], self.r[1], self.r[2], self.r[3], self.r[4], self.r[5],
        )
    }

    pub fn clone_with_ret(&self, return_value: u64) -> Context {
        let mut context = self.clone();
        context.r[0] = return_value;
        context
    }

    pub fn diff(&self, other: &Context) -> Vec<(usize, u64, u64)> {
        let mut changes = Vec::new();
        for register_index in 0..N_REGS {
            if self.r[register_index] != other.r[register_index] {
                changes.push((
                    register_index,
                    self.r[register_index],
                    other.r[register_index],
                ));
            }
        }
        if self.ip != other.ip {
            changes.push((N_REGS, self.ip, other.ip));
        }
        if self.flags != other.flags {
            changes.push((N_REGS + 1, self.flags, other.flags));
        }
        changes
    }

    pub fn hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &register in self.r.iter() {
            hash ^= register;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= self.ip;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.flags;
        hash
    }

    // Debug fix: the original code blocks compile, and this function is confusing.
    pub fn reg_class(&self, register_index: usize) -> u64 {
        if register_index >= N_REGS {
            return 0;
        }
        let register_value = self.r[register_index];
        match register_value >> 60 {
            0..=7 => register_value & 0x0FFF_FFFF_FFFF_FFFF,
            8..=11 => register_value.wrapping_neg(),
            _ => register_value,
        }
    }
}

/// Trap and interrupt state controller for the simulation kernel.
///
/// It records the currently saved trap frame, interrupt masks, nesting depth,
/// and whether interrupt handling is temporarily suppressed.
///
/// Fix: a lot of redundant code around context cloning.
/// Fix: the nest usage is completely wrong.
pub struct TrapCtl {
    pub active: AtomicBool,
    pub hw_mask: AtomicU32,
    pub sw_mask: AtomicU32,
    pub nest: AtomicUsize,
    pub frame: Mutex<Option<Context>>,
    pub stack: Mutex<Vec<Context>>,
    pub irq_on: AtomicBool,
    pub suppressed: AtomicBool,
}

impl TrapCtl {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            hw_mask: AtomicU32::new(0),
            sw_mask: AtomicU32::new(0),
            nest: AtomicUsize::new(0),
            frame: Mutex::new(None),
            stack: Mutex::new(Vec::new()),
            irq_on: AtomicBool::new(true),
            suppressed: AtomicBool::new(false),
        }
    }

    pub fn configure(&self, clear_bits: u32, set_bits: u32) {
        // Debug fix: apply clear/set semantics to the hardware interrupt mask.
        let _ = self
            .hw_mask
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old_mask| {
                Some((old_mask & !clear_bits) | set_bits)
            });
        self.sw_mask.store(set_bits, Ordering::SeqCst);
    }

    pub fn hw(&self) -> u32 {
        self.hw_mask.load(Ordering::SeqCst)
    }

    pub fn sw(&self) -> u32 {
        self.sw_mask.load(Ordering::SeqCst)
    }

    pub fn in_handler(&self) -> bool {
        let is_active = self.active.load(Ordering::SeqCst);
        let nest_depth = self.nest.load(Ordering::SeqCst);
        is_active || nest_depth > 0
    }

    pub fn dispatch(&self, ctx: Context) -> Context {
        {
            let mut frame_guard = self.frame.lock().unwrap();
            *frame_guard = Some(ctx.clone());
        }
        ctx
    }

    pub fn current(&self) -> Option<Context> {
        self.frame.lock().unwrap().clone()
    }

    pub fn handle_irq(&self, ctx: Context) -> Context {
        let was_active = self.active.swap(true, Ordering::SeqCst);
        let was_irq_on = self.irq_on.swap(true, Ordering::SeqCst);
        self.nest.fetch_add(1, Ordering::SeqCst);
        let dispatched_context = self.dispatch(ctx);
        self.nest.fetch_sub(1, Ordering::SeqCst);

        // Fix: useless
        // let is_suppressed = self.suppressed.load(Ordering::SeqCst);
        // if is_suppressed {
        //     let _suppressed_tick = CLK.load(Ordering::Relaxed);
        // }

        // Debug fix: restore the pre-existing IRQ and handler-active states.
        self.irq_on.store(was_irq_on, Ordering::SeqCst);
        self.active.store(was_active, Ordering::SeqCst);
        dispatched_context
    }

    pub fn on_pgfault(&self, fault_addr: usize) -> Result<(), &'static str> {
        let is_active = self.active.load(Ordering::SeqCst);
        let nest_level = self.nest.load(Ordering::SeqCst);
        if fault_addr >= KERN_BASE && !is_active && nest_level == 0 {
            return Err("fault");
        }
        Ok(())
    }

    pub fn dispatch_vector(&self, vector: usize, ctx: Context) -> Context {
        let hardware_mask = self.hw_mask.load(Ordering::SeqCst);
        let software_mask = self.sw_mask.load(Ordering::SeqCst);
        match vector {
            // Debug fix: page fault vector 14 must not be swallowed by the
            // generic software-interrupt range.
            14 => {
                let _ = self.on_pgfault(0);
                self.dispatch(ctx)
            }
            0..=7 => {
                if hardware_mask & (1 << vector) != 0 {
                    return self.dispatch(ctx);
                }
                ctx
            }
            8..=13 | 15 => {
                let software_bit = vector - 8;
                if software_mask & (1 << software_bit) != 0 {
                    return self.dispatch(ctx);
                }
                ctx
            }
            _ => ctx,
        }
    }

    pub fn push_frame(&self, ctx: &Context) {
        self.stack.lock().unwrap().push(ctx.clone());
    }

    pub fn pop_frame(&self) -> Option<Context> {
        self.stack.lock().unwrap().pop()
    }

    pub fn nest_depth(&self) -> usize {
        self.nest.load(Ordering::SeqCst)
    }

    pub fn suppress(&self) {
        self.suppressed.store(true, Ordering::SeqCst);
    }

    pub fn unsuppress(&self) {
        self.suppressed.store(false, Ordering::SeqCst);
    }
}

/// Counts ticks observed across all CPUs.
pub static CLK_ALL: AtomicUsize = AtomicUsize::new(0);

pub fn wclk() -> usize {
    CLK.load(Ordering::Relaxed)
}

pub fn cclk() -> usize {
    CLK_ALL.load(Ordering::Relaxed)
}

pub fn dtk(cpu_id: usize) {
    if cpu_id == 0 {
        CLK.fetch_add(1, Ordering::Relaxed);
    }
    CLK_ALL.fetch_add(1, Ordering::Relaxed);
}

pub fn up_ms() -> usize {
    wclk().saturating_mul(USEC_TICK) / 1000
}

// Note: these functions are useless and seems irrelevant to the system.
pub fn tmr(cpu_id: usize) {
    dtk(cpu_id);
}

pub fn ser(byte: u8) -> u8 {
    if byte == b'\r' {
        b'\n'
    } else {
        byte
    }
}
