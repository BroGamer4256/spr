#![allow(dead_code)]
use binrw::prelude::*;
use binrw::*;
use ddsfile::{Dds, DxgiFormat};
use image::{DynamicImage, EncodableLayout};
use io::{Cursor, SeekFrom};
use std::collections::HashMap;
use std::ops::Deref;

pub mod py;

#[derive(Debug, BinRead)]
struct SprSetReader {
	flags: u32,
	tex_sets: FilePtr32<TexSetReader>,
	tex_sets_count: u32,
	sprite_count: u32,
	#[br(count = sprite_count)]
	sprites: FilePtr32<Vec<SpriteReader>>,
	#[br(count = tex_sets_count)]
	tex_names: FilePtr32<Vec<FilePtr32<NullString>>>,
	#[br(count = sprite_count)]
	sprite_names: FilePtr32<Vec<FilePtr32<NullString>>>,
	#[br(count = sprite_count)]
	sprite_extras: FilePtr32<Vec<(u32, ScreenMode)>>,
}

#[derive(Debug, BinRead)]
#[br(magic = b"TXP\x03")]
struct TexSetReader {
	#[br(parse_with = get_position)]
	position: u32,
	texture_count: u32,
	padding: u32,
	#[br(offset = (position - 4).into(), count = texture_count)]
	textures: Vec<FilePtr32<TexReader>>,
}

#[derive(Debug, BinRead)]
enum TexReader {
	#[br(magic = b"TXP\x04")]
	Tex2d(Tex2dReader),
	#[br(magic = b"TXP\x05")]
	TexCubeMap(TexCubeMapReader),
}

#[derive(Debug, BinRead)]
struct Tex2dReader {
	#[br(parse_with = get_position)]
	position: u32,
	mip_maps: u32,
	mip_levels: u8,
	array_size: u8,
	depth: u8,
	dimensions: u8,
	#[br(args { inner: (mip_levels, position - 4) })]
	#[br(count = array_size)]
	mip_map_array: Vec<TexMipMapArrayReader>,
}

#[derive(Debug, BinRead)]
struct TexCubeMapReader {
	#[br(parse_with = get_position)]
	position: u32,
	mip_maps: u32,
	mip_levels: u8,
	array_size: u8,
	depth: u8,
	dimensions: u8,
	#[br(calc = mip_levels / array_size)]
	mip_levels_adjusted: u8,
	#[br(args { inner: (mip_levels_adjusted, position - 4) })]
	#[br(count = array_size)]
	mip_map_array: Vec<TexMipMapArrayReader>,
}

#[derive(Debug, BinRead)]
#[br(import(mip_levels: u8, position: u32))]
struct TexMipMapArrayReader {
	#[br(count = mip_levels)]
	#[br(offset = position.into())]
	mip_maps: Vec<FilePtr32<TexMipMapReader>>,
}

#[derive(Debug, BinRead)]
#[br(magic = b"TXP\x02")]
struct TexMipMapReader {
	width: i32,
	height: i32,
	format: TextureFormat,
	index: u8,
	array_index: u8,
	padding: u16,
	data_size: u32,
	#[br(count = data_size)]
	data: Vec<u8>,
}

#[derive(Debug, BinRead, Clone)]
#[br(repr = u32)]
enum TextureFormat {
	Unknown = -1,
	A8 = 0,
	RGB8 = 1,
	RGBA8 = 2,
	RGB5 = 3,
	RGB5A1 = 4,
	RGBA4 = 5,
	DXT1 = 6,
	DXT1a = 7,
	DXT3 = 8,
	DXT5 = 9,
	ATI1 = 10,
	ATI2 = 11,
	L8 = 12,
	L8A8 = 13,
	BC7 = 15,
	BC6H = 127,
}

