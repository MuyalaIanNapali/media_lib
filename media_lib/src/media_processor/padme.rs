// use std::io::Write;
//
// use sequoia_openpgp as openpgp;
// use openpgp::serialize::stream::{Message, LiteralWriter};
// use openpgp::serialize::stream::padding::{Padder, padme};
//
// pub(crate) fn pad_message(message: &[u8]) -> openpgp::Result<Vec<u8>> {
//     let mut sink = vec![];
//
//     // Bottom of the stack: write into `sink`.
//     let msg = Message::new(&mut sink);
//
//     // Insert an Encryptor/Signer here if you need one, before the Padder.
//     let msg = Padder::new(msg).with_policy(padme).build()?;
//
//     // Literal data packet wrapping your bytes.
//     let mut msg = LiteralWriter::new(msg).build()?;
//     msg.write_all(message)?;
//
//     // Flushing the stack is what actually emits the padding.
//     msg.finalize()?;
//
//     Ok(sink)
// }