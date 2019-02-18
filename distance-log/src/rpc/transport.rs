use crate::rpc::parse;
use crossbeam_channel::Sender;
use futures::{sync::oneshot, Future};
use jsonrpc_client_core::Transport;
use log::Level;
use std::{
    io::{self, Read, Write},
    mem,
    net::{TcpStream, ToSocketAddrs},
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct TcpTransport {
    req_tx: Sender<(Vec<u8>, self::oneshot::Sender<Vec<u8>>)>,
    id: u64,
}

impl TcpTransport {
    pub fn connect<A: ToSocketAddrs + Clone + Send + 'static>(addr: A) -> io::Result<Self> {
        fn new_tcp_stream(addr: impl ToSocketAddrs + Clone) -> TcpStream {
            loop {
                let result: io::Result<TcpStream> = try {
                    let stream = TcpStream::connect(addr.clone())?;
                    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
                    stream
                };

                if let Ok(x) = result {
                    return x;
                } else {
                    thread::sleep(Duration::from_secs(10));
                }
            }
        }

        let (req_tx, req_rx) =
            crossbeam_channel::unbounded::<(Vec<u8>, oneshot::Sender<Vec<u8>>)>();

        thread::spawn(move || {
            let mut stream = new_tcp_stream(addr.clone());
            let reconnect = |stream: &mut TcpStream| {
                take_mut::take(stream, |x| {
                    mem::drop(x);
                    new_tcp_stream(addr.clone())
                });
            };
            while let Ok((req, resp_tx)) = req_rx.recv() {
                loop {
                    if stream.write_all(&req).is_err() {
                        reconnect(&mut stream);
                        continue;
                    }

                    let content_length = match parse::parse_json_rpc_header(&mut stream) {
                        Ok(len) => len,
                        Err(_) => {
                            reconnect(&mut stream);
                            continue;
                        }
                    };

                    let mut received_body = vec![0; content_length];
                    match stream.read_exact(&mut received_body) {
                        Ok(_) => {
                            resp_tx.send(received_body).unwrap();
                            break;
                        }
                        Err(_) => {
                            reconnect(&mut stream);
                            continue;
                        }
                    }
                }
            }
        });

        Ok(Self { req_tx, id: 0 })
    }
}

impl Transport for TcpTransport {
    type Future = Box<dyn Future<Item = Vec<u8>, Error = Self::Error> + Send>;
    type Error = oneshot::Canceled;

    fn get_next_id(&mut self) -> u64 {
        self.id = self.id.wrapping_add(1);
        self.id
    }

    fn send(&self, json_data: Vec<u8>) -> Self::Future {
        let data = {
            if log_enabled!(Level::Trace) {
                let d = String::from_utf8_lossy(&json_data);
                trace!("sending json data: {}", d)
            }
            let header = format!("Content-Length: {}\r\n\r\n", json_data.len());
            let header = header.as_bytes();

            let mut data = Vec::with_capacity(header.len() + json_data.len());
            data.extend_from_slice(header);
            data.extend_from_slice(&json_data);

            data
        };

        let (resp_tx, resp_rx) = oneshot::channel();
        self.req_tx.send((data, resp_tx)).unwrap();

        Box::new(resp_rx)
    }
}
