using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Threading;
using System.Threading.Tasks;
using OneOf;
using Steamworks;
using StreamJsonRpc;

namespace DistanceSteamworksProxy
{
    internal static class Program
    {
        public static AppId_t DistanceAppId;

        private static void Main(string[] args)
        {
            if (args.Length != 1)
            {
                Console.Error.WriteLine("Error: expected one parameter (what port to listen on)");
                Environment.Exit(1);
            }
            
            var port = int.Parse(args[0]);
            Init();

            var rpcThread = new Thread(() =>
            {
                var listener = new TcpListener(IPAddress.Any, port);
                listener.Start();
                while (true)
                {
                    using (var client = listener.AcceptTcpClient())
                    using (var stream = client.GetStream())
                    {
                        try
                        {
                            JsonRpc.Attach(stream, new RpcServer()).Completion.Wait();
                        }
                        catch (AggregateException aex)
                        {
                            aex.Flatten().Handle(ex =>
                            {
                                Console.WriteLine(ex);
                                return true;
                            });
                        }
                    }
                }
            });
            rpcThread.Start();

            while (rpcThread.IsAlive)
            {
                SteamAPI.RunCallbacks();
                Thread.Sleep(10);
            }

            SteamAPI.Shutdown();
        }

        private static void Init()
        {
            DistanceAppId = new AppId_t(233610);

            if (!Packsize.Test())
            {
                Console.Error.WriteLine(
                    "[Steamworks.NET] Packsize Test returned false, the wrong version of Steamworks.NET is being run in this platform.");
                Environment.Exit(1);
            }

            if (!DllCheck.Test())
            {
                Console.Error.WriteLine(
                    "[Steamworks.NET] DllCheck Test returned false, One or more of the Steamworks binaries seems to be the wrong version.");
                Environment.Exit(1);
            }

            try
            {
                if (!SteamAPI.Init())
                {
                    Console.Error.WriteLine(
                        "[Steamworks.NET] SteamAPI_Init() failed. Refer to Valve's documentation for more information.");
                    Environment.Exit(1);
                }
            }
            catch (DllNotFoundException e)
            {
                Console.Error.WriteLine(
                    "[Steamworks.NET] Could not load [lib]steam_api.dll/so/dylib. It's likely not in the correct location.\n" +
                    e);
                Environment.Exit(1);
            }
        }
    }

    internal class RpcServer
    {
        public dynamic GetLeaderboardRange(string leaderboardName, int start, int end)
        {
            return GetLeaderboard(LeaderboardQuery.RunAsync(leaderboardName, (start, end)));
        }

        public dynamic GetLeaderboardPlayers(string leaderboardName, ulong[] players)
        {
            return GetLeaderboard(LeaderboardQuery.RunAsync(leaderboardName, players));
        }

        public dynamic[] GetWorkshopLevels(uint maxResults, string searchText)
        {
            var entries = WorkshopQuery.RunAsync(maxResults, searchText).Result;
            // ReSharper disable once CoVariantArrayConversion
            return entries.Select(entry =>
            {
                var d = entry.Details;
                return new
                {
                    published_file_id = d.m_nPublishedFileId.m_PublishedFileId,
                    steam_id_owner = d.m_ulSteamIDOwner, file_name = d.m_pchFileName,
                    title = d.m_rgchTitle,
                    description = d.m_rgchDescription, time_created = d.m_rtimeCreated,
                    time_updated = d.m_rtimeUpdated, file_size = d.m_nFileSize,
                    votes_up = d.m_unVotesUp,
                    votes_down = d.m_unVotesDown, score = d.m_flScore, tags = d.m_rgchTags.Split(','),
                    author_name = entry.AuthorName, preview_url = entry.PreviewUrl
                };
            }).ToArray();
        }

        public string GetPersonaName(ulong steamId)
        {
            return SteamFriends.GetFriendPersonaName(new CSteamID(steamId));
        }

        private static dynamic GetLeaderboard(Task<(List<LeaderboardQuery.Entry> entries, int total_entries)> task)
        {
            var result = task.Result;
            var entries = result.entries
                .Select(entry =>
                {
                    var entryT = entry.EntryT;
                    return new
                    {
                        steam_id = entryT.m_steamIDUser.m_SteamID, global_rank = entryT.m_nGlobalRank,
                        score = entryT.m_nScore, player_name = entry.PlayerName
                    };
                })
                .ToArray();

            return new {entries, result.total_entries};
        }
    }

    internal class SteamApiException : Exception
    {
        public SteamApiException(string message) : base(message)
        {
        }
    }

    internal class LeaderboardQuery
    {
        public struct Entry
        {
            public LeaderboardEntry_t EntryT;
            public string PlayerName;
        }

        private readonly CallResult<LeaderboardScoresDownloaded_t> _leaderboardScoresDownloaded;

        private readonly TaskCompletionSource<(List<Entry> entries, int total_entries)>
            _taskCompletionSource;

        private OneOf<(int, int), ulong[]> _oneOf;
        private int _totalEntries;

        public static Task<(List<Entry> entries, int total_entries)> RunAsync(string leaderboardName,
            OneOf<(int, int), ulong[]> oneOf)
        {
            var task = new TaskCompletionSource<(List<Entry> entries, int total_entries)>();

            // ReSharper disable once ObjectCreationAsStatement
            new LeaderboardQuery(leaderboardName, oneOf, task);

            return task.Task;
        }

        private LeaderboardQuery(string leaderboardName, OneOf<(int, int), ulong[]> oneOf,
            TaskCompletionSource<(List<Entry> entries, int total_entries)> taskCompletionSource)
        {
            _leaderboardScoresDownloaded =
                CallResult<LeaderboardScoresDownloaded_t>.Create(OnLeaderboardScoresDownloaded);
            _taskCompletionSource = taskCompletionSource;
            _oneOf = oneOf;

            var callResult = SteamUserStats.FindLeaderboard(leaderboardName);
            CallResult<LeaderboardFindResult_t>.Create(OnLeaderboardFindResult).Set(callResult);
        }