impl TextureFormat {
	fn to_dxgi_format(&self) -> DxgiFormat {
		match self {
			Self::A8 => DxgiFormat::R8_UNorm,
			Self::RGBA8 => DxgiFormat::R8G8B8A8_UNorm,
			Self::DXT1 => DxgiFormat::BC1_UNorm,
			Self::DXT1a => DxgiFormat::BC1_UNorm,
			Self::DXT3 => DxgiFormat::BC2_UNorm_sRGB,
			Self::DXT5 => DxgiFormat::BC3_UNorm,
			Self::ATI1 => DxgiFormat::BC4_UNorm,
			Self::ATI2 => DxgiFormat::BC5_UNorm,
			Self::L8 => DxgiFormat::A8_UNorm,
			Self::L8A8 => DxgiFormat::A8P8,
			Self::BC7 => DxgiFormat::BC7_UNorm,
			Self::BC6H => DxgiFormat::BC6H_UF16,
			_ => DxgiFormat::Unknown,
		}
	}

	fn from_dxgi_format(format: &DxgiFormat) -> Self {
		match format {
			DxgiFormat::R8_UNorm => Self::A8,
			DxgiFormat::R8G8B8A8_UNorm => Self::RGBA8,
			DxgiFormat::BC1_UNorm => Self::DXT1,
			DxgiFormat::BC2_UNorm_sRGB => Self::DXT3,
			DxgiFormat::BC3_UNorm => Self::DXT5,
			DxgiFormat::BC4_UNorm => Self::ATI1,
			DxgiFormat::BC5_UNorm => Self::ATI2,
			DxgiFormat::A8_UNorm => Self::L8,
			DxgiFormat::A8P8 => Self::L8A8,
			DxgiFormat::BC7_UNorm => Self::BC7,
			_ => Self::Unknown,
		}
	}
}

#[derive(Debug, BinRead, BinWrite, Clone, Copy)]
pub struct Vec4 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
	pub w: f32,
}

#[derive(Debug, BinRead)]
struct SpriteReader {
	texture_index: i32,
	rotate: i32,
	texel_region: Vec4,
	pixel_region: Vec4,
}

#[pyo3::prelude::pyclass]
#[derive(Debug, BinRead, Clone, Copy, PartialEq)]
#[br(repr = u32)]
pub enum ScreenMode {
	QVGA = 0,
	VGA = 1,
	SVGA = 2,
	XGA = 3,
	SXGA = 4,
	SXGAPLUS = 5,
	UXGA = 6,
	WVGA = 7,
	WSVGA = 8,
	WXGA = 9,
	WXGA_ = 10,
	WUXGA = 11,
	WQXGA = 12,
	HDTV720 = 13,
	HDTV1080 = 14,
	WQHD = 15,
	HVGA = 16,
	QHD = 17,
	Custom = 18,
}

fn get_position<R: io::Read + io::Seek>(reader: &mut R, _: &ReadOptions, _: ()) -> BinResult<u32> {
	Ok(reader.stream_position()? as u32)
}

#[derive(Debug, Default)]
pub struct SprSet {
	pub name: String,
	flags: u32,
	pub textures: HashMap<String, DynamicImage>,
	pub sprites: HashMap<String, Sprite>,
}

#[derive(Debug, Clone)]
pub struct Sprite {
	pub screen_mode: ScreenMode,
	texel_region: Vec4,
	pub pixel_region: Vec4,
	pub texture_name: String,
	rotate: i32,
}

#[derive(Debug)]
pub enum SpriteError {
	Io(io::Error),
	BinRead(binrw::Error),
	NulError(std::ffi::NulError),
	Dds(ddsfile::Error),
	MissingData,
}

impl From<io::Error> for SpriteError {
	fn from(value: io::Error) -> Self {
		Self::Io(value)
	}
}

impl From<binrw::Error> for SpriteError {
	fn from(value: binrw::Error) -> Self {
		Self::BinRead(value)
	}
}

impl From<std::ffi::NulError> for SpriteError {
	fn from(value: std::ffi::NulError) -> Self {
		Self::NulError(value)
	}
}

impl From<ddsfile::Error> for SpriteError {
	fn from(value: ddsfile::Error) -> Self {
		Self::Dds(value)
	}
}

