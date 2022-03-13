#![no_std]
#![deny(clippy::all)]
#![feature(never_type)]

use playdate::{CStr, CString, LCDBitmapFlip, LCDColor, LCDPattern, LCDSolidColor, PDStringEncoding};

#[playdate::main]
async fn main(api: playdate::Api) -> ! {
  let system = &api.system;
  let graphics = &api.graphics;

  let grey50: LCDPattern = [
    // Bitmap
    0b10101010, 0b01010101, 0b10101010, 0b01010101, 0b10101010, 0b01010101, 0b10101010, 0b01010101,
    // Mask
    0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111, 0b11111111,
  ];
  graphics.clear(LCDColor::Pattern(&grey50));

  let bmp = graphics.new_bitmap(100, 40, LCDColor::Solid(LCDSolidColor::kColorWhite));
  graphics.draw_bitmap(&bmp, 5, 9, LCDBitmapFlip::kBitmapUnflipped);
  drop(bmp);

  // TODO: this crashes???
  // let text = CString::new("Bloop");

  let text = CStr::from_bytes_with_nul(b"Bloop\0").unwrap();
  graphics.draw_text(text, PDStringEncoding::kASCIIEncoding, 30, 20);

  let copy = graphics.copy_frame_buffer_bitmap();

  let data = copy.data();
  unsafe {
    for i in 0..15 {
      *data.data.offset(i) = 0;
    }
  }
  graphics.draw_bitmap(&copy, 0, 30, LCDBitmapFlip::kBitmapUnflipped);

  loop {
    let fw = system.frame_watcher();
    //system.log(CString::from_vec("cstring").unwrap());
    system.log("before");
    fw.next().await;
    system.log("after");
  }
}
