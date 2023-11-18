use clap::{Parser, Subcommand};
use color_eyre::eyre;
use std::path::PathBuf;
use std::time::Instant;

#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Options {
    /// Input to operate on
    #[arg(value_name = "TRACE_DIR")]
    pub trace_dir: PathBuf,

    /// Stats output file
    #[arg(short = 'o', long = "stats", value_name = "STATS_OUT")]
    pub stats_out_file: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// Use multi-threading
    #[arg(long = "parallel")]
    pub parallel: bool,

    /// Use non-deterministic simulation
    #[arg(long = "non-deterministic")]
    pub non_deterministic: Option<usize>,

    /// Interleave serial part for non-deterministic simulation
    #[arg(long = "interleave-serial")]
    pub interleave_serial: Option<bool>,

    #[clap(long = "cores-per-cluster", help = "cores per cluster")]
    pub cores_per_cluster: Option<usize>,

    #[clap(long = "num-clusters", help = "number of clusters")]
    pub num_clusters: Option<usize>,

    #[clap(
        long = "threads",
        help = "number of threads to use for parallel simulation"
    )]
    pub num_threads: Option<usize>,

    #[clap(long = "mem-only", help = "simulate only memory instructions")]
    pub memory_only: Option<bool>,

    #[clap(long = "fill-l2", help = "fill L2 cache on CUDA memcopy")]
    pub fill_l2: Option<bool>,

    #[clap(long = "flush-l1", help = "flush L1 cache between kernel launches")]
    pub flush_l1: Option<bool>,

    #[clap(long = "flush-l2", help = "flush L2 cache between kernel launches")]
    pub flush_l2: Option<bool>,

    #[clap(long = "accelsim-compat", help = "accelsim compat mode")]
    pub accelsim_compat_mode: Option<bool>,

    #[clap(long = "simulate-clock-domains", help = "simulate clock domains")]
    pub simulate_clock_domains: Option<bool>,

    #[clap(flatten)]
    pub accelsim: gpucachesim::config::accelsim::Config,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    gpucachesim::init_deadlock_detector();

    let start = Instant::now();
    let options = Options::parse();
    #[cfg(debug_assertions)]
    std::env::set_var("RUST_BACKTRACE", "full");

    let log_after_cycle = std::env::var("LOG_AFTER")
        .unwrap_or_default()
        .parse::<u64>()
        .ok();

    if log_after_cycle.is_none() {
        gpucachesim::init_logging();
    }

    let deadlock_check = std::env::var("DEADLOCK_CHECK")
        .unwrap_or_default()
        .to_lowercase()
        == "yes";

    let parallelization = match (
        options.parallel,
        (options.non_deterministic, options.interleave_serial),
    ) {
        (false, _) => gpucachesim::config::Parallelization::Serial,
        #[cfg(feature = "parallel")]
        (true, (None, _)) => gpucachesim::config::Parallelization::Deterministic,
        #[cfg(feature = "parallel")]
        (true, (Some(run_ahead), interleave)) => {
            gpucachesim::config::Parallelization::Nondeterministic {
                run_ahead,
                interleave: interleave.unwrap_or(true),
            }
        }
        #[cfg(not(feature = "parallel"))]
        _ => eyre::bail!(
            "{} was compiled with parallel simulation disabled",
            env!("CARGO_BIN_NAME")
        ),
    };

    let config = gpucachesim::config::GPU {
        num_simt_clusters: options.num_clusters.unwrap_or(28), // 20
        num_cores_per_simt_cluster: options.cores_per_cluster.unwrap_or(1),
        num_schedulers_per_core: 4,                  // 4
        num_memory_controllers: 12,                  // 8
        num_dram_chips_per_memory_controller: 1,     // 1
        num_sub_partitions_per_memory_controller: 2, // 2
        simulate_clock_domains: options.simulate_clock_domains.unwrap_or(false),
        fill_l2_on_memcopy: options.fill_l2.unwrap_or(false),
        flush_l1_cache: options.flush_l1.unwrap_or(true),
        flush_l2_cache: options.flush_l2.unwrap_or(false),
        accelsim_compat: options.accelsim_compat_mode.unwrap_or(false),
        memory_only: options.memory_only.unwrap_or(false),
        parallelization,
        deadlock_check,
        log_after_cycle,
        simulation_threads: options.num_threads,
        ..gpucachesim::config::GPU::default()
    };

    dbg!(&config.memory_only);
    dbg!(&config.num_schedulers_per_core);
    dbg!(&config.num_simt_clusters);
    dbg!(&config.num_cores_per_simt_cluster);
    dbg!(&config.simulate_clock_domains);

    let sim = gpucachesim::accelmain(&options.trace_dir, config)?;
    let stats = sim.stats();

    // save stats to file
    if let Some(stats_out_file) = options.stats_out_file.as_ref() {
        gpucachesim::save_stats_to_file(&stats, stats_out_file)?;
    }

    eprintln!("STATS:\n");
    for (kernel_launch_id, kernel_stats) in stats.as_ref().iter().enumerate() {
        eprintln!(
            "\n ===== kernel launch {kernel_launch_id:<3}: {}  =====\n",
            kernel_stats.sim.kernel_name
        );
        eprintln!("DRAM: {:#?}", &kernel_stats.dram.reduce());
        eprintln!("SIM: {:#?}", &kernel_stats.sim);
        eprintln!("INSTRUCTIONS: {:#?}", &kernel_stats.instructions);
        eprintln!("ACCESSES: {:#?}", &kernel_stats.accesses);
        eprintln!("L1I: {:#?}", &kernel_stats.l1i_stats.reduce());
        eprintln!("L1D: {:#?}", &kernel_stats.l1d_stats.reduce());
        eprintln!("L2D: {:#?}", &kernel_stats.l2d_stats.reduce());
    }
    eprintln!("completed in {:?}", start.elapsed());
    Ok(())
}
