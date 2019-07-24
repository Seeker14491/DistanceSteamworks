use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {}

pub fn get() -> Opt {
    Opt::from_args()
}
