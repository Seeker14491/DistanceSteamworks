use distance_util::LeaderboardGameMode;

pub fn iter() -> impl Iterator<Item = (&'static str, LeaderboardGameMode)> {
    [
        LeaderboardGameMode::Sprint,
        LeaderboardGameMode::Challenge,
        LeaderboardGameMode::Stunt,
    ]
    .iter()
    .flat_map(|game_mode| {
        game_mode
            .official_level_names()
            .iter()
            .map(move |level| (*level, *game_mode))
    })
}
