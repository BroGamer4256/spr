use crate::*;
use pyo3::{exceptions::*, prelude::*};
use std::{collections::BTreeMap, path::Path};

impl From<SpriteError> for PyErr {
	fn from(value: SpriteError) -> Self {
		match value {
			SpriteError::Io(io_err) => PyErr::new::<PyIOError, _>(io_err.to_string()),
			SpriteError::BinRead(bin_err) => PyErr::new::<PyException, _>(format!("{}", bin_err)),
			SpriteError::NulError(_) => PyErr::new::<PyException, _>("Null in middle of name"),
			SpriteError::MissingData => PyErr::new::<PyException, _>("Failed to parse file"),
			SpriteError::Dds(_) => PyErr::new::<PyException, _>("Failed to parse texture"),
		}
	}
}

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct PyImage {
	pub width: u32,
	pub height: u32,
	pub data: Vec<u8>,
}

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct PySprite {
	#[pyo3(get, set)]
	pub texture: String,
	#[pyo3(get, set)]
	pub x: f32,
	#[pyo3(get, set)]
	pub y: f32,
	#[pyo3(get, set)]
	pub width: f32,
	#[pyo3(get, set)]
	pub height: f32,
	#[pyo3(get, set)]
	pub screen_mode: ScreenMode,
}

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub struct PySprSet {
	#[pyo3(get, set)]
	pub name: String,
	#[pyo3(get, set)]
	pub textures: BTreeMap<String, PyImage>,
	#[pyo3(get, set)]
	pub sprites: BTreeMap<String, PySprite>,
}

#[pymethods]
impl PySprite {
	fn __repr__(&self) -> PyResult<String> {
		Ok(format!(
			"PySprite {}x{} in {} at {}x{}",
			self.height, self.width, self.texture, self.x, self.y
		))
	}

	#[new]
	fn py_new() -> PyResult<Self> {
		Ok(Self {
			texture: String::new(),
			x: 0.0,
			y: 0.0,
			width: 0.0,
			height: 0.0,
			screen_mode: ScreenMode::HDTV1080,
		})
	}
}

#[pymethods]
impl PyImage {
	fn __repr__(&self) -> PyResult<String> {
		Ok(format!("PyImage {}x{}", self.width, self.height))
	}

	#[new]
	pub fn new(path: &str) -> PyResult<Self> {
		let path = Path::new(path);
		if !path.is_file() {
			return Err(PyErr::new::<PyException, _>(format!(
				"{} is not file",
				path.to_string_lossy()
			)));
		}
		let image = match image::io::Reader::open(path)?.decode() {
			Ok(image) => image,
			Err(_) => {
				return Err(PyErr::new::<PyException, _>(format!(
					"Failed to decode image file at {}",
					path.to_string_lossy()
				)));
			}
		};
		if !image.width().is_power_of_two() || !image.height().is_power_of_two() {
			return Err(PyErr::new::<PyException, _>(
				"Image resolution not power of 2",
			));
		}
		let rgba8 = image.to_rgba8();
		Ok(Self {
			width: image.width(),
			height: image.height(),
			data: rgba8.as_bytes().to_vec(),
		})
	}
}

#[pymethods]
impl PySprSet {
	fn __repr__(&self) -> PyResult<String> {
		let mut textures = self
			.textures
			.iter()
			.map(|(name, texture)| {
				Ok::<(String, String), PyErr>((name.clone(), texture.__repr__()?))
			})
			.collect::<Result<Vec<(String, String)>, _>>()?;
		textures.sort_by(|(a, _), (b, _)| a.cmp(b));
		let mut sprites: Vec<(String, String)> = self
			.sprites
			.iter()
			.map(|(name, sprite)| Ok::<(String, String), PyErr>((name.clone(), sprite.__repr__()?)))
			.collect::<Result<Vec<(String, String)>, _>>()?;
		sprites.sort_by(|(a, _), (b, _)| a.cmp(b));
		Ok(format!("PySprSet {textures:?} {sprites:?}",))
	}

	pub fn set_texture(&mut self, texture_name: String, texture: PyImage) -> PyResult<()> {
		self.textures.insert(texture_name, texture);
		Ok(())
	}

