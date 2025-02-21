use crate::error::{Context as _, Result};
use memchr::memchr;
use std::str;

/// Master playlist that lists multiple variant streams of the same content
#[derive(Debug)]
pub struct MultiVariantPlaylist {
	/// These lines define the variant streams.
	/// Each line represents a different version of the same content, encoded at different bitrates and resolutions.
	/// This allows the player to dynamically switch between streams based on the user's network conditions, a feature known as Adaptive Bitrate Streaming (ABR)
	pub variant_streams: Vec<VariantStream>,
	/// These lines provide information about the I-frame streams.
	/// I-frames are keyframes in the video that contain the complete image information.
	/// These streams allow for faster seeking and trick play.
	pub frame_streams: Vec<FrameStream>,
}

#[derive(Debug)]
pub struct VariantStream {
	/// Identifies the program or content.
	pub program_id: Option<u8>,
	/// The average bitrate of the stream in bits per second.
	pub bandwidth: u32,
	/// The resolution of the video (e.g., 1440x1080).
	pub resolution: (u16, u16),
	/// The frame rate of the video.
	pub frame_rate: Option<f32>,
	/// Specifies the codecs used for the audio and video streams.
	pub codecs: Option<String>,
	/// The URI of the m3u8 file containing the media segments for this variant.
	pub uri: String,
}

#[derive(Debug)]
pub struct FrameStream {
	/// The average bitrate of the I-frame stream.
	pub bandwidth: u32,
	/// The resolution of the I-frame stream (e.g., 1440x1080).
	pub resolution: (u16, u16),
	/// Specifies the codecs used for the I-frame stream.
	pub codecs: String,
	/// The URI of the m3u8 file containing the I-frames for this variant.
	pub uri: String,
}

pub fn parse(bytes: &[u8]) -> Result<MultiVariantPlaylist> {
	let mut variant_streams = Vec::new();
	let mut frame_streams = Vec::new();

	let mut position = 0;
	while position < bytes.len() {
		let newline_pos =
			memchr(b'\n', &bytes[position..]).unwrap_or(bytes.len() - position) + position;
		let line = &bytes[position..newline_pos];

		if line.starts_with(b"#EXT-X-STREAM-INF") {
			variant_streams.push(parse_variant_stream(line)?);
		} else if line.ends_with(b".m3u8\r") {
			if let Some(last_stream) = variant_streams.last_mut() {
				last_stream.uri = str::from_utf8(line)?.trim().to_string();
			}
		} else if line.starts_with(b"#EXT-X-I-FRAME-STREAM-INF") {
			frame_streams.push(parse_frame_stream(line)?);
		}

		position = newline_pos + 1;
	}

	Ok(MultiVariantPlaylist {
		variant_streams,
		frame_streams,
	})
}

fn parse_variant_stream(line: &[u8]) -> Result<VariantStream> {
	let mut in_quotes = false;
	let mut attributes = Vec::new();
	let mut current_attribute = Vec::new();

	for &byte in &line[18..] {
		if byte == b'"' {
			in_quotes = !in_quotes;
			current_attribute.push(byte);
		} else if byte == b',' && !in_quotes {
			attributes.push(current_attribute.clone());
			current_attribute.clear();
		} else {
			current_attribute.push(byte);
		}
	}
	attributes.push(current_attribute);

	let mut stream = VariantStream {
		program_id: None,
		bandwidth: 0,
		resolution: (0, 0),
		frame_rate: None,
		codecs: None,
		uri: String::new(),
	};

	for attribute in attributes {
		let mut parts = attribute.splitn(2, |&byte| byte == b'=');
		let key = parts.next().context("Missing key")?;
		let value = parts.next().context("Missing value")?;

		match key {
			b"PROGRAM-ID" => stream.program_id = str::from_utf8(value)?.parse().ok(),
			b"BANDWIDTH" => stream.bandwidth = str::from_utf8(value)?.parse()?,
			b"RESOLUTION" => {
				let mut res_parts = value.trim_ascii().splitn(2, |&byte| byte == b'x');
				stream.resolution.0 =
					str::from_utf8(res_parts.next().context("Missing width")?)?.parse()?;
				stream.resolution.1 =
					str::from_utf8(res_parts.next().context("Missing height")?)?.parse()?;
			}
			b"FRAME-RATE" => stream.frame_rate = str::from_utf8(value)?.parse().ok(),
			b"CODECS" => {
				stream.codecs = Some(str::from_utf8(value)?.replace('\"', "").trim().to_string())
			}
			_ => eprintln!(
				"Unknown attribute: {}",
				str::from_utf8(key).unwrap_or("Invalid UTF-8")
			),
		}
	}

	Ok(stream)
}

fn parse_frame_stream(line: &[u8]) -> Result<FrameStream> {
	let attributes = line[26..].split(|&byte| byte == b',').collect::<Vec<_>>();
	let mut stream = FrameStream {
		bandwidth: 0,
		resolution: (0, 0),
		codecs: String::new(),
		uri: String::new(),
	};

	for attribute in attributes {
		let mut parts = attribute.splitn(2, |&byte| byte == b'=');
		let key = parts.next().context("Missing key")?;
		let value = parts.next().context("Missing value")?;

		match key {
			b"BANDWIDTH" => stream.bandwidth = str::from_utf8(value)?.parse()?,
			b"RESOLUTION" => {
				let mut res_parts = value.trim_ascii().splitn(2, |&byte| byte == b'x');
				stream.resolution.0 =
					str::from_utf8(res_parts.next().context("Missing width")?)?.parse()?;
				stream.resolution.1 =
					str::from_utf8(res_parts.next().context("Missing height")?)?.parse()?;
			}
			b"CODECS" => {
				stream.codecs = str::from_utf8(value)?.replace('\"', "").trim().to_string()
			}
			b"URI" => stream.uri = str::from_utf8(value)?.replace('\"', "").trim().to_string(),
			_ => eprintln!(
				"Unknown attribute: {}",
				str::from_utf8(key).unwrap_or("Invalid UTF-8")
			),
		}
	}

	Ok(stream)
}
