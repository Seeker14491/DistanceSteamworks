use humantime::Duration;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    /// Port to connect to the backend
    #[structopt(short = "p", long = "port")]
    pub port: u16,

    /// How long to wait after an update finishes, before starting the next one
    #[structopt(short = "d", long = "delay")]
    pub update_delay: Duration,

    /// Prevents running an update immediately
    #[structopt(long = "delay-start")]
    pub delay_start: bool,
}

pub fn get() -> Opt {
    Opt::from_args()
}
