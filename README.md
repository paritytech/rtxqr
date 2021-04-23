# rtxqr
Rust implementation of animated QRs for to transfer arbitrarily sized data.

# Usage

	$cargo run inputfile [settingsfile]

# Settings file

	raptorq:
	const CHUNK_SIZE: u16 = 1072;
	
	apng:
	const MAIN_COLOR: u8 = 0x00;
	const BACK_COLOR: u8 = 0xFF;
	const SCALING: i32 = 4;
	const FPS_NOM: u16 = 1;
	const FPS_DEN: u16 = 4;
	const BORDER: i32 = 4;


