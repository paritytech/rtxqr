// Expected command line syntax 'cargo run source_file_name (optional)_setup_file_name'

use std::env;
use std::fs;
use std::error::Error;
use hex;
use std::string::String;
use raptorq;
use regex::Regex;
use qrcodegen::{QrCode, QrCodeEcc};
use apng_encoder;

/// struct to store the names of source file and setur file
/// taken from env::args input

pub struct Entry {
    pub filename: String,
    pub setupname: Option<String>,
}

/// function make Entry from env::args input,
/// can add later more env::args parameters

impl Entry {
    pub fn new(mut args: env::Args) -> Result<Entry, &'static str> {
        args.next();

        let filename = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a file name"),
        };
        
        let setupname = args.next();
        
        Ok(Entry {
            filename,
            setupname,
        })
    }
}

/// function to read hex line from the file into Vec<u8> encodeable into qr

pub fn prepare_hex (entry: &Entry) -> Result<Vec<u8>, Box<dyn Error>> {
    let contents = fs::read_to_string(&entry.filename)?;
    let data = hex::decode(contents.trim())?;
    Ok(data)
}

/// function to transform random text from the file into Vec<u8> encodeable into qr

pub fn prepare_text (entry: &Entry) -> Result<Vec<u8>, Box<dyn Error>> {
    let contents = fs::read_to_string(&entry.filename)?;
    let data = contents.to_owned().into_bytes();
    Ok(data)
}

/// struct to store the constants needed for qr code making

pub struct Constants {
    pub chunk_size: u16,
    pub main_color: u8,
    pub back_color: u8,
    pub scaling: i32,
    pub fps_nom: u16,
    pub fps_den: u16,
    pub border: i32,
}

/// function to set up the constants from the external file