        // FIXME: limitation: "A maximum of 100 users can be downloaded at a time" when using 'DownloadLeaderboardEntriesForUsers'
        private void OnLeaderboardFindResult(LeaderboardFindResult_t result, bool ioFailure)
        {
            if (ioFailure)
            {
                _taskCompletionSource.SetException(new IOException());
                return;
            }

            if (result.m_bLeaderboardFound == 0)
            {
                _taskCompletionSource.SetResult((new List<Entry>(), 0));
                return;
            }

            var board = result.m_hSteamLeaderboard;
            _totalEntries = SteamUserStats.GetLeaderboardEntryCount(board);
            var callResult = _oneOf.Match(
                range => SteamUserStats.DownloadLeaderboardEntries(board,
                    ELeaderboardDataRequest.k_ELeaderboardDataRequestGlobal, range.Item1, range.Item2),
                players =>
                {
                    var players2 = players
                        .Select(id => new CSteamID(id))
                        .ToArray();

                    return SteamUserStats.DownloadLeaderboardEntriesForUsers(board, players2, players2.Length);
                }
            );
            _leaderboardScoresDownloaded.Set(callResult);
        }

        private void OnLeaderboardScoresDownloaded(LeaderboardScoresDownloaded_t result, bool ioFailure)
        {
            if (ioFailure)
            {
                _taskCompletionSource.SetException(new IOException());
                return;
            }

            var entriesFromSteam = result.m_hSteamLeaderboardEntries;
            var entriesToReturn = new List<Entry>();
            var numEntriesToReturn = _oneOf.Match(
                range => Math.Min(result.m_cEntryCount, range.Item2 - range.Item1 + 1),
                players => result.m_cEntryCount
            );
            for (var i = 0; i < numEntriesToReturn; i++)
            {
                var result2 =
                    SteamUserStats.GetDownloadedLeaderboardEntry(entriesFromSteam, i, out var entry, new int[0], 0);
                if (!result2)
                {
                    _taskCompletionSource.SetException(
                        new SteamApiException("Error retrieving leaderboard entry data"));
                    return;
                }

                var authorName = SteamFriends.GetFriendPersonaName(new CSteamID(entry.m_steamIDUser.m_SteamID));
                entriesToReturn.Add(new Entry {EntryT = entry, PlayerName = authorName});
            }

            _taskCompletionSource.SetResult((entriesToReturn, _totalEntries));
        }
    }

    internal class WorkshopQuery
    {
        public struct Entry
        {
            public SteamUGCDetails_t Details;
            public string AuthorName;
            public string PreviewUrl;
        }

        private readonly CallResult<SteamUGCQueryCompleted_t> _steamUgcQueryCompleted;
        private readonly TaskCompletionSource<List<Entry>> _taskCompletionSource;
        private readonly List<Entry> _results = new List<Entry>();
        private uint _page;
        private uint _toFetch;
        private readonly string _searchText;

        public static Task<List<Entry>> RunAsync(uint maxResults, string searchText)
        {
            var task = new TaskCompletionSource<List<Entry>>();

            // ReSharper disable once ObjectCreationAsStatement
            new WorkshopQuery(maxResults, searchText, task);

            return task.Task;
        }

        private WorkshopQuery(uint maxResults, string searchText, TaskCompletionSource<List<Entry>> task)
        {
            _steamUgcQueryCompleted = new CallResult<SteamUGCQueryCompleted_t>(OnSteamUgcQueryCompleted);
            _taskCompletionSource = task;
            _page = 1;
            _toFetch = maxResults;
            _searchText = searchText;

            QueryPage();
        }

        private void QueryPage()
        {
            var query = SteamUGC.CreateQueryAllUGCRequest(EUGCQuery.k_EUGCQuery_RankedByPublicationDate,
                EUGCMatchingUGCType.k_EUGCMatchingUGCType_Items_ReadyToUse, Program.DistanceAppId,
                Program.DistanceAppId, _page);
            SteamUGC.SetMatchAnyTag(query, true);
            SteamUGC.AddRequiredTag(query, "Sprint");
            SteamUGC.AddRequiredTag(query, "Challenge");
            SteamUGC.AddRequiredTag(query, "Stunt");
            if (_searchText != null)
            {
                SteamUGC.SetSearchText(query, _searchText);
            }

            _steamUgcQueryCompleted.Set(SteamUGC.SendQueryUGCRequest(query));
        }

        private void OnSteamUgcQueryCompleted(SteamUGCQueryCompleted_t result, bool ioFailure)
        {
            if (ioFailure)
            {
                _taskCompletionSource.SetException(new IOException());
                return;
            }

            var fetchThisRound = Math.Min(_toFetch, result.m_unNumResultsReturned);
            for (uint i = 0; i < fetchThisRound; i++)
            {
                SteamUGC.GetQueryUGCResult(result.m_handle, i, out var details);
                var authorName = SteamFriends.GetFriendPersonaName(new CSteamID(details.m_ulSteamIDOwner));
                SteamUGC.GetQueryUGCPreviewURL(result.m_handle, i, out var previewUrl, 2048);

                _results.Add(new Entry
                {
                    Details = details,
                    AuthorName = authorName,
                    PreviewUrl = previewUrl
                });
            }

            _toFetch -= fetchThisRound;

            if (_results.Count < result.m_unTotalMatchingResults && _toFetch > 0)
            {
                _page += 1;
                QueryPage();
            }
            else
            {
                _taskCompletionSource.SetResult(_results);
            }
        }
    }
}