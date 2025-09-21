use std::io::{Read, Write, Cursor, Seek, SeekFrom};
use std::path::Path;
use std::fs::File;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use image::{ImageBuffer, Rgba, DynamicImage};
use texpresso::{Format, Algorithm, Params};
use minilzo_rs::LZO;


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaaType {
    DXT1 = 0xFF01,
    DXT3 = 0xFF03,
    DXT5 = 0xFF05,
    ARGB4444 = 0x4444,
    ARGB1555 = 0x1555,
    AI88 = 0x8080,
}

impl PaaType {
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            0xFF01 => Some(PaaType::DXT1),
            0xFF03 => Some(PaaType::DXT3),
            0xFF05 => Some(PaaType::DXT5),
            0x4444 => Some(PaaType::ARGB4444),
            0x1555 => Some(PaaType::ARGB1555),
            0x8080 => Some(PaaType::AI88),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CompressionType {
    None = 0,
    LZO = 2,
}

#[derive(Debug)]
struct Tagg {
    name: [u8; 4],
    data_size: u32,
    data: Vec<u8>,
}

#[derive(Debug)]
struct MipMap {
    width: u16,
    height: u16,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Paa {
    pub paa_type: PaaType,
    taggs: Vec<Tagg>,
    mipmaps: Vec<MipMap>,
}

impl Paa {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        Self::from_reader(&mut file)
    }

    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, std::io::Error> {
        let paa_type_raw = reader.read_u16::<LittleEndian>()?;
        let paa_type = PaaType::from_u16(paa_type_raw)
            .ok_or_else(|| error!("Unknown PAA type: 0x{:04X}", paa_type_raw))?;

        let mut taggs = Vec::new();
        loop {
            let mut tagg_name = [0u8; 4];
            match reader.read_exact(&mut tagg_name) {
                Ok(_) => {},
                Err(_) => break,  // End of TAGGs
            }

            // Check if this is a TAGG (might be stored as GGAT reversed)
            if &tagg_name != b"GGAT" && &tagg_name != b"TAGG" {
                reader.seek(SeekFrom::Current(-4))?;
                break;
            }

            let mut tagg_sig = [0u8; 4];
            reader.read_exact(&mut tagg_sig)?;
            let data_size = reader.read_u32::<LittleEndian>()?;

            let mut data = vec![0u8; data_size as usize];
            reader.read_exact(&mut data)?;

            taggs.push(Tagg {
                name: tagg_sig,
                data_size,
                data,
            });
        }

        let mut offset_table = Vec::new();
        for tagg in &taggs {
            if &tagg.name == b"SFFO" {
                let mut cursor = Cursor::new(&tagg.data);
                let num_mipmaps = tagg.data_size / 4;
                for _ in 0..num_mipmaps {
                    offset_table.push(cursor.read_u32::<LittleEndian>()?);
                }
                break;
            }
        }

        let mut mipmaps = Vec::new();
        if !offset_table.is_empty() {
            for i in 0..offset_table.len() {
                if offset_table[i] == 0 {
                    continue;  // Skip mipmaps with null offset
                }
                reader.seek(SeekFrom::Start(offset_table[i] as u64))?;

                let width_raw = reader.read_u16::<LittleEndian>()?;
                let height_raw = reader.read_u16::<LittleEndian>()?;

                // The high bit (0x8000) in width appears to be a compression flag
                // Mask it off to get the actual width
                let width = width_raw & 0x7FFF;
                let height = height_raw;
                // Read 24-bit size (3 bytes)
                let mut size_bytes = [0u8; 3];
                reader.read_exact(&mut size_bytes)?;
                let size = u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], 0]);

                let actual_size = (size & 0xFFFFFF) as usize;
                let expected_uncompressed = calculate_mipmap_size(paa_type, width, height);

                // Check if data appears to be compressed
                // The high bit in width (0x8000) or size field (0x800000) indicates LZO compression
                let compression_type = if (width_raw & 0x8000) != 0 || (size & 0x800000) != 0 || actual_size < expected_uncompressed / 2 {
                    CompressionType::LZO
                } else {
                    CompressionType::None
                };

                let mut data = vec![0u8; actual_size];
                reader.read_exact(&mut data)?;

                if compression_type == CompressionType::LZO {
                    let decompressed_size = calculate_mipmap_size(paa_type, width, height);

                    let lzo = LZO::init().map_err(|_| error!("LZO init failed"))?;
                    let result = lzo.decompress_safe(&data[..], decompressed_size);
                    match result {
                        Ok(decompressed_data) => {
                            data = decompressed_data;
                        },
                        Err(_) => return Err(error!("LZO decompression failed"))
                    }
                }