pub fn set_constants (entry: &Entry) -> Result<Constants, String> {
    let setup_file_name = match &entry.setupname {
        Some(a) => a,
        None => "default_constants",
    };
    let contents = match fs::read_to_string(&setup_file_name) {
        Ok(a) => a,
        Err(e) => return Err(e.to_string()),
    };
    // chunk_size
    let re = Regex::new(r#"(?i)CHUNK_SIZE.*= (?P<chunk_size>[0-9]+);"#).unwrap();
    let chunk_size: u16 = match re.captures(&contents) {
        Some(a) => (&a["chunk_size"]).parse().unwrap(),
        None => return Err(format!("No chunk_size value found in {}", &setup_file_name)),
    };
    // main_color (black by default)
    let re = Regex::new(r#"(?i)MAIN_COLOR.*= (0x)?(?P<main_color>[0-9a-f]{2});"#).unwrap();
    let main_color: u8 = match re.captures(&contents) {
        Some(a) => (hex::decode(&a["main_color"]).unwrap())[0],
        None => return Err(format!("No main_color value found in {}", &setup_file_name)),
    };
    // back_color (white by default)
    let re = Regex::new(r#"(?i)BACK_COLOR.*= (0x)?(?P<back_color>[0-9a-f]{2});"#).unwrap();
    let back_color: u8 = match re.captures(&contents) {
        Some(a) => (hex::decode(&a["back_color"]).unwrap())[0],
        None => return Err(format!("No back_color value found in {}", &setup_file_name)),
    };
    // scaling
    let re = Regex::new(r#"(?i)SCALING.*= (?P<scaling>[0-9]*);"#).unwrap();
    let scaling: i32 = match re.captures(&contents) {
        Some(a) => (&a["scaling"]).parse().unwrap(),
        None => return Err(format!("No scaling value found in {}", &setup_file_name)),
    };
    // fps_nom
    let re = Regex::new(r#"(?i)FPS_NOM.*= (?P<fps_nom>[0-9]+);"#).unwrap();
    let fps_nom: u16 = match re.captures(&contents) {
        Some(a) => (&a["fps_nom"]).parse().unwrap(),
        None => return Err(format!("No fps_nom value found in {}", &setup_file_name)),
    };
    // fps_den
    let re = Regex::new(r#"(?i)FPS_DEN.*= (?P<fps_den>[0-9]+);"#).unwrap();
    let fps_den: u16 = match re.captures(&contents) {
        Some(a) => (&a["fps_den"]).parse().unwrap(),
        None => return Err(format!("No fps_den value found in {}", &setup_file_name)),
    };
    // border
    let re = Regex::new(r#"(?i)BORDER.*= (?P<border>[0-9]+);"#).unwrap();
    let border: i32 = match re.captures(&contents) {
        Some(a) => (&a["border"]).parse().unwrap(),
        None => return Err(format!("No border value found in {}", &setup_file_name)),
    };
    if main_color == back_color {
        return Err(format!("Main and back color are identical: {}, qr code generaion not possible.", main_color));
    }
    let out = Constants {
        chunk_size,
        main_color,
        back_color,
        scaling,
        fps_nom,
        fps_den,
        border,
    };
    Ok(out)
}

/// function to take data as Vec<u8>, apply raptorq to get Vec<EncodingPacket>
/// and serialize it to get Vec<u8> output

pub fn make_data_packs (input: Vec<u8>, constants: &Constants) -> Result<Vec<Vec<u8>>, &'static str> {

// checking that data is not too long, set limit for now at 2^31 bit
    if input.len() >= 0x80000000 { 
        return Err("Input data is too long, processing not possible");
    }
// added at the beginning to each vector before transforming into qr code: contains input length info, also has first bit always 1 indicating it is new fountain qr - possibly need to change this later
    let data_size_info = (input.len() as u32 + 0x80000000).to_be_bytes();

// number of additional packets; currently roughly equal to number of core packets
    let repair_packets_per_block: u32 = {
        if input.len() as u32 <= constants.chunk_size as u32 {0}
        else {input.len() as u32/constants.chunk_size as u32}
    };
// making raptorq Encoder, with defaults
    let raptor_encoder = raptorq::Encoder::with_defaults(&input, constants.chunk_size);
// making EncodingPacket and deserializing each into Vec<u8>
    let out: Vec<Vec<u8>> = raptor_encoder.get_encoded_packets(repair_packets_per_block)
        .iter()
        .map(|x| [data_size_info.to_vec(), x.serialize()].concat())
        .collect();
    let len_check = out[0].len();
    for x in out.iter() {
        if x.len() != len_check {
            return Err("Encoded chunks have different length");
        }
    }
    if len_check > 2953 {
            return Err("Encoded chunks too large to be turned into QR codes");
    }
    Ok(out)
}

/// function to take data as Vec<Vec<u8>> with all stuff added and make Vec<QrCode>

pub fn make_qr_codes (data: Vec<Vec<u8>>) -> Vec<QrCode> {
// safe to unwrap, length checked while making data
    let out: Vec<QrCode> = data
        .iter()
        .map(|x| QrCode::encode_binary(&x, QrCodeEcc::Low).unwrap())
        .collect();
    out
}

pub fn make_apng (data: Vec<QrCode>, constants: &Constants, output_name: &str) -> Result<(), Box<dyn Error>> {
    let mut output_file = fs::File::create(output_name)?;
    let frames_count: u32 = data.len() as u32;
    let border_size = constants.border*constants.scaling;
    let size: u32 = (data[0].size() as u32) * (constants.scaling as u32) + 2*border_size as u32; // size is always positive and small
    let apng_meta = apng_encoder::Meta {
        width: size,
        height: size,
        color: apng_encoder::Color::Grayscale(8), 
        frames: frames_count,
        plays: None,
    };
    let apng_frame = apng_encoder::Frame {
        delay: Some(apng_encoder::Delay::new(constants.fps_nom, constants.fps_den)),
        ..Default::default()
    };
    let mut apng_encoder = apng_encoder::Encoder::create(&mut output_file, apng_meta).unwrap();

// making actual apng
// qr.get_module(x,y) = false corresponds to back color (white by default)
// qr.get_module(x,y) = true corresponds to main color (black by default)

    for qr in data.iter() {
        let mut buffer: Vec<u8> = Vec::new();
        for x in 0..size {
            for y in 0..size {
                if qr.get_module(x as i32/constants.scaling - constants.border, y as i32/constants.scaling - constants.border) {
                    buffer.push(constants.main_color);
                }
                else {
                    buffer.push(constants.back_color);
                }
            }
        }
// deal with errors, ApngError enum
        apng_encoder.write_frame(&buffer, Some(&apng_frame), None, None).unwrap()
    }
    apng_encoder.finish().unwrap();
    Ok(())
}

// function to encode hex string from file into qr apng

pub fn run_hex (entry: &Entry, output_name: &str) -> Result<(), Box<dyn Error>>{
    let data = prepare_hex(entry)?;
    let constants = set_constants(entry)?;
    let data_packs = make_data_packs(data, &constants)?;
    make_apng(make_qr_codes(data_packs), &constants, output_name)?;
    Ok(())
}

// function to encode text from file into qr apng

pub fn run_text (entry: &Entry, output_name: &str) -> Result<(), Box<dyn Error>>{
    let data = prepare_text(entry)?;
    let constants = set_constants(entry)?;
    let data_packs = make_data_packs(data, &constants)?;
    make_apng(make_qr_codes(data_packs), &constants, output_name)?;
    Ok(())
}