	pub fn set_sprite(&mut self, spr_name: String, sprite: PySprite) -> PyResult<()> {
		let Some(texture) = self.textures.get(&sprite.texture) else {
			return Err(PyErr::new::<PyException, _>(format!(
				"Could not find {} in textures",
				sprite.texture
			)));
		};
		if sprite.x + sprite.width > texture.width as f32
			|| sprite.y + sprite.height > texture.height as f32
		{
			return Err(PyErr::new::<PyException, _>("Sprite too large for texture"));
		}
		self.sprites.insert(spr_name, sprite);
		Ok(())
	}

	pub fn save_to_raw(&self) -> PyResult<Vec<u8>> {
		let sprset = py_set_to_set(self)?;
		let mut data = vec![];
		let mut writer = Cursor::new(&mut data);
		sprset.to_writer(&mut writer)?;
		Ok(data)
	}

	pub fn save_to_file(&self, path: &str) -> PyResult<()> {
		let sprset = py_set_to_set(self)?;
		let mut writer = std::fs::File::create(path)?;
		sprset.to_writer(&mut writer)?;
		Ok(())
	}

	#[new]
	fn py_new() -> PyResult<Self> {
		Ok(Self {
			name: String::new(),
			textures: BTreeMap::new(),
			sprites: BTreeMap::new(),
		})
	}
}

fn py_set_to_set(pyset: &PySprSet) -> PyResult<SprSet> {
	Ok(SprSet {
		name: pyset.name.clone(),
		flags: 0,
		textures: pyset
			.textures
			.iter()
			.map(|(name, texture)| {
				let buffer =
					image::RgbaImage::from_raw(texture.width, texture.height, texture.data.clone());
				let buffer = match buffer {
					Some(buffer) => buffer,
					None => {
						dbg!(name);
						return None;
					}
				};
				let image = DynamicImage::ImageRgba8(buffer);
				Some((name.clone(), image))
			})
			.collect::<Option<_>>()
			.ok_or(PyErr::new::<PyException, _>("Failed to create textures"))?,
		sprites: pyset
			.sprites
			.iter()
			.map(|(name, sprite)| {
				(
					name.clone(),
					Sprite {
						screen_mode: sprite.screen_mode,
						texel_region: Vec4 {
							x: 0.0,
							y: 0.0,
							z: 0.0,
							w: 0.0,
						},
						rotate: 0,
						texture_name: sprite.texture.clone(),
						pixel_region: Vec4 {
							x: sprite.x,
							y: sprite.y,
							z: sprite.width,
							w: sprite.height,
						},
					},
				)
			})
			.collect(),
	})
}

fn set_to_py_set(sprset: SprSet) -> PySprSet {
	PySprSet {
		name: sprset.name,
		textures: sprset
			.textures
			.iter()
			.map(|(name, texture)| {
				(
					name.clone(),
					PyImage {
						width: texture.width(),
						height: texture.height(),
						data: texture.as_bytes().to_vec(),
					},
				)
			})
			.collect(),
		sprites: sprset
			.sprites
			.iter()
			.map(|(name, sprite)| {
				(
					name.clone(),
					PySprite {
						texture: sprite.texture_name.clone(),
						x: sprite.pixel_region.x,
						y: sprite.pixel_region.y,
						width: sprite.pixel_region.z,
						height: sprite.pixel_region.w,
						screen_mode: sprite.screen_mode,
					},
				)
			})
			.collect(),
	}
}

#[pyfunction]
fn read_from_raw(data: Vec<u8>) -> PyResult<PySprSet> {
	let mut reader = Cursor::new(data);
	let sprset = SprSet::from_reader(&mut reader, None)?;
	Ok(set_to_py_set(sprset))
}

#[pyfunction]
fn read_from_file(path: &str) -> PyResult<PySprSet> {
	let sprset =
		SprSet::read(path, None).ok_or(PyErr::new::<PyException, _>("Failed to read spr set"))?;
	Ok(set_to_py_set(sprset))
}

#[pymodule]
fn spr(_: Python<'_>, m: &PyModule) -> PyResult<()> {
	m.add_class::<PyImage>()?;
	m.add_class::<PySprite>()?;
	m.add_class::<PySprSet>()?;
	m.add_class::<ScreenMode>()?;
	m.add_function(wrap_pyfunction!(read_from_file, m)?)?;
	m.add_function(wrap_pyfunction!(read_from_raw, m)?)?;

	Ok(())
}