                mipmaps.push(MipMap {
                    width,
                    height,
                    data,
                });
            }
        }

        Ok(Paa {
            paa_type,
            taggs,
            mipmaps,
        })
    }

    pub fn to_image(&self) -> Result<DynamicImage, std::io::Error> {
        if self.mipmaps.is_empty() {
            return Err(error!("No mipmaps found in PAA file"));
        }

        let mipmap = &self.mipmaps[0];
        let width = mipmap.width as u32;
        let height = mipmap.height as u32;

        let mut rgba_data = vec![0u8; (width * height * 4) as usize];

        match self.paa_type {
            PaaType::DXT1 => {
                let expected_size = calculate_mipmap_size(self.paa_type, mipmap.width, mipmap.height);
                if mipmap.data.len() != expected_size {
                    eprintln!("Warning: DXT1 data size mismatch. Expected {} bytes, got {} bytes", expected_size, mipmap.data.len());
                }
                let format = Format::Bc1;
                format.decompress(&mipmap.data, width as usize, height as usize, &mut rgba_data);
            },
            PaaType::DXT5 => {
                let expected_size = calculate_mipmap_size(self.paa_type, mipmap.width, mipmap.height);
                if mipmap.data.len() != expected_size {
                    eprintln!("Warning: DXT5 data size mismatch. Expected {} bytes, got {} bytes", expected_size, mipmap.data.len());
                }
                let format = Format::Bc3;
                format.decompress(&mipmap.data, width as usize, height as usize, &mut rgba_data);
            },
            _ => {
                return Err(error!("Unsupported PAA type for conversion: {:?}", self.paa_type));
            }
        }

        let img_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, rgba_data)
            .ok_or_else(|| error!("Failed to create image buffer"))?;

        Ok(DynamicImage::ImageRgba8(img_buffer))
    }

    pub fn from_image(img: &DynamicImage, paa_type: PaaType, use_compression: bool) -> Result<Self, std::io::Error> {
        let rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();

        let mut taggs = Vec::new();

        let avg_color = calculate_average_color(&rgba);
        taggs.push(Tagg {
            name: *b"CGVA",
            data_size: 4,
            data: avg_color.to_vec(),
        });

        let max_color = calculate_maximum_color(&rgba);
        taggs.push(Tagg {
            name: *b"CXAM",
            data_size: 4,
            data: max_color.to_vec(),
        });

        let mut mipmaps = Vec::new();
        let mut current_img = img.clone();
        let mut mipmap_width = width;
        let mut mipmap_height = height;

        while mipmap_width >= 1 && mipmap_height >= 1 && mipmaps.len() < 15 {
            let rgba = current_img.to_rgba8();

            let compressed_data = match paa_type {
                PaaType::DXT1 => compress_dxt1(&rgba)?,
                PaaType::DXT5 => compress_dxt5(&rgba)?,
                _ => return Err(error!("Unsupported PAA type: {:?}", paa_type)),
            };

            let mut final_data = compressed_data.clone();

            if use_compression && compressed_data.len() > 128 {
                let mut lzo = match LZO::init() {
                    Ok(lzo) => lzo,
                    Err(_) => continue,  // Skip compression if LZO init fails
                };
                let result = lzo.compress(&compressed_data[..]);
                if let Ok(lzo_compressed) = result {
                    if lzo_compressed.len() < compressed_data.len() {
                        final_data = lzo_compressed;
                    }
                }
            }

            mipmaps.push(MipMap {
                width: mipmap_width as u16,
                height: mipmap_height as u16,
                data: final_data,
            });

            if mipmap_width == 1 && mipmap_height == 1 {
                break;
            }

            mipmap_width = (mipmap_width / 2).max(1);
            mipmap_height = (mipmap_height / 2).max(1);

            if mipmap_width >= 4 || mipmap_height >= 4 {
                current_img = current_img.resize_exact(
                    mipmap_width,
                    mipmap_height,
                    image::imageops::FilterType::Lanczos3
                );
            }
        }

        Ok(Paa {
            paa_type,
            taggs,
            mipmaps,
        })
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_u16::<LittleEndian>(self.paa_type as u16)?;

        for tagg in &self.taggs {
            writer.write_all(b"TAGG")?;
            writer.write_all(&tagg.name)?;
            writer.write_u32::<LittleEndian>(tagg.data_size)?;
            writer.write_all(&tagg.data)?;
        }

        let _offset_tagg_pos = writer.seek(SeekFrom::Current(0))?;
        writer.write_all(b"TAGG")?;
        writer.write_all(b"SFFO")?;
        writer.write_u32::<LittleEndian>((self.mipmaps.len() * 4) as u32)?;

        let mut offset_positions = Vec::new();
        for _ in 0..self.mipmaps.len() {
            offset_positions.push(writer.seek(SeekFrom::Current(0))?);
            writer.write_u32::<LittleEndian>(0)?;
        }

        let mut offsets = Vec::new();
        for mipmap in &self.mipmaps {
            offsets.push(writer.seek(SeekFrom::Current(0))? as u32);

            writer.write_u16::<LittleEndian>(mipmap.width)?;
            writer.write_u16::<LittleEndian>(mipmap.height)?;

            let is_compressed = mipmap.data.len() < calculate_mipmap_size(self.paa_type, mipmap.width, mipmap.height);
            let size = if is_compressed {
                0x80000000 | mipmap.data.len() as u32
            } else {
                mipmap.data.len() as u32
            };
            // Write 24-bit size (3 bytes)
            let size_bytes = size.to_le_bytes();
            writer.write_all(&size_bytes[0..3])?;
            writer.write_all(&mipmap.data)?;
        }

        for (i, offset_pos) in offset_positions.iter().enumerate() {
            writer.seek(SeekFrom::Start(*offset_pos))?;
            writer.write_u32::<LittleEndian>(offsets[i])?;
        }

        Ok(())
    }

    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        self.write(&mut file)
    }
}

