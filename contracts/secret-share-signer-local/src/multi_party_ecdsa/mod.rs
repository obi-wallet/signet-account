pub mod local_signature;
pub mod rounds;

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum Error {
    InvalidSig,
}
