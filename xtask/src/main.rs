mod accelsim;
mod coverage;
#[cfg(feature = "cuda")]
mod cuda;
mod docs;
mod format;
mod purge;
mod trace;
mod util;

use clap::Parser;
use color_eyre::eyre;

#[derive(Parser, Debug, Clone)]
pub enum Command {
    Coverage(coverage::Options),
    Format(format::Options),
    Accelsim(self::accelsim::Options),
    Purge(purge::Options),
    Trace(trace::Options),
    #[cfg(feature = "cuda")]
    Cuda(cuda::Options),
    Docs,
}

#[derive(Parser, Debug, Clone)]
pub struct Options {
    #[clap(subcommand)]
    pub command: Command,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let options = Options::parse();
    dbg!(&options);
    match options.command {
        Command::Coverage(ref opts) => coverage::coverage(opts),
        Command::Format(opts) => format::format(opts),
        Command::Accelsim(opts) => accelsim::run(opts),
        Command::Purge(opts) => purge::run(&opts),
        Command::Trace(opts) => trace::run(&opts),
        #[cfg(feature = "cuda")]
        Command::Cuda(opts) => cuda::run(&opts),
        Command::Docs => docs::docs(),
    }
}
