use combine::parser::{
    byte::digit,
    item::tokens2,
    repeat::{many1, skip_many},
};

use combine::{
    easy,
    error::StreamError,
    stream::{buffered::BufferedStream, state::State, ReadStream, StreamErrorFor},
    ParseError, Parser, Stream,
};
use std::{io::Read, str};

// Returns the value of "Content-Length", the size of the message body
pub fn parse_json_rpc_header<R>(data: R) -> Result<usize, easy::Errors<u8, u8, usize>>
where
    R: Read,
{
    let stream = BufferedStream::new(easy::Stream(State::new(ReadStream::new(data))), 1);
    parser().parse(stream).map(|(x, _)| x)
}

fn parser<I>() -> impl Parser<Input = I, Output = usize>
where
    I: Stream<Item = u8>,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let range = |s| tokens2(|&l, r| l == r, s);

    let content_length =
        range(&b"Content-Length: "[..]).with(many1(digit()).and_then(|digits: Vec<u8>| {
            str::from_utf8(&digits)
                .unwrap()
                .parse::<usize>()
                .map_err(StreamErrorFor::<I>::other)
        }));

    (
        skip_many(range(&b"\r\n"[..])),
        content_length,
        range(&b"\r\n\r\n"[..]).map(|_| ()),
    )
        .map(|(_, message_length, _)| message_length)
}