impl SprSet {
	pub fn from_reader<R: io::Read + io::Seek>(
		reader: &mut R,
		spr_db_set: Option<&diva_db::spr::SprDbSet>,
	) -> Result<Self, SpriteError> {
		let spr_set: SprSetReader = reader.read_ne()?;
		let mut out_sprites = HashMap::with_capacity(spr_set.sprite_count as usize);
		let mut out_textures = HashMap::with_capacity(spr_set.tex_sets_count as usize);

		let (set_name, replacement_spr, replacement_tex) = match spr_db_set {
			Some(spr_db_set) => {
				let mut replacement_spr = spr_db_set.name.clone();
				replacement_spr.push('_');
				(
					spr_db_set.name.clone(),
					replacement_spr.clone(),
					replacement_spr.replace("SPR", "SPRTEX"),
				)
			}
			None => (String::new(), String::new(), String::new()),
		};

		for (i, tex) in spr_set.tex_sets.textures.iter().enumerate() {
			let mut name = spr_set
				.tex_names
				.get(i as usize)
				.ok_or(SpriteError::MissingData)?
				.to_string();
			if name.is_empty() {
				if let Some(spr_db_set) = spr_db_set {
					name = spr_db_set
						.textures
						.iter()
						.find(|tex| tex.1.index as usize == i)
						.ok_or(SpriteError::MissingData)?
						.1
						.name
						.clone()
						.replace(&replacement_tex, "");
				}
			}
			let tex = tex.deref();
			let params = match &tex {
				TexReader::Tex2d(texture) => ddsfile::NewDxgiParams {
					height: texture
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.height as u32,
					width: texture
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.width as u32,
					depth: Some(texture.depth as u32),
					format: texture
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.format
						.to_dxgi_format(),
					mipmap_levels: Some(texture.mip_maps as u32),
					array_layers: Some(texture.array_size as u32),
					caps2: None,
					is_cubemap: false,
					resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
					alpha_mode: ddsfile::AlphaMode::Unknown,
				},
				TexReader::TexCubeMap(cubemap) => ddsfile::NewDxgiParams {
					height: cubemap
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.height as u32,
					width: cubemap
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.width as u32,
					depth: Some(cubemap.depth as u32),
					format: cubemap
						.mip_map_array
						.first()
						.ok_or(SpriteError::MissingData)?
						.mip_maps
						.first()
						.ok_or(SpriteError::MissingData)?
						.format
						.to_dxgi_format(),
					mipmap_levels: Some(cubemap.mip_maps as u32),
					array_layers: Some(cubemap.array_size as u32),
					caps2: Some(ddsfile::Caps2::CUBEMAP),
					is_cubemap: true,
					resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
					alpha_mode: ddsfile::AlphaMode::Unknown,
				},
			};
			let mut dds = Dds::new_dxgi(params)?;
			match &tex {
				TexReader::Tex2d(texture) => {
					for (i, layer) in texture.mip_map_array.iter().enumerate() {
						let dest = dds.get_mut_data(i as u32)?;
						let src = &layer.mip_maps.first().ok_or(SpriteError::MissingData)?.data;
						unsafe {
							std::ptr::copy_nonoverlapping(
								src.as_ptr(),
								dest.as_mut_ptr(),
								src.len(),
							);
						}
					}
				}
				TexReader::TexCubeMap(cubemap) => {
					for (i, layer) in cubemap.mip_map_array.iter().enumerate() {
						let dest = dds.get_mut_data(i as u32)?;
						let src = &layer.mip_maps.first().ok_or(SpriteError::MissingData)?.data;
						unsafe {
							std::ptr::copy_nonoverlapping(
								src.as_ptr(),
								dest.as_mut_ptr(),
								src.len(),
							);
						}
					}
				}
			}

			out_textures.insert(name, dds_to_dynamic(&dds).ok_or(SpriteError::MissingData)?);
		}

		for (i, spr) in spr_set.sprites.iter().enumerate() {
			let mut name = spr_set
				.sprite_names
				.get(i as usize)
				.ok_or(SpriteError::MissingData)?
				.to_string();
			let mut texture_name = spr_set
				.tex_names
				.get(spr.texture_index as usize)
				.ok_or(SpriteError::MissingData)?
				.to_string();
			if name.is_empty() {
				if let Some(spr_db_set) = spr_db_set {
					name = spr_db_set
						.sprites
						.iter()
						.find(|sprite| sprite.1.index as usize == i)
						.ok_or(SpriteError::MissingData)?
						.1
						.name
						.clone()
						.replace(&replacement_spr, "");
				}
			}
			if texture_name.is_empty() {
				if let Some(spr_db_set) = spr_db_set {
					texture_name = spr_db_set
						.textures
						.iter()
						.find(|tex| tex.1.index as usize == spr.texture_index as usize)
						.ok_or(SpriteError::MissingData)?
						.1
						.name
						.clone()
						.replace(&replacement_tex, "");
				}
			}
			out_sprites.insert(
				name,
				Sprite {
					screen_mode: spr_set
						.sprite_extras
						.get(i)
						.ok_or(SpriteError::MissingData)?
						.1,
					pixel_region: spr.pixel_region,
					texel_region: spr.texel_region,
					rotate: spr.rotate,
					texture_name,
				},
			);
		}

		Ok(Self {
			name: set_name,
			flags: spr_set.flags,
			textures: out_textures,
			sprites: out_sprites,
		})
	}

