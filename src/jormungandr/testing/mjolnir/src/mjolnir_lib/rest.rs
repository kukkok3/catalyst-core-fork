use crate::{generators::RestRequestGen, mjolnir_lib::MjolnirError};
use clap::Parser;
use jormungandr_automation::jormungandr::JormungandrRest;
use jortestkit::{
    load::{ConfigurationBuilder, Monitor},
    prelude::{parse_progress_bar_mode_from_str, ProgressBarMode},
};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestLoadCommandError {
    #[error("Client Error")]
    ClientError(#[from] MjolnirError),
}

#[derive(Parser, Debug)]
pub struct RestLoadCommand {
    /// Number of threads
    #[clap(short = 'c', long = "count", default_value = "3")]
    pub count: usize,
    /// Address in format:
    /// http://127.0.0.1:8002/api/
    #[clap(short = 'e', long = "endpoint")]
    pub endpoint: String,

    /// Amount of delay (in milliseconds) between sync attempts
    #[clap(long = "delay", default_value = "50")]
    pub delay: u64,

    /// Amount of delay (in seconds) between sync attempts
    #[clap(short = 'd', long = "duration")]
    pub duration: u64,

    /// Show progress bar
    #[clap(
        long = "progress-bar-mode",
        short = 'b',
        default_value = "Monitor",
        value_parser = parse_progress_bar_mode_from_str
    )]
    progress_bar_mode: ProgressBarMode,

    /// Prints post load measurements
    #[clap(short = 'm', long = "measure")]
    pub measure: bool,
}

impl RestLoadCommand {
    pub fn exec(&self) -> Result<(), RestLoadCommandError> {
        let mut rest_client = JormungandrRest::new(self.endpoint.clone());
        rest_client.disable_logger();
        let mut request_gen = RestRequestGen::new(rest_client);
        request_gen.do_setup(Vec::new()).unwrap();

        let config = ConfigurationBuilder::duration(Duration::from_secs(self.duration))
            .thread_no(self.count)
            .step_delay(Duration::from_millis(self.delay))
            .monitor(self.build_monitor())
            .build();
        let stats = jortestkit::load::start_sync(request_gen, config, "rest load test");
        if self.measure {
            assert!((stats.calculate_passrate() as u32) > 95);
        }
        Ok(())
    }

    fn build_monitor(&self) -> Monitor {
        match self.progress_bar_mode {
            ProgressBarMode::Monitor => Monitor::Progress(100),
            ProgressBarMode::Standard => Monitor::Standard(100),
            ProgressBarMode::None => Monitor::Disabled(10),
        }
    }
}