fn calculate_mipmap_size(paa_type: PaaType, width: u16, height: u16) -> usize {
    let blocks_x = ((width + 3) / 4) as usize;
    let blocks_y = ((height + 3) / 4) as usize;

    match paa_type {
        PaaType::DXT1 => blocks_x * blocks_y * 8,
        PaaType::DXT5 => blocks_x * blocks_y * 16,
        _ => (width as usize) * (height as usize) * 4,
    }
}

fn compress_dxt1(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>, std::io::Error> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;

    let mut output = vec![0u8; blocks_x * blocks_y * 8];

    let format = Format::Bc1;
    let params = Params {
        algorithm: Algorithm::IterativeClusterFit,
        weights: [1.0, 1.0, 1.0],
        weigh_colour_by_alpha: false,
    };

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut rgba_block: [[u8; 4]; 16] = [[0; 4]; 16];

            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px).min(width - 1);
                    let y = (by * 4 + py).min(height - 1);
                    let pixel = img.get_pixel(x as u32, y as u32);

                    let idx = py * 4 + px;
                    rgba_block[idx][0] = pixel[0];
                    rgba_block[idx][1] = pixel[1];
                    rgba_block[idx][2] = pixel[2];
                    rgba_block[idx][3] = pixel[3];
                }
            }

            let block_index = (by * blocks_x + bx) * 8;
            format.compress_block_masked(
                rgba_block,
                0xFFFF,
                params,
                &mut output[block_index..block_index + 8]
            );
        }
    }

    Ok(output)
}

fn compress_dxt5(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<Vec<u8>, std::io::Error> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let blocks_x = (width + 3) / 4;
    let blocks_y = (height + 3) / 4;

    let mut output = vec![0u8; blocks_x * blocks_y * 16];

    let format = Format::Bc3;
    let params = Params {
        algorithm: Algorithm::IterativeClusterFit,
        weights: [1.0, 1.0, 1.0],
        weigh_colour_by_alpha: false,
    };

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let mut rgba_block: [[u8; 4]; 16] = [[0; 4]; 16];

            for py in 0..4 {
                for px in 0..4 {
                    let x = (bx * 4 + px).min(width - 1);
                    let y = (by * 4 + py).min(height - 1);
                    let pixel = img.get_pixel(x as u32, y as u32);

                    let idx = py * 4 + px;
                    rgba_block[idx][0] = pixel[0];
                    rgba_block[idx][1] = pixel[1];
                    rgba_block[idx][2] = pixel[2];
                    rgba_block[idx][3] = pixel[3];
                }
            }

            let block_index = (by * blocks_x + bx) * 16;
            format.compress_block_masked(
                rgba_block,
                0xFFFF,
                params,
                &mut output[block_index..block_index + 16]
            );
        }
    }

    Ok(output)
}

fn calculate_average_color(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> [u8; 4] {
    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut a_sum = 0u64;
    let pixel_count = (img.width() * img.height()) as u64;

    for pixel in img.pixels() {
        r_sum += pixel[0] as u64;
        g_sum += pixel[1] as u64;
        b_sum += pixel[2] as u64;
        a_sum += pixel[3] as u64;
    }

    [
        (r_sum / pixel_count) as u8,
        (g_sum / pixel_count) as u8,
        (b_sum / pixel_count) as u8,
        (a_sum / pixel_count) as u8,
    ]
}

fn calculate_maximum_color(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> [u8; 4] {
    let mut max_r = 0u8;
    let mut max_g = 0u8;
    let mut max_b = 0u8;
    let mut max_a = 0u8;

    for pixel in img.pixels() {
        max_r = max_r.max(pixel[0]);
        max_g = max_g.max(pixel[1]);
        max_b = max_b.max(pixel[2]);
        max_a = max_a.max(pixel[3]);
    }

    [max_r, max_g, max_b, max_a]
}

pub fn cmd_paa2img(source: &Path, target: &Path, force: bool) -> Result<(), std::io::Error> {
    // Check if target exists and force flag is not set
    if target.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Output file '{}' already exists. Use -f/--force to overwrite.", target.display())
        ));
    }
    let paa = Paa::from_file(source)?;
    let img = paa.to_image()?;
    img.save(target).map_err(|e| error!("Failed to save image: {}", e))?;
    Ok(())
}

pub fn cmd_img2paa(source: &Path, target: &Path, paa_type: PaaType, compress: bool, force: bool) -> Result<(), std::io::Error> {
    // Check if target exists and force flag is not set
    if target.exists() && !force {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Output file '{}' already exists. Use -f/--force to overwrite.", target.display())
        ));
    }
    let img = image::open(source).map_err(|e| error!("Failed to open image: {}", e))?;
    let paa = Paa::from_image(&img, paa_type, compress)?;
    paa.write_to_file(target)?;
    Ok(())
}