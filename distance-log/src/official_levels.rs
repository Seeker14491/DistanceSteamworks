use distance_util::{enumflags2::BitFlags, LeaderboardGameMode};

pub fn iter() -> impl Iterator<Item = (&'static str, LeaderboardGameMode)> {
    let all: BitFlags<LeaderboardGameMode> = BitFlags::all();
    all.iter().flat_map(|game_mode| {
        game_mode
            .official_levels()
            .iter()
            .map(move |level| (*level, game_mode))
    })
}
