use mangofetch_core::core::registry::PlatformRegistry;
use std::sync::Arc;

pub fn register_platforms(registry: &mut PlatformRegistry) {
    use mangofetch_core::platforms::*;
    registry.register(Arc::new(instagram::InstagramDownloader::new()));
    registry.register(Arc::new(pinterest::PinterestDownloader::new()));
    registry.register(Arc::new(tiktok::TikTokDownloader::new()));
    registry.register(Arc::new(twitter::TwitterDownloader::new()));
    registry.register(Arc::new(twitch::TwitchClipsDownloader::new()));
    registry.register(Arc::new(bluesky::BlueskyDownloader::new()));
    registry.register(Arc::new(reddit::RedditDownloader::new()));
    registry.register(Arc::new(youtube::YouTubeDownloader::new()));
    registry.register(Arc::new(vimeo::VimeoDownloader::new()));
    registry.register(Arc::new(bilibili::BilibiliDownloader::new()));
    let torrent_session = Arc::new(tokio::sync::Mutex::new(None));
    registry.register(Arc::new(magnet::MagnetDownloader::new(torrent_session)));
    registry.register(Arc::new(p2p::P2pDownloader::new()));
    registry.register(Arc::new(generic_ytdlp::GenericYtdlpDownloader::new()));
}
