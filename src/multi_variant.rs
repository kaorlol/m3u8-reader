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
	#[token("#EXT-X-STREAM-INF")]
	StreamInf,
	#[token("#EXT-X-I-FRAME-STREAM-INF")]
	IFrameStreamInf,

	#[token("PROGRAM-ID")]
	ProgramId,
	#[token("BANDWIDTH")]
	Bandwidth,
	#[token("RESOLUTION")]
	Resolution,
	#[token("FRAME-RATE")]
	FrameRate,
	#[token("CODECS")]
	Codecs,
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

	#[regex(r"[0-9]+x[0-9]+", |lex| {
		let mut parts = lex.slice().split('x');
		let width = parts.next().unwrap().parse().unwrap();
		let height = parts.next().unwrap().parse().unwrap();
		(width, height)
	})]
	ResolutionValue((usize, usize)),
	#[regex(r"[a-zA-Z0-9\-_]+\.m3u8")]
	UriValue(&'a str),
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

pub fn parse(input: &str) -> Result<MultiVariantPlaylist> {
	let mut lexer = Token::lexer(input);
	let mut variant_streams = Vec::new();
	let mut frame_streams = Vec::new();

	while let Some(token) = lexer.next() {
		match token? {
			Token::ExtM3U => (),
			Token::StreamInf => {
				variant_streams.push(parse_variant_stream(&mut lexer)?);
			}
			Token::IFrameStreamInf => {
				frame_streams.push(parse_frame_stream(&mut lexer)?);
			}
			_ => (),
		}
	}

	Ok(MultiVariantPlaylist {
		variant_streams,
		frame_streams,
	})
}

fn parse_variant_stream<'a>(lexer: &mut logos::Lexer<'a, Token<'a>>) -> Result<VariantStream> {
	let mut program_id = None;
	let mut bandwidth = 0;
	let mut resolution = (0, 0);
	let mut frame_rate = None;
	let mut codecs = None;
	let mut uri = String::new();

	while let Some(token) = lexer.next() {
		match token? {
			Token::Colon => (),
			Token::Equal => (),
			Token::Comma => (),
			Token::ProgramId => {
				program_id = match lexer.nth(1).context("program id")?? {
					Token::Integer(value) => Some(value as u8),
					_ => bail!("Invalid program id"),
				};
			}
			Token::Bandwidth => {
				bandwidth = match lexer.nth(1).context("bandwidth")?? {
					Token::Integer(value) => value as u32,
					_ => bail!("Invalid bandwidth"),
				};
			}
			Token::Resolution => {
				resolution = match lexer.nth(1).context("resolution")?? {
					Token::ResolutionValue(res) => (res.0 as u16, res.1 as u16),
					_ => bail!("Invalid resolution"),
				};
			}
			Token::FrameRate => {
				frame_rate = match lexer.nth(1).context("frame rate")?? {
					Token::Float(rate) => Some(rate as f32),
					_ => bail!("Invalid frame rate"),
				};
			}
			Token::Codecs => {
				codecs = Some(match lexer.nth(1).context("codecs")?? {
					Token::String(codec) => codec.to_string(),
					_ => bail!("Invalid codecs"),
				});
			}
			Token::UriValue(value) => {
				uri = value.to_string();
				break;
			}
			_ => bail!("Invalid variant stream"),
		}
	}

	Ok(VariantStream {
		program_id,
		bandwidth,
		resolution,
		frame_rate,
		codecs,
		uri,
	})
}

