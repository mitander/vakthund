pub mod runtime;

// Re-export the runtime functions so frontends can simply do:
pub use runtime::{
    generate_bug_report, run_fuzz_mode, run_production_mode, run_simulation_mode, save_scenario,
};
