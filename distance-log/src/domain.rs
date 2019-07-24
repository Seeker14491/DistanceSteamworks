use crate::steamworks::{LeaderboardResponse, WorkshopResponse};
use chrono::{DateTime, Utc};
use distance_util::LeaderboardGameMode;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelInfo {
    pub name: String,
    pub mode: LeaderboardGameMode,
    pub leaderboard_name: String,
    pub workshop_response: Option<WorkshopResponse>,
    pub leaderboard_response: LeaderboardResponse,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangelistEntry {
    pub map_name: String,
    pub map_author: Option<String>,
    pub map_preview: Option<String>,
    pub mode: String,
    pub new_recordholder: String,
    pub old_recordholder: Option<String>,
    pub record_new: String,
    pub record_old: Option<String>,
    pub workshop_item_id: Option<String>,
    pub steam_id_author: Option<String>,
    pub steam_id_new_recordholder: String,
    pub steam_id_old_recordholder: Option<String>,
    pub fetch_time: String,
}

impl ChangelistEntry {
    pub fn is_likely_a_duplicate_of(&self, other: &Self) -> bool {
        self.map_name == other.map_name
            && self.mode == other.mode
            && self.record_new == other.record_new
            && self.workshop_item_id == other.workshop_item_id
            && self.steam_id_author == other.steam_id_author
            && self.steam_id_new_recordholder == other.steam_id_new_recordholder
    }
}
