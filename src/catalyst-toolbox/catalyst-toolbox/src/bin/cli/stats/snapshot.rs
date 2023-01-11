use catalyst_toolbox::stats::distribution::Stats;
use catalyst_toolbox::stats::snapshot::read_initials;
use catalyst_toolbox::stats::voters::calculate_wallet_distribution_from_initials;
use color_eyre::Report;
use jormungandr_lib::interfaces::Initial;
use std::path::PathBuf;
use clap::Parser;
#[derive(Parser, Debug)]
pub struct SnapshotCommand {
    #[clap(long = "support-lovelace")]
    pub support_lovelace: bool,
    #[clap(name = "SNAPSHOT")]
    pub snapshot: PathBuf,
    #[clap(long = "threshold")]
    pub threshold: u64,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    Count,
    Ada,
}

impl SnapshotCommand {
    pub fn exec(&self) -> Result<(), Report> {
        let initials: Vec<Initial> = read_initials(jortestkit::file::read_file(&self.snapshot)?)?;

        match self.command {
            Command::Count => calculate_wallet_distribution_from_initials(
                Stats::new(self.threshold)?,
                initials,
                vec![],
                self.support_lovelace,
                |stats, _, _| stats.add(1),
            )?
            .print_count_per_level(),
            Command::Ada => calculate_wallet_distribution_from_initials(
                Stats::new(self.threshold)?,
                initials,
                vec![],
                self.support_lovelace,
                |stats, value, _| stats.add(value),
            )?
            .print_ada_per_level(),
        };

        Ok(())
    }
}
