use env_logger::Builder;
use log::LevelFilter;

pub fn setup_env() {
    dotenvy::dotenv().ok();
    Builder::new().filter_level(LevelFilter::Info).init();
}

pub fn map<I, F, B, C, O>(iter: I, f: F) -> O
where
    I: IntoIterator<Item = B>,
    F: FnMut(B) -> C,
    O: FromIterator<C>,
{
    iter.into_iter().map(f).collect()
}
