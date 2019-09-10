pub mod user_auth;
pub mod connection;

pub trait Service {
    const NAME: &'static str;
}
