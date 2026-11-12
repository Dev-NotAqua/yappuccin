//! Media detection: what a media player (YouTube Music, Spotify, browsers)
//! is playing, published as a Discord "Listening" activity.
//!
//! On Windows this reads the foreground window's title, which for YouTube
//! Music and Spotify carries the track. It is a lightweight fallback that
//! needs no COM/SMTC plumbing and works for every player that names its
//! window, including browsers playing YouTube Music.

use crate::api::Api;
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch;

pub(crate) fn start(api: Arc<Api>) -> watch::Receiver<Option<Arc<Value>>> {
    let (shown, receiver) = watch::channel(None);
    tokio::spawn(async move {
        let mut last: Option<String> = None;
        let mut interval = tokio::time::interval(Duration::from_secs(3));
        loop {
            tokio::select! {
                () = shown.closed() => return,
                _ = interval.tick() => {}
            }
            let title = tokio::task::spawn_blocking(foreground_title).await.unwrap_or(None);
            let activity = title.and_then(|t| parse(&t));
            if let Some(act) = activity {
                let key = act.to_string();
                if last.as_deref() != Some(&key) {
                    last = Some(key);
                    shown.send_replace(Some(Arc::new(act)));
                }
            } else if last.is_some() {
                last = None;
                shown.send_replace(None);
            }
        }
    });
    let _ = api;
    receiver
}

fn foreground_title() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.is_null() { return None; }
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if n <= 0 { return None; }
            Some(String::from_utf16_lossy(&buf[..n as usize]))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

fn parse(title: &str) -> Option<Value> {
    let lower = title.to_lowercase();
    if lower.contains("youtube music") {
        let track = title
            .split(" - YouTube Music")
            .next()
            .unwrap_or(title)
            .trim();
        if track.is_empty() || track.to_lowercase() == "youtube music" {
            return None;
        }
        return Some(json!({"type":2,"name":"YouTube Music","details":track}));
    }
    if lower.starts_with("spotify") {
        let rest = title.trim_start_matches("Spotify").trim_start_matches(" - ").trim();
        if rest.is_empty() || rest.to_lowercase() == "spotify" {
            return None;
        }
        return Some(json!({"type":2,"name":"Spotify","details":rest}));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_youtube_music_title() {
        let v = parse("Bohemian Rhapsody - YouTube Music").unwrap();
        assert_eq!(v["type"], 2);
        assert_eq!(v["name"], "YouTube Music");
        assert_eq!(v["details"], "Bohemian Rhapsody");
    }
    #[test]
    fn ignores_bare_youtube_music() {
        assert!(parse("YouTube Music").is_none());
    }
}
