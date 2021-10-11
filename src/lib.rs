#![feature(doc_notable_trait)]
#![feature(trait_alias)]
#![deny(clippy::correctness)]
#![warn(clippy::pedantic)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate serde_json;

pub mod endpoint;
mod error;
pub mod middleware;
mod request;
mod response;
mod router;
mod stack;

pub use self::endpoint::Endpoint;
pub use self::error::ShortError;
pub use self::request::Request;
pub use self::response::Response;
#[doc(inline)]
pub use self::router::RoutePath;
pub use self::stack::Stack;

pub fn http() -> Stack {
    Stack::new()
}
