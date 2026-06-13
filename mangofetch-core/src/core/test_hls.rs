use crate::core::hls_downloader::HlsDownloader;
use httpmock::prelude::*;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_hls_download_master_playlist() {
    let server = MockServer::start();

    let master_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/master.m3u8");
        then.status(200)
            .header("content-type", "application/vnd.apple.mpegurl")
            .body("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360\n360.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720\n720.m3u8\n");
    });

    // In `download_with_quality`, the HlsDownloader hits the variant playlist URL
    // `fetch_m3u8_with_retry` gets called once on the variant URL. Then `parse_media_playlist` fails
    // (wait, does `select_best_variant` return `720.m3u8` directly without parsing the media playlist?)
    // Actually `download_media_playlist` is called, which calls `fetch_m3u8_with_retry` again! So it requests `/720.m3u8` twice if not using a different branch
    // Let's see what actually happens.

    let _variant_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/720.m3u8");
        then.status(200)
            .header("content-type", "application/vnd.apple.mpegurl")
            .body("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:5\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:5.0,\nseg0.ts\n#EXTINF:5.0,\nseg1.ts\n#EXT-X-ENDLIST\n");
    });

    let seg0_mock = server.mock(|when, then| {
        when.method(GET).path("/seg0.ts");
        then.status(200).body(vec![0; 100]);
    });

    let seg1_mock = server.mock(|when, then| {
        when.method(GET).path("/seg1.ts");
        then.status(200).body(vec![0; 100]);
    });

    let downloader = HlsDownloader::new();
    let cancel_token = CancellationToken::new();

    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("out.ts");

    let res = downloader
        .download_with_quality(
            &server.url("/master.m3u8"),
            out_file.to_str().unwrap(),
            "",
            None,
            cancel_token,
            2,
            3,
            Some(720),
        )
        .await
        .unwrap();

    master_mock.assert();
    seg0_mock.assert();
    seg1_mock.assert();

    assert_eq!(res.segments, 2);
    assert_eq!(res.file_size, 200);
    assert!(out_file.exists());
    assert_eq!(std::fs::metadata(&out_file).unwrap().len(), 200);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_hls_download_cancel() {
    let server = MockServer::start();

    let downloader = HlsDownloader::new();
    let cancel_token = CancellationToken::new();
    cancel_token.cancel(); // Cancel immediately

    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("out_cancel.ts");

    let res = downloader
        .download_with_quality(
            &server.url("/master.m3u8"),
            out_file.to_str().unwrap(),
            "",
            None,
            cancel_token,
            2,
            3,
            Some(720),
        )
        .await;

    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "Download cancelled by user");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_hls_download_direct_variant() {
    let server = MockServer::start();

    let _variant_mock = server.mock(|when, then| {
        when.method(GET)
            .path("/variant.m3u8");
        then.status(200)
            .header("content-type", "application/vnd.apple.mpegurl")
            .body("#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:5\n#EXT-X-MEDIA-SEQUENCE:0\n#EXTINF:5.0,\nseg0.ts\n#EXT-X-ENDLIST\n");
    });

    let seg0_mock = server.mock(|when, then| {
        when.method(GET).path("/seg0.ts");
        then.status(200).body(vec![1; 50]);
    });

    let downloader = HlsDownloader::new();
    let cancel_token = CancellationToken::new();

    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("out_direct.ts");

    let res = downloader
        .download_with_quality(
            &server.url("/variant.m3u8"),
            out_file.to_str().unwrap(),
            "",
            None,
            cancel_token,
            2,
            3,
            Some(720),
        )
        .await
        .unwrap();

    seg0_mock.assert();

    assert_eq!(res.segments, 1);
    assert_eq!(res.file_size, 50);
    assert!(out_file.exists());
    assert_eq!(std::fs::metadata(&out_file).unwrap().len(), 50);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn test_hls_download_invalid_m3u8() {
    let server = MockServer::start();

    let _invalid_mock = server.mock(|when, then| {
        when.method(GET).path("/invalid.m3u8");
        then.status(200)
            .body("This is not a valid m3u8 file at all");
    });

    let downloader = HlsDownloader::new();
    let cancel_token = CancellationToken::new();

    let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("out_invalid.ts");

    let res = downloader
        .download_with_quality(
            &server.url("/invalid.m3u8"),
            out_file.to_str().unwrap(),
            "",
            None,
            cancel_token,
            2,
            3,
            Some(720),
        )
        .await;

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        "Failed to parse m3u8: neither master nor media playlist"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