fn parse_frame_stream<'a>(lexer: &mut logos::Lexer<'a, Token<'a>>) -> Result<FrameStream> {
	let mut bandwidth = 0;
	let mut resolution = (0, 0);
	let mut codecs = String::new();
	let mut uri = String::new();

	while let Some(token) = lexer.next() {
		match token? {
			Token::Colon => (),
			Token::Equal => (),
			Token::Comma => (),
			Token::Bandwidth => {
				bandwidth = match lexer.nth(1).context("bandwidth")?? {
					Token::Integer(value) => value as u32,
					_ => bail!("Invalid bandwidth"),
				};
			}
			Token::Resolution => {
				resolution = match lexer.nth(1).context("resolution")?? {
					Token::ResolutionValue(res) => (res.0 as u16, res.1 as u16),
					_ => bail!("Invalid resolution"),
				};
			}
			Token::Codecs => {
				codecs = match lexer.nth(1).context("codecs")?? {
					Token::String(codec) => codec.to_string(),
					_ => bail!("Invalid codecs"),
				};
			}
			Token::Uri => {
				uri = match lexer.nth(1).context("uri")?? {
					Token::String(uri) => uri.to_string(),
					_ => bail!("Invalid uri"),
				};
				break;
			}
			_ => bail!("Invalid frame stream"),
		}
	}

	Ok(FrameStream {
		bandwidth,
		resolution,
		codecs,
		uri,
	})
}

#[test]
fn test_variant_stream_token() {
	let input = "
		#EXTM3U
		#EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=2553505,RESOLUTION=1920x1080,FRAME-RATE=25.000,CODECS=\"avc1.640032,mp4a.40.2\"
		index-f1-v1-a1.m3u8
		#EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=1420969,RESOLUTION=1280x720,FRAME-RATE=25.000,CODECS=\"avc1.64001f,mp4a.40.2\"
		index-f2-v1-a1.m3u8
		#EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=641061,RESOLUTION=640x360,FRAME-RATE=25.000,CODECS=\"avc1.64001e,mp4a.40.2\"
		index-f3-v1-a1.m3u8

		#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=217533,RESOLUTION=1920x1080,CODECS=\"avc1.640032\",URI=\"iframes-f1-v1-a1.m3u8\"
		#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=140609,RESOLUTION=1280x720,CODECS=\"avc1.64001f\",URI=\"iframes-f2-v1-a1.m3u8\"
		#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=58096,RESOLUTION=640x360,CODECS=\"avc1.64001e\",URI=\"iframes-f3-v1-a1.m3u8\"
	";

	let multi_variant_playlist = parse(input).unwrap();
	assert_eq!(
		multi_variant_playlist,
		MultiVariantPlaylist {
			variant_streams: vec![
				VariantStream {
					program_id: Some(1),
					bandwidth: 2553505,
					resolution: (1920, 1080),
					frame_rate: Some(25.0),
					codecs: Some("avc1.640032,mp4a.40.2".to_string()),
					uri: "index-f1-v1-a1.m3u8".to_string(),
				},
				VariantStream {
					program_id: Some(1),
					bandwidth: 1420969,
					resolution: (1280, 720),
					frame_rate: Some(25.0),
					codecs: Some("avc1.64001f,mp4a.40.2".to_string()),
					uri: "index-f2-v1-a1.m3u8".to_string(),
				},
				VariantStream {
					program_id: Some(1),
					bandwidth: 641061,
					resolution: (640, 360),
					frame_rate: Some(25.0),
					codecs: Some("avc1.64001e,mp4a.40.2".to_string()),
					uri: "index-f3-v1-a1.m3u8".to_string(),
				},
			],
			frame_streams: vec![
				FrameStream {
					bandwidth: 217533,
					resolution: (1920, 1080),
					codecs: "avc1.640032".to_string(),
					uri: "iframes-f1-v1-a1.m3u8".to_string(),
				},
				FrameStream {
					bandwidth: 140609,
					resolution: (1280, 720),
					codecs: "avc1.64001f".to_string(),
					uri: "iframes-f2-v1-a1.m3u8".to_string(),
				},
				FrameStream {
					bandwidth: 58096,
					resolution: (640, 360),
					codecs: "avc1.64001e".to_string(),
					uri: "iframes-f3-v1-a1.m3u8".to_string(),
				},
			],
		}
	);
}
