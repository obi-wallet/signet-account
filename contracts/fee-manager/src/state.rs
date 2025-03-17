use secret_toolkit::storage::{Item, Keymap};

#[allow(clippy::declare_interior_mutable_const)]
pub const FEE_DIVISORS: Keymap<String, u64> = Keymap::new(b"fee_divisors");
#[allow(clippy::declare_interior_mutable_const)]
pub const FEE_PAY_ADDRESSES: Keymap<String, String> = Keymap::new(b"fee_pay_address");
pub const LAST_UPDATE: Item<u64> = Item::new(b"last_update");
