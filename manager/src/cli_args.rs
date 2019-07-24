use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Opt {
    // /// TODO
// #[structopt(short = "e", long = "example-arg")]
// pub example_arg: u16,
}

pub fn get() -> Opt {
    Opt::from_args()
}
