mod analyze;
mod cli;
mod db;
mod import;
mod inspect;
mod mapping;
mod models;
mod normalize;
mod query;
mod tax;
mod util;

fn main() -> anyhow::Result<()> {
    cli::run()
}
