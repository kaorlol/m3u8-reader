use std::ops::Range;

use crate::{
	bail,
	error::{Context as _, Error, Result},
};
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error = String)]
pub enum Token<'a> {
	#[token("#EXTM3U")]
	ExtM3U,
	#[token("#EXT-X-ENDLIST")]
	EndList,

	#[token("#EXT-X-TARGETDURATION")]
	TargetDuration,
	#[token("#EXT-X-VERSION")]
	Version,
	#[token("#EXT-X-MEDIA-SEQUENCE")]
	MediaSequence,
	#[token("#EXT-X-KEY")]
	Key,
	#[token("#EXT-X-ALLOW-CACHE")]
	AllowCache,
	#[token("#EXT-X-PLAYLIST-TYPE")]
	PlaylistType,
	#[token("#EXT-X-I-FRAMES-ONLY")]
	IFramesOnly,

	#[token("#EXTINF")]
	Inf,
	#[token("#EXT-X-BYTERANGE")]
	ByteRange,

	#[token("METHOD")]
	Method,
	#[token("URI")]
	Uri,

	#[token("=")]
	Equal,
	#[token(",")]
	Comma,
	#[token(":")]
	Colon,

	#[regex(r"[0-9]+\.[0-9]+", |lex| lexical::parse(lex.slice()).ok())]
	Float(f64),
	#[regex(r"[0-9]+", |lex| lexical::parse(lex.slice()).ok())]
	Integer(usize),
	#[regex(r#""([^"]*)""#, |lex| lex.slice()[1..lex.slice().len() - 1].as_ref())]
	String(&'a str),

	#[regex(r"AES-128|SAMPLE-AES|NONE", |lex| match lex.slice() {
		"AES-128" => Method::Aes128,
		"SAMPLE-AES" => Method::SampleAes,
		"NONE" => Method::None,
		_ => unreachable!(),
	})]
	MethodValue(Method),
	#[regex(r"YES|NO", |lex| match lex.slice() {
		"YES" => true,
		"NO" => false,
		_ => unreachable!(),
	})]
	AllowCacheValue(bool),
	#[regex(r"[0-9]+@[0-9]+", |lex| {
		let mut parts = lex.slice().split('@');
		let length: usize = lexical::parse(parts.next().unwrap()).unwrap();
		let offset: usize = lexical::parse(parts.next().unwrap()).unwrap();
		length..offset
	})]
	ByteRangeValue(Range<usize>),
	#[regex(r"VOD|EVENT", |lex| match lex.slice() {
		"VOD" => PlaylistType::Vod,
		"EVENT" => PlaylistType::Event,
		_ => unreachable!(),
	})]
	PlaylistTypeValue(PlaylistType),
	#[regex(r"https?://[^ \t\n\f]+", |lex| lex.slice())]
	UriValue(&'a str),
}

#[derive(Debug, PartialEq)]
pub struct MediaPlaylist {
	pub version: u8,
	pub media_sequence: u32,
	pub key: Option<Key>,
	pub allow_cache: bool,
	pub target_duration: u32,
	pub playlist_type: PlaylistType,
	pub iframes_only: bool,
	pub segments: Vec<MediaSegment>,
}

#[derive(Debug, PartialEq)]
pub struct Key {
	pub method: Method,
	pub uri: String,
}

#[derive(Debug, PartialEq)]
pub enum Method {
	Aes128,
	SampleAes,
	None,
}

#[derive(Debug, PartialEq)]
pub enum PlaylistType {
	Vod,
	Event,
}

#[derive(Debug, PartialEq)]
pub struct MediaSegment {
	pub duration: f32,
	pub byte_range: Option<Range<usize>>,
	pub url: String,
}

