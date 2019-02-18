#![allow(non_snake_case)]

mod parse;
mod transport;

use self::transport::TcpTransport;
use failure::{Error, SyncFailure};
use futures::Future;
use jsonrpc_client_core::{expand_params, jsonrpc_client, RpcRequest};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::{self, io, net::ToSocketAddrs};

pub type Rpc = LeaderboardService<TcpTransport>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub entries: Box<[LeaderboardEntry]>,
    pub total_entries: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub steam_id: u64,
    pub global_rank: i32,
    pub score: i32,
    pub player_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopResponse {
    pub published_file_id: u64,
    pub steam_id_owner: u64,
    pub file_name: String,
    pub title: String,
    pub description: String,
    pub time_created: u32,
    pub time_updated: u32,
    pub file_size: i32,
    pub votes_up: u32,
    pub votes_down: u32,
    pub score: f32,
    pub tags: Box<[String]>,
    pub author_name: String,
    pub preview_url: String,
}

jsonrpc_client!(pub struct LeaderboardService {
    pub fn GetLeaderboardRange(&mut self, leaderboardName: &str, start: i32, end: i32) -> RpcRequest<LeaderboardResponse>;
    pub fn GetLeaderboardPlayers(&mut self, leaderboardName: &str, players: &[u64]) -> RpcRequest<LeaderboardResponse>;
    pub fn GetWorkshopLevels(&mut self, maxResults: u32, searchText: &str) -> RpcRequest<Box<[WorkshopResponse]>>;
    pub fn GetPersonaName(&mut self, steamId: u64) -> RpcRequest<String>;
});

pub trait RpcRequestExt<T> {
    fn get(self) -> Result<T, Error>;
}

impl<T, E, F> RpcRequestExt<T> for RpcRequest<T, F>
where
    T: DeserializeOwned + Send + 'static,
    E: std::error::Error + Send + 'static,
    F: Future<Item = Vec<u8>, Error = E> + Send + 'static,
{
    fn get(self) -> Result<T, Error> {
        self.call().map_err(|e| SyncFailure::new(e).into())
    }
}

pub fn client_connect<A: ToSocketAddrs + Clone + Send + 'static>(addr: A) -> io::Result<Rpc> {
    Ok(LeaderboardService::new(TcpTransport::connect(addr)?))
}