	pub fn read(path: &str, spr_db: Option<&diva_db::spr::SprDb>) -> Option<Self> {
		let filename = std::path::Path::new(path).file_name()?.to_str()?;
		let bytes = std::fs::read(path.clone()).ok()?;
		let mut reader = Cursor::new(bytes);
		match spr_db {
			Some(spr_db) => {
				let (_, spr_db_set) = spr_db
					.sets
					.iter()
					.find(|x| x.1.filename == filename)
					.unzip();
				Some(Self::from_reader(&mut reader, spr_db_set).ok()?)
			}
			None => Some(Self::from_reader(&mut reader, None).ok()?),
		}
	}

	pub fn to_writer<W: io::Write + io::Seek>(self, writer: &mut W) -> Result<(), SpriteError> {
		writer.write_ne(&self.flags)?;
		let tex_ptr_pos = writer.stream_position()?;
		writer.write_ne(&0u32)?;
		writer.write_ne(&(self.textures.len() as u32))?;
		writer.write_ne(&(self.sprites.len() as u32))?;
		let spr_ptr_pos = writer.stream_position()?;
		writer.write_ne(&0u32)?;
		let tex_names_ptr_pos = writer.stream_position()?;
		writer.write_ne(&0u32)?;
		let spr_names_ptr_pos = writer.stream_position()?;
		writer.write_ne(&0u32)?;
		let spr_extra_ptr_pos = writer.stream_position()?;
		writer.write_ne(&0u32)?;

		let mut textures = self.textures.iter().collect::<Vec<_>>();
		textures.sort_by(|(a, _), (b, _)| a.cmp(b));
		let mut sprites = self.sprites.iter().collect::<Vec<_>>();
		sprites.sort_by(|(a, _), (b, _)| a.cmp(b));

		// Textures
		let tex_pos = writer.stream_position()?;
		writer.seek(SeekFrom::Start(tex_ptr_pos))?;
		writer.write_ne(&(tex_pos as u32))?;
		writer.seek(SeekFrom::Start(tex_pos))?;
		writer.write(b"TXP\x03")?;
		writer.write_ne(&(textures.len() as u32))?;
		writer.write_ne(&0u32)?; // Padding
		let mut textures_pos = vec![];
		for _ in textures.iter() {
			textures_pos.push(writer.stream_position()?);
			writer.write_ne(&0u32)?;
		}
		for (i, (_, texture)) in textures.iter().enumerate() {
			let texture = dynamic_to_dds(texture).ok_or(SpriteError::MissingData)?;
			let pos = writer.stream_position()?;
			writer.seek(SeekFrom::Start(textures_pos[i]))?;
			writer.write_ne(&((pos - tex_pos) as u32))?;
			writer.seek(SeekFrom::Start(pos))?;
			let header10 = texture.header10.clone().ok_or(SpriteError::MissingData)?;
			writer.write(b"TXP\x04")?;
			let mip_levels = texture.header.mip_map_count.unwrap_or(1);
			writer.write_ne(&mip_levels)?;
			writer.write_ne(&(mip_levels as u8))?;
			writer.write_ne(&(header10.array_size as u8))?;
			writer.write_ne(&(texture.header.depth.unwrap_or(8) as u8))?;
			writer.write_ne(&0u8)?; // dimensions

			let mut mip_pos = vec![];
			for _ in 0..(header10.array_size) {
				mip_pos.push(writer.stream_position()?);
				writer.write_ne(&0u32)?;
			}
			for i in 0..(header10.array_size) {
				let data_pos = writer.stream_position()?;
				writer.seek(SeekFrom::Start(mip_pos[i as usize]))?;
				writer.write_ne(&((data_pos - pos) as u32))?;
				writer.seek(SeekFrom::Start(data_pos))?;
				writer.write(b"TXP\x02")?;
				writer.write_ne(&texture.get_width())?;
				writer.write_ne(&texture.get_height())?;
				let format = texture.get_dxgi_format().ok_or(SpriteError::MissingData)?;
				writer.write_ne(&(TextureFormat::from_dxgi_format(&format) as u32))?;
				writer.write_ne(&(i as u8))?;
				writer.write_ne(&(i as u8))?;
				writer.write_ne(&0u16)?;
				let data = texture.get_data(i)?;
				writer.write_ne(&(data.len() as u32))?;
				writer.write(data)?;
			}
		}

		// Sprites
		let pos = writer.stream_position()?;
		writer.seek(SeekFrom::Start(spr_ptr_pos))?;
		writer.write_ne(&(pos as u32))?;
		writer.seek(SeekFrom::Start(pos))?;
		for (_, sprite) in sprites.iter() {
			let (index, (_, _)) = textures
				.iter()
				.enumerate()
				.find(|(_, (name, _))| name == &&sprite.texture_name)
				.ok_or(SpriteError::MissingData)?;
			writer.write_ne(&(index as i32))?;
			writer.write_ne(&sprite.rotate)?;
			writer.write_ne(&sprite.texel_region)?;
			writer.write_ne(&sprite.pixel_region)?;
		}

		// Texture names
		let pos = writer.stream_position()?;
		writer.seek(SeekFrom::Start(tex_names_ptr_pos))?;
		writer.write_ne(&(pos as u32))?;
		writer.seek(SeekFrom::Start(pos))?;
		let mut texture_names_locs = vec![];
		for _ in textures.iter() {
			texture_names_locs.push(writer.stream_position()?);
			writer.write_ne(&0u32)?;
		}
		for (i, (name, _)) in textures.iter().enumerate() {
			let pos = writer.stream_position()?;
			writer.seek(SeekFrom::Start(texture_names_locs[i]))?;
			writer.write_ne(&(pos as u32))?;
			writer.seek(SeekFrom::Start(pos))?;
			writer.write(std::ffi::CString::new(name.clone().clone())?.as_bytes_with_nul())?;
		}

		// Sprite names
		let pos = writer.stream_position()?;
		writer.seek(SeekFrom::Start(spr_names_ptr_pos))?;
		writer.write_ne(&(pos as u32))?;
		writer.seek(SeekFrom::Start(pos))?;
		let mut spr_names_locs = vec![];
		for _ in sprites.iter() {
			spr_names_locs.push(writer.stream_position()?);
			writer.write_ne(&0u32)?;
		}
		for (i, (name, _)) in sprites.iter().enumerate() {
			let pos = writer.stream_position()?;
			writer.seek(SeekFrom::Start(spr_names_locs[i]))?;
			writer.write_ne(&(pos as u32))?;
			writer.seek(SeekFrom::Start(pos))?;
			writer.write(std::ffi::CString::new(name.clone().clone())?.as_bytes_with_nul())?;
		}

		// Sprite extras
		let pos = writer.stream_position()?;
		writer.seek(SeekFrom::Start(spr_extra_ptr_pos))?;
		writer.write_ne(&(pos as u32))?;
		writer.seek(SeekFrom::Start(pos))?;
		for (_, sprite) in sprites.iter() {
			writer.write_ne(&0u32)?;
			writer.write_ne(&(sprite.screen_mode as u32))?;
		}

		Ok(())
	}
}

