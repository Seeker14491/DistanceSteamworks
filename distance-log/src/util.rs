use futures::prelude::*;

// Workaround for TryStreamExt's lack of a flatten method:
// https://github.com/rust-lang-nursery/futures-rs/issues/1730
pub fn flatten_try_stream<T, E, S>(
    s: impl Stream<Item = Result<S, E>>,
) -> impl Stream<Item = Result<T, E>>
where
    S: Stream<Item = Result<T, E>>,
{
    s.map(|result| match result {
        Ok(inner_stream) => inner_stream.left_stream(),
        Err(e) => stream::once(future::ready(Err(e))).right_stream(),
    })
    .flatten()
}
