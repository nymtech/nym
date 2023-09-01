//! # Block manager
//!
//! Block manager is quite simple. It keeps pending messages in memory and puts all of them into a block
//! at predefined intervals. That's all it does.
//!
//! If the block actually will be broadcast or not is decided by the application. If not, it will produce next block with
//! the same messages plus the new ones.
//!
//! When application shuts down, pending messages are lost.
//!
//! When a block gets accepted by reliable broadcast then Block Manager will remove all messages included in the block from the
//! pending messages queue.
//!
//! # Synchronization and duplicate messages in sequence of blocks
//!
//! When previous block hasn't been accepted yet, then the next block will contain the same messages as the previous one.
//! One way to solve this is that an application itself keeps track of duplicate messages and discards them if necessary.
//!
//! But it seems a reasonable assumption that in general duplicate messages are unwanted. Therefore, Ephemera solves this
//! by dropping previous blocks which get Finalised/Committed after a new block has been created.

pub(crate) mod builder;
pub(crate) mod manager;
pub(crate) mod message_pool;
pub(crate) mod producer;
pub(crate) mod types;