pub fn parse(input: &str) -> Result<MediaPlaylist> {
	let mut lexer = Token::lexer(input);
	let mut playlist = MediaPlaylist {
		version: 0,
		media_sequence: 0,
		key: None,
		allow_cache: false,
		target_duration: 0,
		playlist_type: PlaylistType::Vod,
		iframes_only: false,
		segments: Vec::new(),
	};

	while let Some(token) = lexer.next() {
		match token? {
			Token::ExtM3U => (),
			Token::Version => {
				playlist.version = match lexer.nth(1).context("Invalid version")?? {
					Token::Integer(version) => version as u8,
					_ => bail!("Invalid version"),
				};
			}
			Token::MediaSequence => {
				playlist.media_sequence = match lexer.nth(1).context("Invalid media sequence")?? {
					Token::Integer(sequence) => sequence as u32,
					_ => bail!("Invalid media sequence"),
				};
			}
			Token::Key => {
				let method = match lexer.nth(3).context("Invalid method")?? {
					Token::MethodValue(method) => method,
					_ => bail!("Invalid method"),
				};
				let uri = match lexer.nth(3).context("Invalid URI")?? {
					Token::String(uri) => uri.to_string(),
					_ => bail!("Invalid key URL"),
				};
				playlist.key = Some(Key { method, uri });
			}
			Token::AllowCache => {
				playlist.allow_cache = match lexer.nth(1).context("Invalid allow cache")?? {
					Token::AllowCacheValue(allow_cache) => allow_cache,
					_ => bail!("Invalid allow cache"),
				};
			}
			Token::TargetDuration => {
				playlist.target_duration =
					match lexer.nth(1).context("Invalid target duration")?? {
						Token::Integer(duration) => duration as u32,
						_ => bail!("Invalid target duration"),
					};
			}
			Token::PlaylistType => {
				playlist.playlist_type = match lexer.nth(1).context("Invalid playlist type")?? {
					Token::PlaylistTypeValue(playlist_type) => playlist_type,
					_ => bail!("Invalid playlist type"),
				};
			}
			Token::IFramesOnly => {
				playlist.iframes_only = true;
			}
			Token::Inf => {
				let duration = match lexer.nth(1).context("Invalid duration")?? {
					Token::Float(duration) => duration as f32,
					_ => bail!("Invalid duration"),
				};

				// let byte_range = match lexer.find(|token| matches!(token, Ok(Token::ByteRange))) {
				// 	Some(Ok(Token::ByteRange)) => {
				// 		lexer.next(); // Consume ByteRange token
				// 		match lexer.next().context("Invalid byte range value")?? {
				// 			Token::ByteRangeValue(range) => Some(range),
				// 			_ => None,
				// 		}
				// 	}
				// 	_ => None,
				// };

				let byte_range = playlist
					.iframes_only
					.then(|| match lexer.nth(3) {
						Some(Ok(Token::ByteRangeValue(range))) => Some(range),
						_ => None,
					})
					.flatten();

				let url_advance = if byte_range.is_some() { 0 } else { 1 };
				let url = match lexer.nth(url_advance).context("Invalid URL")?? {
					Token::UriValue(uri) => uri.to_string(),
					_ => bail!("Invalid URL"),
				};
				playlist.segments.push(MediaSegment {
					duration,
					byte_range,
					url,
				});
			}
			Token::EndList => break,
			_ => (),
		}
	}

	Ok(playlist)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_media_playlist() {
		let input = r#"
			#EXTM3U
			#EXT-X-TARGETDURATION:17
			#EXT-X-ALLOW-CACHE:YES
			#EXT-X-PLAYLIST-TYPE:VOD
			#EXT-X-KEY:METHOD=AES-128,URI="https://example.com/mon.key"
			#EXT-X-VERSION:3
			#EXT-X-MEDIA-SEQUENCE:1
			#EXTINF:6.006,
			https://example.com/segment-1.ts
			#EXTINF:4.588,
			https://example.com/segment-2.ts
			#EXT-X-ENDLIST
		"#;

		let media_playlist = parse(input).unwrap();
		assert_eq!(
			media_playlist,
			MediaPlaylist {
				version: 3,
				media_sequence: 1,
				key: Some(Key {
					method: Method::Aes128,
					uri: "https://example.com/mon.key".to_string(),
				}),
				allow_cache: true,
				target_duration: 17,
				playlist_type: PlaylistType::Vod,
				iframes_only: false,
				segments: vec![
					MediaSegment {
						duration: 6.006,
						byte_range: None,
						url: "https://example.com/segment-1.ts".to_string(),
					},
					MediaSegment {
						duration: 4.588,
						byte_range: None,
						url: "https://example.com/segment-2.ts".to_string(),
					},
				],
			}
		)
	}

	#[allow(clippy::reversed_empty_ranges)]
	#[test]
	fn test_media_playlist_iframes() {
		let input = r#"
			#EXTM3U
			#EXT-X-TARGETDURATION:3
			#EXT-X-VERSION:4
			#EXT-X-MEDIA-SEQUENCE:1
			#EXT-X-PLAYLIST-TYPE:VOD
			#EXT-X-I-FRAMES-ONLY
			#EXTINF:1.120,
			#EXT-X-BYTERANGE:1316@376
			https://example.com/segment-1.ts
			#EXTINF:6.720,
			#EXT-X-BYTERANGE:44744@7896
			https://example.com/segment-2.ts
			#EXT-X-ENDLIST
		"#;

		let media_playlist = parse(input).unwrap();
		assert_eq!(
			media_playlist,
			MediaPlaylist {
				version: 4,
				media_sequence: 1,
				key: None,
				allow_cache: false,
				target_duration: 3,
				playlist_type: PlaylistType::Vod,
				iframes_only: true,
				segments: vec![
					MediaSegment {
						duration: 1.12,
						byte_range: Some(1316..376),
						url: "https://example.com/segment-1.ts".to_string(),
					},
					MediaSegment {
						duration: 6.72,
						byte_range: Some(44744..7896),
						url: "https://example.com/segment-2.ts".to_string(),
					},
				],
			}
		)
	}
}