pub fn get_spr_db_set<'a>(
	filename: &str,
	spr_db: &'a diva_db::spr::SprDb,
) -> Option<&'a diva_db::spr::SprDbSet> {
	let (_, set) = spr_db.sets.iter().find(|x| x.1.filename == filename)?;
	Some(set)
}

fn dds_to_dynamic(texture: &Dds) -> Option<image::DynamicImage> {
	let format = match texture.get_dxgi_format()? {
		DxgiFormat::BC1_UNorm => texpresso::Format::Bc1,
		DxgiFormat::BC2_UNorm => texpresso::Format::Bc2,
		DxgiFormat::BC3_UNorm => texpresso::Format::Bc3,
		DxgiFormat::BC4_UNorm => texpresso::Format::Bc4,
		DxgiFormat::BC5_UNorm => texpresso::Format::Bc5,
		_ => return None,
	};
	let mut decompressed =
		vec![0u8; 4 * texture.header.width as usize * texture.header.height as usize];
	format.decompress(
		&texture.data,
		texture.header.width as usize,
		texture.header.height as usize,
		&mut decompressed,
	);
	let buffer =
		image::RgbaImage::from_raw(texture.header.width, texture.header.height, decompressed)?;
	Some(DynamicImage::ImageRgba8(buffer).flipv())
}

