use crate::error::{Context as _, Error, Result};
use memchr::memchr;
use std::str;

#[derive(Debug)]
pub struct MediaPlaylist {
	pub version: u8,
	pub media_sequence: u32,
	pub allow_cache: bool,
	pub target_duration: u32,
	pub playlist_type: PlaylistType,
	pub segments: Vec<MediaSegment>,
}

#[derive(Debug)]
pub enum PlaylistType {
	Vod,
	Event,
}

#[derive(Debug)]
pub struct MediaSegment {
	pub duration: f32,
	pub url: String,
}

// TODO: Support i-frame playlists
pub fn parse(bytes: &[u8]) -> Result<MediaPlaylist> {
	let mut version = 0;
	let mut media_sequence = 0;
	let mut allow_cache = true;
	let mut target_duration = 0;
	let mut playlist_type = PlaylistType::Event;
	let mut segments = Vec::new();

	let mut position = 0;
	while position < bytes.len() {
		let newline_pos =
			memchr(b'\n', &bytes[position..]).unwrap_or(bytes.len() - position) + position;
		let line = &bytes[position..newline_pos];

		if line.starts_with(b"#EXTINF") {
			let mut parts = line.split(|&b| b == b':');
			let duration = parts
				.nth(1)
				.context("missing duration")?
				.split(|&b| b == b',')
				.next()
				.context("missing duration")?;
			let duration = str::from_utf8(duration)?.parse()?;

			segments.push(MediaSegment {
				duration,
				url: String::new(),
			});
		} else if line.starts_with(b"https://") {
			if let Some(last_segment) = segments.last_mut() {
				last_segment.url = str::from_utf8(line)?.trim().to_string();
			}
		} else if line.starts_with(b"#EXT-X-VERSION") {
			version = str::from_utf8(&line[15..])?.trim().parse()?;
		} else if line.starts_with(b"#EXT-X-MEDIA-SEQUENCE") {
			media_sequence = str::from_utf8(&line[22..])?.trim().parse()?;
		} else if line.starts_with(b"#EXT-X-ALLOW-CACHE") {
			allow_cache = str::from_utf8(&line[19..])?.trim() == "YES";
		} else if line.starts_with(b"#EXT-X-TARGETDURATION") {
			target_duration = str::from_utf8(&line[22..])?.trim().parse()?;
		} else if line.starts_with(b"#EXT-X-PLAYLIST-TYPE") {
			playlist_type = match str::from_utf8(&line[21..])?.trim() {
				"VOD" => PlaylistType::Vod,
				"EVENT" => PlaylistType::Event,
				_ => return Err(Error::InvalidPlaylistType),
			};
		}

		position = newline_pos + 1;
	}

	Ok(MediaPlaylist {
		version,
		media_sequence,
		allow_cache,
		target_duration,
		playlist_type,
		segments,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse() {
		let variant = include_bytes!("../variant.m3u8");
		let playlist = parse(variant).unwrap();
		println!("{:#?}", playlist);
	}
}