/*
fn dynamic_to_dds(texture: &image::DynamicImage) -> Option<Dds> {
	let rgba8 = texture.flipv().to_rgba8();
	let rgba = rgba8.as_bytes();

	let width = texture.width() as usize;
	let height = texture.height() as usize;

	let format = texpresso::Format::Bc3;
	let compressed_size = format.compressed_size(width, height);
	let params = texpresso::Params::default();

	let mut buf = vec![0u8; compressed_size];
	format.compress(&rgba, width, height, params, &mut buf);
	let mut dds = Dds::new_dxgi(ddsfile::NewDxgiParams {
		height: height as u32,
		width: width as u32,
		depth: None,
		format: ddsfile::DxgiFormat::BC3_UNorm,
		mipmap_levels: None,
		array_layers: None,
		caps2: None,
		is_cubemap: false,
		resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
		alpha_mode: ddsfile::AlphaMode::Straight,
	})
	.unwrap();
	dds.data = buf;
	Some(dds)
}
*/

fn dynamic_to_dds(texture: &image::DynamicImage) -> Option<Dds> {
	let rgba8 = texture.flipv().to_rgba8();
	let rgba = rgba8.as_bytes();

	let width = texture.width();
	let height = texture.height();
	let mut dds = Dds::new_dxgi(ddsfile::NewDxgiParams {
		height: height as u32,
		width: width as u32,
		depth: None,
		format: ddsfile::DxgiFormat::R8G8B8A8_UNorm,
		mipmap_levels: None,
		array_layers: None,
		caps2: None,
		is_cubemap: false,
		resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
		alpha_mode: ddsfile::AlphaMode::PreMultiplied,
	})
	.unwrap();
	dds.data = rgba.to_vec();
	Some(dds)
}

pub fn load_sprite_image(texture: image::DynamicImage, sprite: Sprite) -> image::DynamicImage {
	unsafe {
		texture.crop_imm(
			sprite.pixel_region.x.to_int_unchecked(),
			sprite.pixel_region.y.to_int_unchecked(),
			sprite.pixel_region.z.to_int_unchecked(),
			sprite.pixel_region.w.to_int_unchecked(),
		)
	}
}
